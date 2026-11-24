//! Opening, reading back and saving a whole world.

mod raids;
mod wandering_trader;

use super::dimensions::{
    DimensionSectionGeometry, Java26DimensionGeometries, Java26DimensionGeometry,
};
use super::entity_publication_fixtures::{
    area_effect_cloud_entity_entries, loaded_entity_chunk, nbt_entry, test_created_save,
    test_flat_column,
};
use super::saved_data::{
    java_26_1_chunk_tickets_document, java_26_1_game_rules_document, java_26_1_map_document,
    java_26_1_map_index_document, java_26_1_world_border_document, load_java_26_1_chunk_tickets,
    load_java_26_1_game_rules, load_java_26_1_world_border, Java26MapSavedData,
    Java26SavedChunkTicket, Java26WorldBorderSettings,
};
use super::{Java26EntityChunkWrite, Java26SaveRequest, Java26WorldStorage, SaveHolderResolver};
use crate::level_file::encode_gzip_named_compound;
use crate::player_advancements::{
    AdvancementProgress, CriterionProgress, Java26PlayerAdvancements,
};
use crate::weather_schema::Java26WeatherData;
use crate::Java26ScoreboardPacked;
use crate::{
    Java26CreatedChunk, Java26CreatedPlayer, Java26LevelSaveState, Java26PlayerSavePatch,
    Java26RootVehicle, LevelDatStore, RegionCompression, RegionFileStore,
};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::{GameType, VANILLA_GAME_TYPE_CREATIVE_ID};
use bbb_protocol::packets::PlayerAbilities;
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::dimension_type::builtin_dimension_type_shape_facts;
use bbb_server_types::random_sequences_saved::RandomSequencesSaved;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};
use bbb_server_types::save::{
    NamedCompound, SaveError, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
    JAVA_26_1_STORAGE_VERSION,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use uuid::Uuid;

mod custom_level;
mod poi;
mod world_border;
mod world_clocks;

/// One column read back the way the running world reads it.
fn stored_column(
    candidate: &super::Java26WorldCandidate,
    dimension: &Java26LevelKey,
    (x, z): (i32, i32),
) -> super::Java26RuntimeColumn {
    let reader = candidate
        .column_reader
        .as_ref()
        .expect("an opened save reads its chunks");
    let mut column = reader.read_column(dimension, x, z).unwrap();
    column.entities = reader.read_entity_chunk(dimension, x, z).unwrap();
    column
}

fn stored_chunk(
    candidate: &super::Java26WorldCandidate,
    dimension: &Java26LevelKey,
    chunk: (i32, i32),
) -> super::Java26LoadedChunk {
    stored_column(candidate, dimension, chunk)
        .chunk
        .expect("the chunk is stored")
}

fn root_vehicle_entity_entries(uuid: Uuid) -> Vec<bbb_protocol::NbtSummaryEntry> {
    vec![
        nbt_entry("id", NbtSummaryValue::String("minecraft:oak_boat".into())),
        nbt_entry("UUID", uuid_int_array(uuid)),
        nbt_entry(
            "Pos",
            NbtSummaryValue::List(
                [1.0_f64, 64.0, 2.0]
                    .into_iter()
                    .map(|value| NbtSummaryValue::Double(value.to_bits()))
                    .collect(),
            ),
        ),
        nbt_entry(
            "Motion",
            NbtSummaryValue::List(
                [0.0_f64; 3]
                    .into_iter()
                    .map(|value| NbtSummaryValue::Double(value.to_bits()))
                    .collect(),
            ),
        ),
        nbt_entry(
            "Rotation",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Float(0.0_f32.to_bits()),
                NbtSummaryValue::Float(0.0_f32.to_bits()),
            ]),
        ),
        nbt_entry("fall_distance", NbtSummaryValue::Double(0.0_f64.to_bits())),
        nbt_entry("Fire", NbtSummaryValue::Short(-20)),
        nbt_entry("Air", NbtSummaryValue::Short(300)),
        nbt_entry("OnGround", NbtSummaryValue::Byte(1)),
        nbt_entry("Invulnerable", NbtSummaryValue::Byte(0)),
        nbt_entry("PortalCooldown", NbtSummaryValue::Int(0)),
    ]
}

fn uuid_int_array(uuid: Uuid) -> NbtSummaryValue {
    let value = uuid.as_u128();
    NbtSummaryValue::IntArray(vec![
        (value >> 96) as i32,
        (value >> 64) as i32,
        (value >> 32) as i32,
        value as i32,
    ])
}

fn level_key(dimension: Java26BuiltinDimension) -> Java26LevelKey {
    dimension.into()
}

fn default_world_borders() -> BTreeMap<Java26LevelKey, Java26WorldBorderSettings> {
    Java26BuiltinDimension::ALL
        .into_iter()
        .map(|dimension| (level_key(dimension), Java26WorldBorderSettings::default()))
        .collect()
}

/// The section geometry a dimension directory is read with is the
/// `dimension_type` its own stem names, so a save filing
/// `minecraft:overworld_caves` under the overworld is read as that type's
/// 24 sections and one filing the nether's type gets 16.
#[test]
fn section_geometry_follows_the_stem_type_rather_than_the_dimension_name() {
    let geometry = |element_id: &str| {
        DimensionSectionGeometry::of(&builtin_dimension_type_shape_facts(element_id).unwrap())
            .unwrap()
    };
    assert_eq!(
        geometry("minecraft:overworld"),
        DimensionSectionGeometry {
            min_section_y: -4,
            section_count: 24,
        }
    );
    assert_eq!(
        geometry("minecraft:overworld_caves"),
        geometry("minecraft:overworld")
    );
    let nether = DimensionSectionGeometry {
        min_section_y: 0,
        section_count: 16,
    };
    assert_eq!(geometry("minecraft:the_nether"), nether);
    assert_eq!(geometry("minecraft:the_end"), nether);
}

/// Every dimension directory is read back with the geometry its own stem
/// declares. The nether column is 16 sections starting at section 0 where
/// the overworld's is 24 starting at -4, and both are read out of the same
/// save by the same reader.
#[test]
fn a_save_reloads_each_dimension_directory_with_its_own_stem_geometry() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-per-dimension-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();

    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let overworld_column = test_flat_column(false, &fixture);
    let mut save = test_created_save(
        &crate::Java26WorldCreation {
            level_name: "bbb-created".to_string(),
            game_type: VANILLA_GAME_TYPE_CREATIVE_ID,
            difficulty: "easy".to_string(),
            spawn_position: [0, 65, 0],
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &overworld_column)],
    )
    .unwrap();

    // The production Nether stem is noise-backed; storage still reads a saved
    // column against that stem's own section geometry.
    let settings = crate::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
        save.world_gen_settings.clone(),
        0,
    )
    .unwrap();
    let nether_stem = &settings.dimensions[&level_key(Java26BuiltinDimension::Nether)];
    assert_eq!(nether_stem.generator, "minecraft:noise");
    let nether_shape = builtin_dimension_type_shape_facts(&nether_stem.dimension_type).unwrap();
    let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap();
    let nether_column = ChunkColumn::new(
        nether_shape.min_y,
        usize::try_from(nether_shape.height / 16).unwrap(),
        air,
        fixture.biome_id(),
    );
    save.chunks.push(Java26CreatedChunk {
        dimension: Java26BuiltinDimension::Nether.into(),
        chunk_x: 0,
        chunk_z: 0,
        document: crate::chunk_writer::full_chunk_document(
            &nether_column,
            0,
            0,
            &|_| Some("minecraft:nether_wastes".to_string()),
            nether_shape.min_y,
        )
        .unwrap(),
    });

    storage
        .create_world(&save, RegionCompression::Zlib, 1)
        .unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert_eq!(
        stored_chunk(
            &candidate,
            &level_key(Java26BuiltinDimension::Overworld),
            (0, 0)
        )
        .data
        .sections()
        .len(),
        24
    );
    let nether = &stored_chunk(
        &candidate,
        &level_key(Java26BuiltinDimension::Nether),
        (0, 0),
    );
    assert_eq!(nether.data.sections().len(), 16);
    assert_eq!(nether.data.sections()[0].y, 0);
    fs::remove_dir_all(root).unwrap();
}

/// The point of creation: a directory with nothing in it becomes a save
/// this server can open again, through the same reader an official save
/// goes through. Anything less would be a save only its writer believes in.
#[test]
fn a_created_world_loads_back_through_the_official_save_reader() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-creation-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    assert!(!storage.holds_level_data(), "an empty directory holds none");

    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let column = test_flat_column(false, &fixture);
    let save = test_created_save(
        &crate::Java26WorldCreation {
            level_name: "bbb-created".to_string(),
            game_type: VANILLA_GAME_TYPE_CREATIVE_ID,
            difficulty: "easy".to_string(),
            spawn_position: [0, 65, 0],
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[(0, 0, &column)],
    )
    .unwrap();
    storage
        .create_world(&save, RegionCompression::Zlib, 1)
        .unwrap();
    assert!(storage.holds_level_data(), "the created save has a level");

    let candidate = storage.load_candidate().unwrap();
    assert_eq!(candidate.level.level_name(), "bbb-created");
    assert_eq!(candidate.world_gen_settings.seed, 0);
    let chunk = stored_chunk(
        &candidate,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    );
    assert_eq!(chunk.data.status().as_str(), "minecraft:full");
    // Every section the column has, written the way a save stores them.
    assert_eq!(chunk.data.sections().len(), column.section_count());

    // Creation is for a directory with no level, not for replacing one.
    assert!(matches!(
        storage.create_world(&save, RegionCompression::Zlib, 1),
        Err(SaveError::InvalidPath { .. })
    ));
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

fn save_fixture_level() -> NamedCompound {
    NamedCompound::new(
        "",
        vec![nbt_entry(
            "Data",
            NbtSummaryValue::Compound(vec![
                nbt_entry(
                    "ServerBrands",
                    NbtSummaryValue::List(vec![NbtSummaryValue::string("vanilla")]),
                ),
                nbt_entry("WasModded", NbtSummaryValue::Byte(0)),
                nbt_entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                nbt_entry("GameType", NbtSummaryValue::Int(1)),
                nbt_entry(
                    "spawn",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
                        nbt_entry("pos", NbtSummaryValue::IntArray(vec![0, 64, 0])),
                        nbt_entry("yaw", NbtSummaryValue::Float(0.0_f32.to_bits())),
                        nbt_entry("pitch", NbtSummaryValue::Float(0.0_f32.to_bits())),
                    ]),
                ),
                nbt_entry("Time", NbtSummaryValue::Long(42)),
                nbt_entry("LastPlayed", NbtSummaryValue::Long(1_786_000_000_000)),
                nbt_entry("LevelName", NbtSummaryValue::string("BBB save fixture")),
                nbt_entry("version", NbtSummaryValue::Int(JAVA_26_1_STORAGE_VERSION)),
                nbt_entry("allowCommands", NbtSummaryValue::Byte(1)),
                nbt_entry("initialized", NbtSummaryValue::Byte(1)),
                nbt_entry(
                    "difficulty_settings",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("difficulty", NbtSummaryValue::string("normal")),
                        nbt_entry("hardcore", NbtSummaryValue::Byte(0)),
                        nbt_entry("locked", NbtSummaryValue::Byte(0)),
                    ]),
                ),
                nbt_entry(
                    "DataPacks",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry(
                            "Enabled",
                            NbtSummaryValue::List(vec![NbtSummaryValue::string(
                                "vanilla".to_string(),
                            )]),
                        ),
                        nbt_entry(
                            "Disabled",
                            NbtSummaryValue::List(
                                [
                                    "trade_rebalance",
                                    "redstone_experiments",
                                    "minecart_improvements",
                                ]
                                .into_iter()
                                .map(NbtSummaryValue::string)
                                .collect(),
                            ),
                        ),
                    ]),
                ),
                nbt_entry(
                    "enabled_features",
                    NbtSummaryValue::List(vec![NbtSummaryValue::string(
                        "minecraft:vanilla".to_string(),
                    )]),
                ),
                nbt_entry(
                    "Version",
                    NbtSummaryValue::Compound(vec![
                        nbt_entry("Name", NbtSummaryValue::string("26.1")),
                        nbt_entry("Id", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                        nbt_entry("Snapshot", NbtSummaryValue::Byte(0)),
                        nbt_entry("Series", NbtSummaryValue::string("main")),
                    ]),
                ),
            ]),
        )],
    )
}

fn save_fixture_world_gen_settings() -> NamedCompound {
    let stem = |dimension_type: &str| {
        NbtSummaryValue::Compound(vec![
            nbt_entry("type", NbtSummaryValue::string(dimension_type)),
            nbt_entry(
                "generator",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "type",
                    NbtSummaryValue::string("minecraft:noise"),
                )]),
            ),
        ])
    };
    NamedCompound::new(
        "",
        vec![
            nbt_entry(
                "data",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("seed", NbtSummaryValue::Long(42)),
                    nbt_entry("generate_structures", NbtSummaryValue::Byte(1)),
                    nbt_entry("bonus_chest", NbtSummaryValue::Byte(0)),
                    nbt_entry(
                        "dimensions",
                        NbtSummaryValue::Compound(vec![
                            nbt_entry("minecraft:overworld", stem("minecraft:overworld")),
                            nbt_entry("minecraft:the_nether", stem("minecraft:the_nether")),
                            nbt_entry("minecraft:the_end", stem("minecraft:the_end")),
                        ]),
                    ),
                ]),
            ),
            nbt_entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        ],
    )
}

fn save_fixture_chunk() -> NamedCompound {
    let post_processing = (0..24).map(|_| NbtSummaryValue::List(Vec::new())).collect();
    NamedCompound::new(
        "",
        vec![
            nbt_entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            nbt_entry("xPos", NbtSummaryValue::Int(0)),
            nbt_entry("yPos", NbtSummaryValue::Int(-4)),
            nbt_entry("zPos", NbtSummaryValue::Int(0)),
            nbt_entry("LastUpdate", NbtSummaryValue::Long(99)),
            nbt_entry("InhabitedTime", NbtSummaryValue::Long(7)),
            nbt_entry("Status", NbtSummaryValue::string("minecraft:full")),
            nbt_entry(
                "sections",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    nbt_entry("Y", NbtSummaryValue::Byte(-4)),
                    nbt_entry(
                        "block_states",
                        NbtSummaryValue::Compound(vec![nbt_entry(
                            "palette",
                            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                                nbt_entry("Name", NbtSummaryValue::string("minecraft:stone")),
                            ])]),
                        )]),
                    ),
                    nbt_entry(
                        "biomes",
                        NbtSummaryValue::Compound(vec![nbt_entry(
                            "palette",
                            NbtSummaryValue::List(vec![NbtSummaryValue::string(
                                "minecraft:plains".to_string(),
                            )]),
                        )]),
                    ),
                ])]),
            ),
            nbt_entry("block_entities", NbtSummaryValue::List(Vec::new())),
            nbt_entry("block_ticks", NbtSummaryValue::List(Vec::new())),
            nbt_entry("fluid_ticks", NbtSummaryValue::List(Vec::new())),
            nbt_entry("PostProcessing", NbtSummaryValue::List(post_processing)),
            nbt_entry(
                "Heightmaps",
                NbtSummaryValue::Compound(
                    [
                        "OCEAN_FLOOR",
                        "WORLD_SURFACE",
                        "MOTION_BLOCKING",
                        "MOTION_BLOCKING_NO_LEAVES",
                    ]
                    .into_iter()
                    .map(|name| nbt_entry(name, NbtSummaryValue::LongArray(vec![0; 37])))
                    .collect(),
                ),
            ),
            nbt_entry(
                "structures",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("starts", NbtSummaryValue::Compound(Vec::new())),
                    nbt_entry("References", NbtSummaryValue::Compound(Vec::new())),
                ]),
            ),
        ],
    )
}

pub(super) fn save_fixture_world(root: &Path) {
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root).unwrap();
    let paths = WorldSavePaths::new(root).unwrap();
    LevelDatStore::new(paths.clone(), WorldSaveLimits::default())
        .unwrap()
        .save(&save_fixture_level())
        .unwrap();
    fs::create_dir_all(paths.saved_data_dir().join("minecraft")).unwrap();
    let world_gen_settings = encode_gzip_named_compound(
        &paths.world_gen_settings_dat(),
        &save_fixture_world_gen_settings(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    fs::write(paths.world_gen_settings_dat(), world_gen_settings).unwrap();
    let region_dir = paths
        .builtin_dimension_dir(Java26BuiltinDimension::Overworld)
        .join("region");
    fs::create_dir_all(&region_dir).unwrap();
    let store = RegionFileStore::new(
        region_dir.join("r.0.0.mca"),
        &region_dir,
        WorldSaveLimits::default(),
    )
    .unwrap();
    store
        .write_chunk(0, 0, &save_fixture_chunk(), RegionCompression::Zlib, 100)
        .unwrap();
}

pub(super) fn test_world_clock_ids() -> Vec<String> {
    vec![
        "minecraft:overworld".to_string(),
        "minecraft:the_end".to_string(),
    ]
}

fn custom_short_geometries(settings: &crate::Java26WorldGenSettings) -> Java26DimensionGeometries {
    let mut entries = BTreeMap::new();
    for (name, stem) in &settings.dimensions {
        if *name == level_key(Java26BuiltinDimension::Overworld) {
            entries.insert(
                name.clone(),
                Java26DimensionGeometry::new(
                    "example:short",
                    DimensionSectionGeometry {
                        min_section_y: 0,
                        section_count: 16,
                    },
                ),
            );
            continue;
        }
        let Some(shape) = builtin_dimension_type_shape_facts(&stem.dimension_type) else {
            continue;
        };
        let Some(sections) = DimensionSectionGeometry::of(&shape).ok() else {
            continue;
        };
        entries.insert(
            name.clone(),
            Java26DimensionGeometry::new(stem.dimension_type.clone(), sections),
        );
    }
    Java26DimensionGeometries::from_entries(entries)
}

fn overwrite_fixture_chunk(root: &Path, document: &NamedCompound) {
    let paths = WorldSavePaths::new(root).unwrap();
    let region_dir = paths
        .builtin_dimension_dir(Java26BuiltinDimension::Overworld)
        .join("region");
    RegionFileStore::new(
        region_dir.join("r.0.0.mca"),
        &region_dir,
        WorldSaveLimits::default(),
    )
    .unwrap()
    .write_chunk(0, 0, document, RegionCompression::Zlib, 100)
    .unwrap();
}

fn short_fixture_chunk() -> NamedCompound {
    let mut document = save_fixture_chunk();
    for entry in document.entries_mut() {
        match entry.name.to_jdk_string().as_str() {
            "yPos" => entry.value = NbtSummaryValue::Int(0),
            "sections" => {
                let NbtSummaryValue::List(sections) = &mut entry.value else {
                    unreachable!()
                };
                let NbtSummaryValue::Compound(section) = &mut sections[0] else {
                    unreachable!()
                };
                section
                    .iter_mut()
                    .find(|entry| entry.name == "Y")
                    .unwrap()
                    .value = NbtSummaryValue::Byte(0);
            }
            "PostProcessing" => {
                entry.value = NbtSummaryValue::List(
                    (0..16).map(|_| NbtSummaryValue::List(Vec::new())).collect(),
                );
            }
            _ => {}
        }
    }
    document
}

fn level_data_mut(document: &mut NamedCompound) -> &mut Vec<bbb_protocol::NbtSummaryEntry> {
    let data = document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Data")
        .unwrap();
    let NbtSummaryValue::Compound(data) = &mut data.value else {
        unreachable!()
    };
    data
}

fn set_pack_list(document: &mut NamedCompound, name: &str, values: &[&str]) {
    let packs = level_data_mut(document)
        .iter_mut()
        .find(|entry| entry.name == "DataPacks")
        .unwrap();
    let NbtSummaryValue::Compound(packs) = &mut packs.value else {
        unreachable!()
    };
    packs
        .iter_mut()
        .find(|entry| entry.name == name)
        .unwrap()
        .value = NbtSummaryValue::List(
        values
            .iter()
            .map(|id| NbtSummaryValue::string(*id))
            .collect(),
    );
}

#[test]
fn dimension_geometries_follow_the_baked_live_holder_before_region_read() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-dimension-geometries-custom-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    overwrite_fixture_chunk(&root, &short_fixture_chunk());
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let mut document = storage
        .load_bootstrap()
        .unwrap()
        .world_gen_settings()
        .document
        .clone();
    let data = document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "data")
        .unwrap();
    let NbtSummaryValue::Compound(fields) = &mut data.value else {
        panic!();
    };
    let dimensions = fields
        .iter_mut()
        .find(|entry| entry.name == "dimensions")
        .unwrap();
    let NbtSummaryValue::Compound(stems) = &mut dimensions.value else {
        panic!();
    };
    stems.push(nbt_entry(
        "example:missing_holder",
        bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
            "type": "example:missing_type",
            "generator": {"type": "minecraft:debug", "keep": "raw generator payload"}
        }))
        .unwrap(),
    ));
    let settings_bytes = encode_gzip_named_compound(
        &storage.paths().world_gen_settings_dat(),
        &document,
        WorldSaveLimits::default(),
    )
    .unwrap();
    fs::write(storage.paths().world_gen_settings_dat(), &settings_bytes).unwrap();
    let unknown_region = storage
        .paths()
        .dimension_dir(&Java26LevelKey::parse("example:missing_holder").unwrap())
        .join("region/r.0.0.mca");
    fs::create_dir_all(unknown_region.parent().unwrap()).unwrap();
    fs::copy(
        storage
            .paths()
            .builtin_dimension_dir(Java26BuiltinDimension::Overworld)
            .join("region/r.0.0.mca"),
        &unknown_region,
    )
    .unwrap();
    let unknown_region_bytes = fs::read(&unknown_region).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let geometries = custom_short_geometries(bootstrap.world_gen_settings());
    let candidate = storage
        .load_candidate_with_geometries(bootstrap, geometries, &test_world_clock_ids())
        .unwrap();
    let geometries = &candidate.dimension_geometries;
    let overworld = geometries
        .get(&level_key(Java26BuiltinDimension::Overworld))
        .unwrap();
    assert_eq!(overworld.dimension_type_id(), "example:short");
    assert_eq!(
        overworld.sections(),
        DimensionSectionGeometry {
            min_section_y: 0,
            section_count: 16,
        }
    );
    let chunk = &stored_chunk(
        &candidate,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    )
    .data;
    assert_eq!(chunk.min_section_y(), 0);
    assert_eq!(chunk.post_processing().len(), 16);
    assert!(geometries
        .get(&Java26LevelKey::parse("example:missing_holder").unwrap())
        .is_none());
    assert_eq!(candidate.world_gen_settings.document, document);
    assert_eq!(fs::read(&unknown_region).unwrap(), unknown_region_bytes);
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn compatibility_load_accepts_missing_pack_selection() {
    let missing_root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-compat-missing-packs-{}",
        std::process::id()
    ));
    save_fixture_world(&missing_root);
    let paths = WorldSavePaths::new(&missing_root).unwrap();
    let mut level = save_fixture_level();
    set_pack_list(
        &mut level,
        "Disabled",
        &["file/not-enabled", "example:missing"],
    );
    LevelDatStore::new(paths.clone(), WorldSaveLimits::default())
        .unwrap()
        .save(&level)
        .unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    storage.load_candidate().unwrap();

    level_data_mut(&mut level).retain(|entry| entry.name != "DataPacks");
    LevelDatStore::new(paths, WorldSaveLimits::default())
        .unwrap()
        .save(&level)
        .unwrap();
    storage.load_candidate().unwrap();
    storage.close().unwrap();
    fs::remove_dir_all(missing_root).unwrap();
}

#[test]
fn an_experiment_world_loads_and_keeps_its_feature_flags_on_save() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-experiments-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let features = ["minecraft:trade_rebalance", "minecraft:vanilla"];
    let mut level = save_fixture_level();
    set_pack_list(&mut level, "Enabled", &["vanilla", "trade_rebalance"]);
    set_pack_list(
        &mut level,
        "Disabled",
        &["redstone_experiments", "minecart_improvements"],
    );
    level_data_mut(&mut level)
        .iter_mut()
        .find(|entry| entry.name == "enabled_features")
        .unwrap()
        .value = NbtSummaryValue::List(
        features
            .iter()
            .map(|id| NbtSummaryValue::string(*id))
            .collect(),
    );
    LevelDatStore::new(paths.clone(), WorldSaveLimits::default())
        .unwrap()
        .save(&level)
        .unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let geometries =
        Java26DimensionGeometries::from_builtin_settings(bootstrap.world_gen_settings());
    let candidate = storage
        .load_candidate_with_geometries(bootstrap, geometries, &test_world_clock_ids())
        .unwrap();
    storage
        .save_snapshot(&candidate, &save_patch_request(Vec::new()))
        .unwrap();
    let saved = LevelDatStore::new(paths, WorldSaveLimits::default())
        .unwrap()
        .load_java_26_1()
        .unwrap()
        .data;
    assert_eq!(
        saved.enabled_features(),
        Some(&features.map(String::from)[..])
    );
    assert_eq!(
        saved.data_packs().unwrap().enabled(),
        ["vanilla", "trade_rebalance"]
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn world_save_reuses_custom_geometry() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-world-save-custom-geometry-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    overwrite_fixture_chunk(&root, &short_fixture_chunk());
    let paths = WorldSavePaths::new(&root).unwrap();
    let region_path = paths
        .builtin_dimension_dir(Java26BuiltinDimension::Overworld)
        .join("region/r.0.0.mca");
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let geometries = custom_short_geometries(bootstrap.world_gen_settings());
    let candidate = storage
        .load_candidate_with_geometries(bootstrap, geometries, &test_world_clock_ids())
        .unwrap();
    let mut short_column = ChunkColumn::new(
        0,
        16,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
        )
        .unwrap(),
        0,
    );
    short_column
        .set_block_state(
            1,
            0,
            2,
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:dirt")
                    .unwrap()
                    .id,
            )
            .unwrap(),
        )
        .unwrap();
    let mut request = save_patch_request(Vec::new());
    request.chunk_writes = vec![Java26CreatedChunk {
        dimension: Java26BuiltinDimension::Overworld.into(),
        chunk_x: 0,
        chunk_z: 0,
        document: crate::chunk_writer::full_chunk_document(
            &short_column,
            0,
            0,
            &|_| Some("minecraft:plains".to_string()),
            0,
        )
        .unwrap(),
    }];
    storage.save_snapshot(&candidate, &request).unwrap();
    let saved = RegionFileStore::new(
        &region_path,
        region_path.parent().unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap()
    .read_chunk(0, 0)
    .unwrap()
    .unwrap();
    assert!(format!("{saved:?}").contains("minecraft:dirt"));
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

fn write_chunk_tickets(paths: &WorldSavePaths, ticket_type: &str, level: i32) {
    write_chunk_ticket_at(paths, ticket_type, level, [0, 0], None);
}

fn write_chunk_ticket_at(
    paths: &WorldSavePaths,
    ticket_type: &str,
    level: i32,
    chunk_pos: [i32; 2],
    ticks_left: Option<i64>,
) {
    let dir = paths
        .builtin_dimension_dir(Java26BuiltinDimension::Overworld)
        .join("data")
        .join("minecraft");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("chunk_tickets.dat");
    let mut ticket = vec![
        nbt_entry("chunk_pos", NbtSummaryValue::IntArray(chunk_pos.to_vec())),
        nbt_entry("level", NbtSummaryValue::Int(level)),
        nbt_entry("type", NbtSummaryValue::string(ticket_type)),
    ];
    if let Some(ticks_left) = ticks_left {
        ticket.push(nbt_entry("ticks_left", NbtSummaryValue::Long(ticks_left)));
    }
    let document = NamedCompound::new(
        "",
        vec![
            nbt_entry(
                "data",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "tickets",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(ticket)]),
                )]),
            ),
            nbt_entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        ],
    );
    let bytes = encode_gzip_named_compound(&path, &document, WorldSaveLimits::default()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn scoreboard_saved_data_round_trips_through_the_save_request() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-scoreboard-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let limits = WorldSaveLimits::default();

    let storage = Java26WorldStorage::open(paths.clone(), limits).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.scoreboard, Java26ScoreboardPacked::default());
    let mut request = save_patch_request(Vec::new());
    request.scoreboard = Java26ScoreboardPacked {
        objectives: vec![crate::Java26ScoreboardObjective {
            name: "kills".into(),
            criteria: "playerKillCount".to_string(),
            display_name: NbtSummaryValue::string("Kills"),
            render_type: bbb_protocol::objective_render_types::ObjectiveRenderType::Integer,
            display_auto_update: false,
            format: None,
        }],
        scores: vec![crate::Java26ScoreboardPackedScore {
            owner: "alice".into(),
            objective: "kills".into(),
            score: 7,
            locked: false,
            display: None,
            format: None,
        }],
        display_slots: vec![(
            bbb_protocol::scoreboard_display_slots::ScoreboardDisplaySlot::Sidebar,
            "kills".into(),
        )],
        teams: vec![crate::Java26ScoreboardTeam {
            name: "red".into(),
            display_name: Some(NbtSummaryValue::string("Red")),
            color: Some(bbb_protocol::chat_formattings::ChatFormatting::Red),
            allow_friendly_fire: false,
            see_friendly_invisibles: true,
            member_name_prefix: None,
            member_name_suffix: None,
            name_tag_visibility:
                bbb_facts::scoreboard_team_options::TeamVisibility::HideForOtherTeams,
            death_message_visibility: bbb_facts::scoreboard_team_options::TeamVisibility::Always,
            collision_rule: bbb_facts::scoreboard_team_options::TeamCollisionRule::PushOwnTeam,
            players: vec!["alice".into()],
        }],
    };
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    // One document for the whole save, beside the other server-wide
    // `SavedData` (`MinecraftServer.java:344`), not one per dimension.
    assert!(paths
        .saved_data_dir()
        .join("minecraft")
        .join("scoreboard.dat")
        .is_file());
    for dimension in Java26BuiltinDimension::ALL {
        assert!(!paths
            .builtin_dimension_dir(dimension)
            .join("data")
            .join("minecraft")
            .join("scoreboard.dat")
            .exists());
    }

    let storage = Java26WorldStorage::open(paths, limits).unwrap();
    let reopened = storage.load_candidate().unwrap();
    assert_eq!(reopened.scoreboard, request.scoreboard);
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn weather_saved_data_is_one_save_wide_document_and_round_trips() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-weather-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let limits = WorldSaveLimits::default();

    let storage = Java26WorldStorage::open(paths.clone(), limits).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.weather, Java26WeatherData::default());
    let mut request = save_patch_request(Vec::new());
    request.weather = Java26WeatherData {
        clear_weather_time: 11,
        rain_time: 2222,
        thunder_time: 333,
        raining: true,
        thundering: true,
    };
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    // One document for the whole save, beside the other server-wide
    // `SavedData` (`MinecraftServer.java:344`), not one per dimension.
    assert!(paths
        .saved_data_dir()
        .join("minecraft")
        .join("weather.dat")
        .is_file());
    for dimension in Java26BuiltinDimension::ALL {
        assert!(!paths
            .builtin_dimension_dir(dimension)
            .join("data")
            .join("minecraft")
            .join("weather.dat")
            .exists());
    }

    let storage = Java26WorldStorage::open(paths, limits).unwrap();
    let reopened = storage.load_candidate().unwrap();
    assert_eq!(reopened.weather, request.weather);
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// `SavedDataStorage.getDataFile` (`SavedDataStorage.java:56-63`) resolves
/// `TicketStorage.TYPE` (`TicketStorage.java:40-42`) under the folder
/// `ServerChunkCache` builds per dimension (`ServerChunkCache.java:94`,
/// `:102-103`), which is not the world-level `data/` the game rules live
/// in. A save that never wrote the file has no ticket rather than an error.
#[test]
fn chunk_tickets_come_from_the_dimension_data_folder_and_carry_their_expiry() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-chunk-tickets-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let limits = WorldSaveLimits::default();
    assert_eq!(
        load_java_26_1_chunk_tickets(&paths, Java26BuiltinDimension::Overworld, limits).unwrap(),
        Vec::new(),
        "a save with no chunk_tickets.dat has no ticket"
    );

    write_chunk_tickets(&paths, "minecraft:forced", 31);
    let expected = vec![Java26SavedChunkTicket {
        chunk_x: 0,
        chunk_z: 0,
        level: 31,
        ticks_left: None,
    }];
    assert_eq!(
        load_java_26_1_chunk_tickets(&paths, Java26BuiltinDimension::Overworld, limits).unwrap(),
        expected
    );
    // The world-level `data/` folder is a different `SavedDataStorage` and
    // holds no tickets, so a reader that looked there would see nothing.
    assert!(!paths
        .saved_data_dir()
        .join("minecraft")
        .join("chunk_tickets.dat")
        .exists());

    let storage = Java26WorldStorage::open(paths.clone(), limits).unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert_eq!(
        candidate.chunk_tickets[&level_key(Java26BuiltinDimension::Overworld)],
        expected
    );
    assert_eq!(
        candidate.chunk_tickets[&level_key(Java26BuiltinDimension::Nether)],
        Vec::new()
    );
    storage.close().unwrap();

    // `TicketType.PORTAL` is the other type `packTickets` writes
    // (`TicketType.java:23`, `register("portal", 300L, 15)`), and a save
    // carrying one is one a server wrote within 300 ticks of somebody
    // stepping through a portal. It comes back with the `ticks_left` it was
    // written with (`Ticket.java:15`).
    write_chunk_ticket_at(&paths, "minecraft:portal", 31, [0, 0], Some(120));
    assert_eq!(
        load_java_26_1_chunk_tickets(&paths, Java26BuiltinDimension::Overworld, limits).unwrap(),
        vec![Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: Some(120),
        }]
    );
    // `Codec.LONG.optionalFieldOf("ticks_left", 0L)` (`Ticket.java:15`)
    // defaults to zero rather than to the type's timeout, so a portal
    // ticket written without the field comes back already spent.
    write_chunk_ticket_at(&paths, "minecraft:portal", 31, [0, 0], None);
    assert_eq!(
        load_java_26_1_chunk_tickets(&paths, Java26BuiltinDimension::Overworld, limits).unwrap()[0]
            .ticks_left,
        Some(0)
    );

    // A type `packTickets` never writes is dropped.
    write_chunk_tickets(&paths, "minecraft:player_simulation", 31);
    assert_eq!(
        load_java_26_1_chunk_tickets(&paths, Java26BuiltinDimension::Overworld, limits).unwrap(),
        Vec::new()
    );
}

#[test]
fn chunk_ticket_world_save_round_trips_all_dimensions_and_overwrites_expired_sets() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-chunk-ticket-round-trip-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut request = save_patch_request(Vec::new());
    request.chunk_tickets = BTreeMap::from([
        (
            Java26BuiltinDimension::Overworld.into(),
            vec![
                Java26SavedChunkTicket {
                    chunk_x: 4,
                    chunk_z: -9,
                    level: 31,
                    ticks_left: None,
                },
                Java26SavedChunkTicket {
                    chunk_x: 4,
                    chunk_z: -9,
                    level: 30,
                    ticks_left: Some(0),
                },
                Java26SavedChunkTicket {
                    chunk_x: 8,
                    chunk_z: 3,
                    level: 29,
                    ticks_left: Some(119),
                },
            ],
        ),
        (
            Java26BuiltinDimension::Nether.into(),
            vec![Java26SavedChunkTicket {
                chunk_x: -1,
                chunk_z: 2,
                level: 31,
                ticks_left: Some(299),
            }],
        ),
        (
            Java26BuiltinDimension::End.into(),
            vec![Java26SavedChunkTicket {
                chunk_x: 0,
                chunk_z: 0,
                level: 31,
                ticks_left: None,
            }],
        ),
    ]);
    let first_tickets = request.chunk_tickets.clone();
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    for dimension in Java26BuiltinDimension::ALL {
        assert!(paths
            .builtin_dimension_dir(dimension)
            .join("data/minecraft/chunk_tickets.dat")
            .is_file());
    }
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.chunk_tickets, first_tickets);
    for dimension in Java26BuiltinDimension::ALL {
        let dimension = level_key(dimension);
        assert_eq!(
            java_26_1_chunk_tickets_document(&baseline.chunk_tickets[&dimension]),
            java_26_1_chunk_tickets_document(&first_tickets[&dimension])
        );
    }

    let retained_end = baseline.chunk_tickets[&level_key(Java26BuiltinDimension::End)].clone();
    let retained_overworld = vec![first_tickets[&level_key(Java26BuiltinDimension::Overworld)][0]];
    let mut request = save_patch_request(Vec::new());
    request.chunk_tickets = BTreeMap::from([
        (
            Java26BuiltinDimension::Overworld.into(),
            retained_overworld.clone(),
        ),
        (Java26BuiltinDimension::Nether.into(), Vec::new()),
    ]);
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert_eq!(
        reloaded.chunk_tickets[&level_key(Java26BuiltinDimension::Overworld)],
        retained_overworld
    );
    assert!(reloaded.chunk_tickets[&level_key(Java26BuiltinDimension::Nether)].is_empty());
    assert_eq!(
        reloaded.chunk_tickets[&level_key(Java26BuiltinDimension::End)],
        retained_end,
        "an unlisted dimension retains its previous whole file"
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_game_rule_document_round_trips() {
    let mut rules = BTreeMap::new();
    rules.insert(
        "minecraft:random_tick_speed".to_string(),
        GameRuleValue::Integer(7),
    );
    rules.insert(
        "minecraft:send_command_feedback".to_string(),
        GameRuleValue::Boolean(false),
    );
    // A rule the writing server's features enabled and this one's do not.
    // The file is read verbatim: `GameRuleMap.CODEC` dispatches on
    // `BuiltInRegistries.GAME_RULE` alone (`GameRuleMap.java:19-20`), and the
    // feature set is only consulted afterwards, by the `GameRules`
    // constructor (`gamerules/GameRules.java:99-110`).
    rules.insert(
        "minecraft:max_minecart_speed".to_string(),
        GameRuleValue::Integer(200),
    );
    let document = java_26_1_game_rules_document(&rules);
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-game-rules-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    fs::create_dir_all(paths.saved_data_dir().join("minecraft")).unwrap();
    let limits = WorldSaveLimits::default();
    fs::write(
        paths.game_rules_dat(),
        encode_gzip_named_compound(&paths.game_rules_dat(), &document, limits).unwrap(),
    )
    .unwrap();
    assert_eq!(load_java_26_1_game_rules(&paths, limits).unwrap(), rules);

    fs::remove_dir_all(&root).unwrap();
}

fn overworld_stone_chunk_with_edits(
    edits: &[(usize, i32, usize, &str)],
    structures: Option<&NbtSummaryValue>,
) -> Java26CreatedChunk {
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    let mut column = ChunkColumn::new(-64, 24, stone, 0);
    for &(x, y, z, name) in edits {
        let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
        let state =
            bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap();
        column
            .set_block_state(x as i32, y, z as i32, state)
            .unwrap();
    }
    Java26CreatedChunk {
        dimension: Java26BuiltinDimension::Overworld.into(),
        chunk_x: 0,
        chunk_z: 0,
        document: crate::chunk_writer::full_chunk_document_with_light(
            &column,
            0,
            0,
            &|_| Some("minecraft:plains".to_string()),
            None,
            structures,
            &[],
            true,
        )
        .unwrap(),
    }
}

#[test]
fn save_snapshot_round_trips_clock_and_block_state_edits() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-save-snapshot-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let request = Java26SaveRequest {
        data_packs: None,
        poi_edits: Vec::new(),
        level_state: fixture_level_state(),
        region_timestamp: 200,
        region_compression: RegionCompression::Zlib,
        player_patches: Vec::new(),
        departed_player_stats: BTreeMap::new(),
        player_advancements: BTreeMap::new(),
        game_rules: BTreeMap::new(),
        world_clocks: baseline.world_clocks.clone(),
        map_writes: BTreeMap::new(),
        map_last_id: None,
        world_borders: baseline.world_borders.clone(),
        raids: baseline.raids.clone(),
        ender_dragon_fights: BTreeMap::new(),
        chunk_tickets: BTreeMap::new(),
        random_sequences: RandomSequencesSaved::default(),
        scheduled_events: Vec::new(),
        stopwatches: Default::default(),
        weather: Java26WeatherData::default(),
        wandering_trader: None,
        scoreboard: Java26ScoreboardPacked::default(),
        custom_boss_events: Default::default(),
        command_storage_writes: BTreeMap::new(),
        item_stack_resolver: SaveHolderResolver::unavailable(),
        chunk_writes: vec![overworld_stone_chunk_with_edits(
            &[(1, -64, 2, "minecraft:dirt")],
            None,
        )],
        chunk_write_epochs: BTreeMap::new(),
        entity_writes: Vec::new(),
    };
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert_eq!(reloaded.level.game_time(), 4_200);
    assert_eq!(reloaded.level.last_played(), 1_786_000_000_999);
    assert_eq!(reloaded.world_gen_settings.seed, 42);
    let chunk = &stored_chunk(
        &reloaded,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    );
    let section = &chunk.data.sections()[0];
    let block_states = section.block_states.as_ref().unwrap();
    assert_eq!(
        block_states.value_at(1, 0, 2).unwrap().name,
        "minecraft:dirt"
    );
    assert_eq!(
        block_states.value_at(0, 0, 0).unwrap().name,
        "minecraft:stone"
    );
    assert_eq!(block_states.bits_per_entry, 4);
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn successive_save_snapshots_overwrite_the_current_region_document() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-successive-save-snapshots-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut request = save_patch_request(Vec::new());
    request.chunk_writes = vec![overworld_stone_chunk_with_edits(
        &[(1, -64, 2, "minecraft:dirt")],
        None,
    )];
    storage.save_snapshot(&baseline, &request).unwrap();

    request.chunk_writes = vec![overworld_stone_chunk_with_edits(
        &[
            (1, -64, 2, "minecraft:dirt"),
            (2, -64, 2, "minecraft:gold_block"),
        ],
        None,
    )];
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let chunk = stored_chunk(
        &reloaded,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    );
    let block_states = chunk.data.sections()[0].block_states.as_ref().unwrap();
    assert_eq!(
        block_states.value_at(1, 0, 2).unwrap().name,
        "minecraft:dirt"
    );
    assert_eq!(
        block_states.value_at(2, 0, 2).unwrap().name,
        "minecraft:gold_block"
    );
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn generation_rewrite_and_followup_edit_round_trip_owned_structure_snapshot() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-structure-rewrite-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let structures = NbtSummaryValue::Compound(vec![
        nbt_entry("starts", NbtSummaryValue::Compound(Vec::new())),
        nbt_entry(
            "References",
            NbtSummaryValue::Compound(vec![nbt_entry(
                "minecraft:mineshaft",
                NbtSummaryValue::LongArray(vec![0]),
            )]),
        ),
    ]);
    let mut request = save_patch_request(Vec::new());
    request.chunk_writes = vec![overworld_stone_chunk_with_edits(&[], Some(&structures))];
    storage.save_snapshot(&baseline, &request).unwrap();

    let mut edit_request = save_patch_request(Vec::new());
    edit_request.chunk_writes = vec![overworld_stone_chunk_with_edits(
        &[(1, -64, 2, "minecraft:dirt")],
        Some(&structures),
    )];
    storage.save_snapshot(&baseline, &edit_request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let chunk = &stored_chunk(
        &reloaded,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    );
    assert_eq!(chunk.data.structures(), &structures);
    assert_eq!(
        chunk.data.sections()[0]
            .block_states
            .as_ref()
            .unwrap()
            .value_at(1, 0, 2)
            .unwrap()
            .name,
        "minecraft:dirt"
    );
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

/// The level state the fixture save is written back with: the fixture's own
/// `difficulty_settings`/`GameType`, plus a moved clock.
fn fixture_level_state() -> Java26LevelSaveState {
    Java26LevelSaveState {
        game_time: 4_200,
        last_played: 1_786_000_000_999,
        game_type: VANILLA_GAME_TYPE_CREATIVE_ID,
        initialized: true,
        difficulty: bbb_protocol::difficulties::Difficulty::Normal,
        difficulty_locked: false,
        spawn: crate::Java26LevelSpawn::new(
            "minecraft:overworld".to_string(),
            [0, 64, 0],
            0.0,
            0.0,
        ),
    }
}

fn map_saved_data_fixture(center_x: i32) -> Java26MapSavedData {
    Java26MapSavedData {
        dimension: "minecraft:overworld".to_string(),
        center_x,
        center_z: -32,
        scale: 2,
        colors: vec![7; 128 * 128],
        tracking_position: true,
        unlimited_tracking: false,
        locked: false,
        banners: Vec::new(),
        frames: Vec::new(),
    }
}

fn write_map_saved_data(paths: &WorldSavePaths, id: i32, document: &NamedCompound) -> Vec<u8> {
    fs::create_dir_all(paths.maps_dir()).unwrap();
    let bytes = encode_gzip_named_compound(
        &paths.map_data_dat(id),
        document,
        WorldSaveLimits::default(),
    )
    .unwrap();
    fs::write(paths.map_data_dat(id), &bytes).unwrap();
    bytes
}

#[test]
fn world_save_maps_publish_only_dirty_documents_and_reopen() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-maps-dirty-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let mut dirty_document = java_26_1_map_document(&map_saved_data_fixture(1));
    let NbtSummaryValue::Compound(data) = &mut dirty_document
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "data")
        .unwrap()
        .value
    else {
        unreachable!()
    };
    data.push(nbt_entry("future_field", NbtSummaryValue::Int(91)));
    let dirty_before = write_map_saved_data(&paths, 4, &dirty_document);
    let untouched_before = write_map_saved_data(
        &paths,
        -5,
        &java_26_1_map_document(&map_saved_data_fixture(2)),
    );
    let index_before = encode_gzip_named_compound(
        &paths.map_index_dat(),
        &java_26_1_map_index_document(9),
        WorldSaveLimits::default(),
    )
    .unwrap();
    fs::write(paths.map_index_dat(), &index_before).unwrap();
    fs::write(paths.maps_dir().join("owner-note.bin"), b"unknown bytes").unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert!(baseline.maps.is_empty());
    assert_eq!(baseline.stored_maps.len(), 2);
    let mut request = save_patch_request(Vec::new());
    request.map_writes.insert(4, map_saved_data_fixture(100));
    storage.save_snapshot(&baseline, &request).unwrap();
    assert_ne!(fs::read(paths.map_data_dat(4)).unwrap(), dirty_before);
    assert_eq!(fs::read(paths.map_data_dat(-5)).unwrap(), untouched_before);
    assert_eq!(fs::read(paths.map_index_dat()).unwrap(), index_before);
    assert_eq!(
        fs::read(paths.maps_dir().join("owner-note.bin")).unwrap(),
        b"unknown bytes"
    );
    let dirty_after = fs::read(paths.map_data_dat(4)).unwrap();
    let mut decoded = Vec::new();
    flate2::read::GzDecoder::new(dirty_after.as_slice())
        .read_to_end(&mut decoded)
        .unwrap();
    assert!(!decoded
        .windows(b"future_field".len())
        .any(|window| window == b"future_field"));
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reopened = storage.load_candidate().unwrap();
    assert_eq!(reopened.load_map(4).unwrap().unwrap().center_x, 100);
    assert_eq!(reopened.load_map(-5).unwrap().unwrap().center_x, 2);
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn world_save_command_storage_joins_the_atomic_snapshot_and_reloads() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-command-storage-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut request = save_patch_request(Vec::new());
    request.command_storage_writes.insert(
        "example".to_string(),
        BTreeMap::from([(
            "value".to_string(),
            vec![nbt_entry("answer", NbtSummaryValue::Int(42))],
        )]),
    );
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reopened = storage.load_candidate().unwrap();
    assert_eq!(
        reopened.command_storage.namespace("example").unwrap()["value"],
        vec![nbt_entry("answer", NbtSummaryValue::Int(42))]
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

pub(super) fn save_patch_request(patches: Vec<Java26PlayerSavePatch>) -> Java26SaveRequest {
    Java26SaveRequest {
        data_packs: None,
        poi_edits: Vec::new(),
        level_state: fixture_level_state(),
        region_timestamp: 200,
        region_compression: RegionCompression::Zlib,
        player_patches: patches,
        departed_player_stats: BTreeMap::new(),
        player_advancements: BTreeMap::new(),
        game_rules: BTreeMap::new(),
        world_clocks: world_clocks::default_world_clocks(),
        map_writes: BTreeMap::new(),
        map_last_id: None,
        world_borders: default_world_borders(),
        raids: BTreeMap::new(),
        ender_dragon_fights: BTreeMap::new(),
        chunk_tickets: BTreeMap::new(),
        random_sequences: RandomSequencesSaved::default(),
        scheduled_events: Vec::new(),
        stopwatches: Default::default(),
        weather: Java26WeatherData::default(),
        wandering_trader: None,
        scoreboard: Java26ScoreboardPacked::default(),
        custom_boss_events: Default::default(),
        command_storage_writes: BTreeMap::new(),
        item_stack_resolver: SaveHolderResolver::unavailable(),
        chunk_writes: Vec::new(),
        chunk_write_epochs: BTreeMap::new(),
        entity_writes: Vec::new(),
    }
}

pub(super) fn save_patch(uuid: Uuid) -> Java26PlayerSavePatch {
    Java26PlayerSavePatch {
        stats: bbb_server_types::stats::ServerStatsCounter::new(),
        uuid,
        position: [1.5, 64.0, 2.5],
        motion: [0.0; 3],
        rotation: [90.0, -30.0],
        on_ground: true,
        custom_name: None,
        custom_data: Vec::new(),
        tags: Vec::new(),
        root_vehicle: None,
        ender_pearls: Vec::new(),
        shoulder_entities: Default::default(),
        respawn: None,
        last_death_location: None,
        previous_game_mode: None,
        entered_nether_pos: None,
        raid_omen_position: None,
        warden_spawn_tracker: None,
        created: Java26CreatedPlayer {
            sleep_timer: 0,
            sleeping_pos: None,
            last_hurt_by: Default::default(),
            spawn_extra_particles_on_fall: false,
            seen_credits: false,
            impulse_reset_grace_time: 0,
            impulse_impact_position: None,
            has_visual_fire: false,
            silent: false,
            glowing: false,
            fall_distance: 0.0,
            remaining_fire_ticks: 0,
            air_supply: 300,
            ticks_frozen: 0,
            portal_cooldown: 0,
            dimension: "minecraft:overworld".to_string(),
            game_mode: GameType::Creative,
            entity_invulnerable: true,
            attributes: Vec::new(),
            active_effects: Vec::new(),
            locator_bar_icon: bbb_protocol::packets::WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: None,
            },
            damage: bbb_server_types::player_damage::PlayerDamageState::created(),
            food: bbb_server_types::food::FoodData::created(),
            experience: bbb_server_types::experience::PlayerExperience::created(),
            abilities: PlayerAbilities {
                invulnerable: true,
                flying: true,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            },
            inventory: vec![bbb_protocol::packets::ItemStackSummary::empty(); 43].into(),
            ender_items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 27].into(),
            fall_flying: false,
            selected_item_slot: 3,
            recipe_book_settings: [true, false, false, false, false, false, false, false],
            recipe_book_recipes: Vec::new(),
            recipe_book_to_be_displayed: Vec::new(),
        },
    }
}

#[test]
fn existing_player_save_reopen_rewrites_every_modeled_live_player_field() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-live-player-fields-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let player_uuid = Uuid::from_u128(76);
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    storage
        .save_snapshot(
            &baseline,
            &save_patch_request(vec![save_patch(player_uuid)]),
        )
        .unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut patch = save_patch(player_uuid);
    patch.position = [-4.5, 72.0, 8.5];
    patch.rotation = [-45.0, 15.0];
    patch.on_ground = false;
    patch.tags = vec!["latest".into()];
    patch.created.dimension = "minecraft:the_nether".to_string();
    patch.created.game_mode = GameType::Adventure;
    patch.created.entity_invulnerable = false;
    patch.created.abilities = PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.075,
        walking_speed: 0.25,
    };
    patch.created.selected_item_slot = 7;
    patch.created.recipe_book_settings = [false, true, true, false, true, false, false, true];
    let mut inventory = vec![bbb_protocol::packets::ItemStackSummary::empty(); 43];
    inventory[5] = bbb_protocol::packets::ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIRT_ID),
        count: 7,
        component_patch: Default::default(),
    };
    inventory[40] = bbb_protocol::packets::ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: Default::default(),
    };
    patch.created.inventory = inventory.into();
    patch.created.fall_flying = true;
    storage
        .save_snapshot(&baseline, &save_patch_request(vec![patch.clone()]))
        .unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reopened = storage.load_candidate().unwrap();
    let (_, player) = reopened.load_player(player_uuid).unwrap().unwrap();
    assert_eq!(player.position(), patch.position);
    assert_eq!(player.rotation(), patch.rotation);
    assert_eq!(player.on_ground(), patch.on_ground);
    assert_eq!(player.tags(), patch.tags);
    assert_eq!(player.dimension(), patch.created.dimension);
    assert_eq!(player.game_mode(), Some(patch.created.game_mode));
    assert_eq!(player.invulnerable(), patch.created.entity_invulnerable);
    assert_eq!(
        player.selected_item_slot(),
        patch.created.selected_item_slot
    );
    assert_eq!(
        player.recipe_book().settings(),
        patch.created.recipe_book_settings
    );
    assert!(player.fall_flying());
    let Some(NbtSummaryValue::List(inventory)) = player.inventory_input() else {
        panic!("inventory list");
    };
    assert_eq!(inventory.len(), 1);
    assert_eq!(
        bbb_protocol::nbt_summary::value_field(&inventory[0], "Slot"),
        Some(&NbtSummaryValue::Byte(5))
    );
    assert_eq!(
        bbb_registry_codec::read_item_stack(
            &inventory[0],
            &bbb_registry_codec::UnavailableHolderResolver
        )
        .unwrap()
        .count,
        7
    );
    let offhand =
        bbb_protocol::nbt_summary::value_field(player.equipment_input().unwrap(), "offhand")
            .unwrap();
    assert_eq!(
        bbb_registry_codec::read_item_stack(
            offhand,
            &bbb_registry_codec::UnavailableHolderResolver
        )
        .unwrap()
        .count,
        1
    );
    assert_eq!(
        (
            player.abilities().invulnerable(),
            player.abilities().flying(),
            player.abilities().may_fly(),
            player.abilities().instant_build(),
            player.abilities().fly_speed(),
            player.abilities().walk_speed(),
        ),
        (true, false, true, false, 0.075, 0.25)
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn root_vehicle_storage_save_reopen_replaces_tree_and_removes_stale_state() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-root-vehicle-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let player_uuid = Uuid::from_u128(77);
    let old_root = Uuid::from_u128(78);
    let new_root = Uuid::from_u128(79);

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut patch = save_patch(player_uuid);
    patch.root_vehicle = Some(Java26RootVehicle::new(
        old_root,
        crate::Java26EntityRecord::parse(&root_vehicle_entity_entries(old_root)).unwrap(),
    ));
    storage
        .save_snapshot(&baseline, &save_patch_request(vec![patch]))
        .unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(
        baseline
            .load_player(player_uuid)
            .unwrap()
            .unwrap()
            .1
            .root_vehicle()
            .unwrap()
            .attach()
            .unwrap(),
        old_root
    );
    let mut patch = save_patch(player_uuid);
    patch.root_vehicle = Some(Java26RootVehicle::new(
        new_root,
        crate::Java26EntityRecord::parse(&root_vehicle_entity_entries(new_root)).unwrap(),
    ));
    storage
        .save_snapshot(&baseline, &save_patch_request(vec![patch]))
        .unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(
        baseline
            .load_player(player_uuid)
            .unwrap()
            .unwrap()
            .1
            .root_vehicle()
            .unwrap()
            .entity()
            .uuid(),
        new_root
    );
    let patch = save_patch(player_uuid);
    storage
        .save_snapshot(&baseline, &save_patch_request(vec![patch]))
        .unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reopened = storage.load_candidate().unwrap();
    assert!(reopened
        .load_player(player_uuid)
        .unwrap()
        .unwrap()
        .1
        .root_vehicle()
        .is_none());
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}

/// The regression this exists for: a player the save has never held used to
/// fail the *whole* save, so one new player joining meant the level, the
/// blocks and every other player stopped being written until shutdown — and
/// the shutdown save failed too. A first save now creates the document, the
/// way `PlayerList.save` does.
#[test]
fn save_snapshot_creates_playerdata_for_a_player_the_save_has_never_held() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-save-new-player-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert!(
        baseline.players.is_empty(),
        "the fixture save holds no playerdata"
    );
    // The block edit rides along so the test can say the *rest* of the save
    // survived, not merely that the playerdata write returned Ok.
    let mut request = save_patch_request(vec![save_patch(Uuid::from_u128(77))]);
    request.chunk_writes = vec![overworld_stone_chunk_with_edits(
        &[(1, -64, 2, "minecraft:dirt")],
        None,
    )];
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let file = paths
        .player_data_dir()
        .join(format!("{}.dat", Uuid::from_u128(77).hyphenated()));
    assert!(file.is_file(), "no playerdata was written to {file:?}");

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (_, player) = reloaded.load_player(Uuid::from_u128(77)).unwrap().unwrap();
    let expected = save_patch(Uuid::from_u128(77));
    assert_eq!(player.uuid(), expected.uuid);
    assert_eq!(player.position(), expected.position);
    assert_eq!(player.rotation(), expected.rotation);
    assert!(player.on_ground());
    assert_eq!(player.dimension(), expected.created.dimension);
    assert_eq!(player.game_mode(), Some(expected.created.game_mode));
    assert_eq!(
        player.selected_item_slot(),
        expected.created.selected_item_slot
    );
    assert_eq!(
        player.recipe_book().settings(),
        expected.created.recipe_book_settings
    );
    let abilities = player.abilities();
    assert!(abilities.invulnerable() && abilities.flying() && abilities.may_fly());
    assert!(abilities.instant_build() && abilities.may_build());
    assert_eq!(abilities.fly_speed(), 0.05);
    assert_eq!(abilities.walk_speed(), 0.1);
    // The rest of the save is still there: this is the half the old failure
    // took down with it.
    assert_eq!(reloaded.level.game_time(), 4_200);
    let chunk = &stored_chunk(
        &reloaded,
        &level_key(Java26BuiltinDimension::Overworld),
        (0, 0),
    );
    let block_states = chunk.data.sections()[0].block_states.as_ref().unwrap();
    assert_eq!(
        block_states.value_at(1, 0, 2).unwrap().name,
        "minecraft:dirt"
    );
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

/// A second save of the same player is a patch, not a second creation: the
/// document the first save authored is the one the motion write lands on,
/// so anything an official server put in it survives.
#[test]
fn a_created_players_second_save_patches_the_document_it_already_has() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-save-repeat-player-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let uuid = Uuid::from_u128(77);
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    storage
        .save_snapshot(&baseline, &save_patch_request(vec![save_patch(uuid)]))
        .unwrap();
    storage.close().unwrap();

    // Something this server does not model, written into the document by
    // somebody else between the two saves.
    let player_store =
        crate::PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
    let mut document = player_store.load(uuid).unwrap().unwrap().document;
    document.entries_mut().push(bbb_protocol::NbtSummaryEntry {
        name: "unknown_future".into(),
        value: NbtSummaryValue::Long(7),
    });
    player_store.save(uuid, &document).unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert!(baseline.players.is_empty());
    assert!(baseline.stored_players.contains(&uuid));
    assert!(!baseline
        .load_player(uuid)
        .unwrap()
        .unwrap()
        .1
        .has_retained_raw());

    let mut patch = save_patch(uuid);
    patch.position = [8.5, 70.0, 9.5];
    patch.on_ground = false;
    storage
        .save_snapshot(&baseline, &save_patch_request(vec![patch]))
        .unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (_, player) = reloaded.load_player(uuid).unwrap().unwrap();
    assert_eq!(player.position(), [8.5, 70.0, 9.5]);
    assert!(!player.on_ground());
    let stored = crate::PlayerDataStore::new(paths, WorldSaveLimits::default())
        .unwrap()
        .load(uuid)
        .unwrap()
        .unwrap()
        .document;
    assert!(
        stored
            .entries()
            .iter()
            .any(|entry| entry.name == "unknown_future" && entry.value == NbtSummaryValue::Long(7)),
        "the patch path dropped a field this server does not model"
    );
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

/// `PlayerList.remove` writes a leaver's statistics file on the spot
/// (`PlayerList.java:302` through `:288-291`). This server has no file
/// handle at a disconnect, so the counter travels on the save request
/// instead; what must not differ is the file that ends up on disk.
#[test]
fn a_departed_profiles_counter_reaches_its_statistics_file() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-save-departed-stats-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let departed = Uuid::from_u128(91);
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut counter = bbb_server_types::stats::ServerStatsCounter::new();
    counter.increment(
        bbb_server_types::stats::custom_stat(bbb_server_types::stats::CUSTOM_STAT_PLAY_TIME),
        42,
    );
    counter.increment(
        bbb_server_types::stats::custom_stat(bbb_server_types::stats::CUSTOM_STAT_LEAVE_GAME),
        1,
    );
    let mut request = save_patch_request(Vec::new());
    request
        .departed_player_stats
        .insert(departed, counter.clone());
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    // The dirty set is send-side bookkeeping the file does not carry
    // (`ServerStatsCounter.java:98-102`), so the counts are what has to
    // match rather than the whole counter.
    assert!(reloaded.player_stats.is_empty());
    assert!(reloaded.stored_player_stats.contains(&departed));
    let restored = reloaded
        .load_player_stats(departed)
        .unwrap()
        .expect("the departed profile's counts are on disk, playerdata document or no");
    assert_eq!(restored.to_json(), counter.to_json());
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

/// `PlayerList.save` writes the statistics and the advancements of one player
/// together (`PlayerList.java:286-297`), and the advancement half lands in
/// `players/advancements/<uuid>.json` (`LevelResource.java:4`).
#[test]
fn a_profiles_advancement_progress_reaches_its_own_file() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-save-advancements-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let profile = Uuid::from_u128(92);
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert!(baseline.player_advancements.is_empty());
    let advancements = Java26PlayerAdvancements {
        progress: BTreeMap::from([(
            "minecraft:story/root".to_string(),
            AdvancementProgress::from_criteria(BTreeMap::from([(
                "crafting_table".into(),
                CriterionProgress::obtained(1_788_143_445_000),
            )])),
        )]),
    };
    let mut request = save_patch_request(Vec::new());
    request
        .player_advancements
        .insert(profile, advancements.clone());
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert!(reloaded.player_advancements.is_empty());
    assert!(reloaded.stored_player_advancements.contains(&profile));
    assert_eq!(
        reloaded.load_player_advancements(profile).unwrap().as_ref(),
        Some(&advancements)
    );
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
#[ignore = "requires BBB_TEST_JAVA_26_1_WORLD from an authorized official-server run"]
fn verified_official_java_26_1_world_preflights_before_publication() {
    let root = PathBuf::from(std::env::var_os("BBB_TEST_JAVA_26_1_WORLD").unwrap());
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let world = storage.load_candidate().unwrap();
    assert!(
        !world.dimensions[&level_key(Java26BuiltinDimension::Overworld)]
            .stored_chunks
            .is_empty()
    );
    assert!(!world.stored_players.is_empty());
    storage.close().unwrap();
}

mod ordinary_derived;

#[test]
fn unavailable_save_holder_has_no_registry_publication() {
    use bbb_protocol::{PersistentRegistryMembership, PersistentRegistryTagMembers};
    let publication = SaveHolderResolver::unavailable();
    let resolver = publication.as_dyn();
    for registry in ["minecraft:item", "minecraft:enchantment", "example:missing"] {
        assert!(!resolver.registry_available(registry));
        assert_eq!(
            resolver.resource_membership(registry, "minecraft:any"),
            PersistentRegistryMembership::Unavailable
        );
        assert_eq!(
            resolver.tag_members(registry, "minecraft:any"),
            PersistentRegistryTagMembers::Unavailable
        );
        assert_eq!(resolver.holder_id(registry, "minecraft:any"), None);
        assert_eq!(resolver.holder_resource_id(registry, 0), None);
    }
}

#[test]
fn save_persists_data_packs_enabled_and_disabled_lists_to_level_dat() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-data-packs-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let updated_packs = crate::Java26DataPacks::new(
        vec!["vanilla".to_string(), "file/tweaks".to_string()],
        vec!["file/blocked".to_string()],
    );
    let mut request = save_patch_request(Vec::new());
    request.data_packs = Some(updated_packs.clone());
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    assert_eq!(bootstrap.level().data_packs(), Some(&updated_packs));

    // Saving with `data_packs: None` keeps the `Data.DataPacks` already in `level.dat`.
    let candidate = storage.load_candidate_with_geometries(
        bootstrap,
        Java26DimensionGeometries::from_builtin_settings(
            storage.load_bootstrap().unwrap().world_gen_settings(),
        ),
        &[],
    );
    let candidate = candidate.unwrap();
    let keep_request = save_patch_request(Vec::new());
    storage.save_snapshot(&candidate, &keep_request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    assert_eq!(
        storage.load_bootstrap().unwrap().level().data_packs(),
        Some(&updated_packs)
    );
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn untouched_offline_player_dat_is_not_retained_or_rewritten_on_save() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-offline-player-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let player_store =
        crate::PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();

    let offline_uuid = Uuid::from_u128(0xdead_beef_0001);
    let active_uuid = Uuid::from_u128(0xdead_beef_0002);
    let mut offline_doc = crate::created_player_document(
        &save_patch(offline_uuid),
        SaveHolderResolver::unavailable().as_dyn(),
    )
    .unwrap();
    offline_doc.entries_mut().push(nbt_entry(
        "unknown_offline_marker",
        NbtSummaryValue::Long(99),
    ));
    player_store.save(offline_uuid, &offline_doc).unwrap();
    let active_doc = crate::created_player_document(
        &save_patch(active_uuid),
        SaveHolderResolver::unavailable().as_dyn(),
    )
    .unwrap();
    player_store.save(active_uuid, &active_doc).unwrap();

    let offline_path = paths
        .player_data_dir()
        .join(format!("{}.dat", offline_uuid.hyphenated()));
    let offline_backup = paths
        .player_data_dir()
        .join(format!("{}.dat_old", offline_uuid.hyphenated()));
    let offline_bytes_before = fs::read(&offline_path).unwrap();
    assert!(!offline_backup.exists());

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert!(baseline.players.is_empty());
    assert!(baseline.stored_players.contains(&offline_uuid));
    assert!(baseline.stored_players.contains(&active_uuid));
    assert!(!baseline
        .load_player(offline_uuid)
        .unwrap()
        .unwrap()
        .1
        .has_retained_raw());
    assert!(!baseline
        .load_player(active_uuid)
        .unwrap()
        .unwrap()
        .1
        .has_retained_raw());

    let mut active_patch = save_patch(active_uuid);
    active_patch.position = [18.5, 65.0, -4.5];
    storage
        .save_snapshot(&baseline, &save_patch_request(vec![active_patch]))
        .unwrap();
    storage.close().unwrap();

    assert_eq!(fs::read(&offline_path).unwrap(), offline_bytes_before);
    assert!(!offline_backup.exists());

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    assert_eq!(
        reloaded
            .load_player(active_uuid)
            .unwrap()
            .unwrap()
            .1
            .position(),
        [18.5, 65.0, -4.5]
    );
    assert_eq!(
        reloaded
            .load_player(offline_uuid)
            .unwrap()
            .unwrap()
            .1
            .position(),
        save_patch(offline_uuid).position
    );
    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn modded_serializable_entities_survive_world_restart_and_chunk_reload() {
    use bbb_protocol::content_registry::{
        ContentRegistry, ContentRegistryBuilder, ModEntityRegistration,
    };
    use std::sync::Arc;

    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-mod-entities-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    save_fixture_world(&root);

    let mut builder = ContentRegistryBuilder::new();
    builder.register_entity(ModEntityRegistration {
        resource_id: "bbb_demo:inferno_warden".to_string(),
        width: 0.9,
        height: 2.9,
        serializable: true,
        ..Default::default()
    });
    builder.register_entity(ModEntityRegistration {
        resource_id: "bbb_demo:transient_spark".to_string(),
        width: 0.25,
        height: 0.25,
        serializable: false,
        ..Default::default()
    });
    let content_registry = Arc::new(builder.freeze().unwrap());

    let paths = WorldSavePaths::new(&root).unwrap();
    let warden_chunk_uuid = Uuid::from_u128(0xabc1);
    let transient_chunk_uuid = Uuid::from_u128(0xabc2);
    let warden_vehicle_uuid = Uuid::from_u128(0xabc3);
    let player_uuid = Uuid::from_u128(0xabc4);
    let warden_pearl_uuid = Uuid::from_u128(0xabc5);

    let mod_entity_nbt = |id: &str, uuid: Uuid| {
        vec![
            nbt_entry("id", NbtSummaryValue::string(id)),
            nbt_entry("UUID", uuid_int_array(uuid)),
            nbt_entry(
                "Pos",
                NbtSummaryValue::List(
                    [2.5_f64, 64.0, 2.5]
                        .into_iter()
                        .map(|v| NbtSummaryValue::Double(v.to_bits()))
                        .collect(),
                ),
            ),
        ]
    };

    let storage = Java26WorldStorage::open_with_content_registry(
        paths.clone(),
        WorldSaveLimits::default(),
        Arc::clone(&content_registry),
    )
    .unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut patch = save_patch(player_uuid);
    let warden_vehicle = crate::Java26EntityRecord::parse_with_content_registry(
        &mod_entity_nbt("bbb_demo:inferno_warden", warden_vehicle_uuid),
        &content_registry,
    )
    .unwrap();
    patch.root_vehicle = Some(Java26RootVehicle::new(warden_vehicle_uuid, warden_vehicle));
    let warden_pearl = crate::Java26EntityRecord::parse_with_content_registry(
        &mod_entity_nbt("bbb_demo:inferno_warden", warden_pearl_uuid),
        &content_registry,
    )
    .unwrap();
    patch.ender_pearls = vec![crate::player_schema::ender_pearls::Java26PlayerEnderPearl {
        dimension: level_key(Java26BuiltinDimension::Overworld),
        entity: warden_pearl,
    }];

    let mut request = save_patch_request(vec![patch]);
    request.entity_writes = vec![Java26EntityChunkWrite {
        dimension: level_key(Java26BuiltinDimension::Overworld),
        chunk_x: 0,
        chunk_z: 0,
        document: crate::entity_schema::entity_chunk_document(
            0,
            0,
            vec![
                mod_entity_nbt("bbb_demo:inferno_warden", warden_chunk_uuid),
                mod_entity_nbt("bbb_demo:transient_spark", transient_chunk_uuid),
            ],
        ),
    }];
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    // Restart with the modded ContentRegistry and confirm entities & RootVehicle reload.
    let reopened_storage = Java26WorldStorage::open_with_content_registry(
        paths.clone(),
        WorldSaveLimits::default(),
        Arc::clone(&content_registry),
    )
    .unwrap();
    let bootstrap = reopened_storage.load_bootstrap().unwrap();
    let geometries =
        Java26DimensionGeometries::from_builtin_settings(bootstrap.world_gen_settings());
    let reopened = reopened_storage
        .load_candidate_with_geometries(bootstrap, geometries, &[])
        .unwrap();
    let (_, reopened_player) = reopened.load_player(player_uuid).unwrap().unwrap();
    let loaded_vehicle = reopened_player
        .root_vehicle()
        .expect("modded RootVehicle must reload across restart");
    assert_eq!(loaded_vehicle.entity().id(), "bbb_demo:inferno_warden");
    assert_eq!(loaded_vehicle.entity().uuid(), warden_vehicle_uuid);
    let loaded_pearls = reopened_player.ender_pearls();
    assert_eq!(loaded_pearls.len(), 1);
    assert_eq!(
        loaded_pearls[0].entity.as_ref().id(),
        "bbb_demo:inferno_warden"
    );
    assert_eq!(loaded_pearls[0].entity.as_ref().uuid(), warden_pearl_uuid);

    let entity_chunk = reopened_storage
        .read_runtime_entity_chunk(
            &reopened,
            &level_key(Java26BuiltinDimension::Overworld),
            0,
            0,
        )
        .unwrap()
        .expect("saved entity chunk must exist");
    let loaded_entities: Vec<_> = entity_chunk.data.entities().collect();
    assert_eq!(loaded_entities.len(), 1);
    assert_eq!(loaded_entities[0].id(), "bbb_demo:inferno_warden");
    assert_eq!(loaded_entities[0].uuid(), warden_chunk_uuid);

    // Without the modded ContentRegistry, unknown entity ids drop instead of crashing.
    let vanilla_only = ContentRegistry::vanilla();
    assert!(!Arc::ptr_eq(reopened.content_registry(), vanilla_only));
    reopened_storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn autosave_skips_unchanged_saved_data_while_preserving_first_dirty_and_explicit_writes() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-autosave-skip-unchanged-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut request = save_patch_request(Vec::new());

    let game_rules_path = paths.game_rules_dat();
    let scoreboard_path = paths
        .saved_data_dir()
        .join("minecraft")
        .join("scoreboard.dat");

    // First autosave writes all SavedData files because none are cached yet.
    storage.save_autosave(&baseline, &request).unwrap();
    assert!(game_rules_path.is_file());
    assert!(scoreboard_path.is_file());

    // Tamper game_rules.dat with sentinel bytes outside storage; an unchanged autosave
    // skips rewriting it because its serialized bytes match the cached last-written copy.
    let sentinel = b"unchanged-sentinel";
    fs::write(&game_rules_path, sentinel).unwrap();
    fs::write(&scoreboard_path, sentinel).unwrap();
    storage.save_autosave(&baseline, &request).unwrap();
    assert_eq!(fs::read(&game_rules_path).unwrap(), sentinel);
    assert_eq!(fs::read(&scoreboard_path).unwrap(), sentinel);

    // Mutating game_rules marks it dirty so autosave rewrites only the changed file.
    request.game_rules.insert(
        "minecraft:keep_inventory".to_string(),
        GameRuleValue::Boolean(true),
    );
    storage.save_autosave(&baseline, &request).unwrap();
    assert_ne!(fs::read(&game_rules_path).unwrap(), sentinel);
    assert_eq!(fs::read(&scoreboard_path).unwrap(), sentinel);

    // An explicit save_snapshot always writes all SavedData files.
    storage.save_snapshot(&baseline, &request).unwrap();
    assert_ne!(fs::read(&scoreboard_path).unwrap(), sentinel);

    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn lazy_candidate_defers_reading_offline_playerdata_stats_advancements_and_maps() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-save-lazy-offline-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();

    // Place corrupt unvisited files on disk; startup must only index their
    // ids/uuids without opening or parsing them.
    let corrupt_uuid = Uuid::from_u128(0xbad0_0001);
    let valid_uuid = Uuid::from_u128(0x600d_0002);
    fs::create_dir_all(paths.player_data_dir()).unwrap();
    fs::create_dir_all(paths.player_stats_dir()).unwrap();
    fs::create_dir_all(crate::player_advancements::player_advancements_dir(&paths)).unwrap();
    fs::create_dir_all(paths.maps_dir()).unwrap();

    let corrupt_player = paths
        .player_data_dir()
        .join(format!("{}.dat", corrupt_uuid.hyphenated()));
    let corrupt_stats = paths.player_stats_file(corrupt_uuid);
    let corrupt_adv = crate::player_advancements::player_advancements_file(&paths, corrupt_uuid);
    let corrupt_map = paths.map_data_dat(99);
    fs::write(&corrupt_player, b"not-gzip-playerdata").unwrap();
    fs::write(&corrupt_stats, b"{not-json").unwrap();
    fs::write(&corrupt_adv, b"{not-json").unwrap();
    fs::write(&corrupt_map, b"not-gzip-map").unwrap();

    let player_store =
        crate::PlayerDataStore::new(paths.clone(), WorldSaveLimits::default()).unwrap();
    let valid_doc = crate::created_player_document(
        &save_patch(valid_uuid),
        SaveHolderResolver::unavailable().as_dyn(),
    )
    .unwrap();
    player_store.save(valid_uuid, &valid_doc).unwrap();
    write_map_saved_data(
        &paths,
        7,
        &java_26_1_map_document(&map_saved_data_fixture(42)),
    );

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let candidate = storage.load_candidate().unwrap();
    assert!(candidate.players.is_empty());
    assert!(candidate.player_stats.is_empty());
    assert!(candidate.player_advancements.is_empty());
    assert!(candidate.maps.is_empty());
    assert!(candidate.stored_players.contains(&corrupt_uuid));
    assert!(candidate.stored_players.contains(&valid_uuid));
    assert!(candidate.stored_player_stats.contains(&corrupt_uuid));
    assert!(candidate.stored_player_advancements.contains(&corrupt_uuid));
    assert!(candidate.stored_maps.contains(&7));
    assert!(candidate.stored_maps.contains(&99));

    // On-demand reads of valid entries succeed while corrupt entries fail only
    // when actually requested.
    assert_eq!(
        candidate
            .load_player(valid_uuid)
            .unwrap()
            .unwrap()
            .1
            .position(),
        save_patch(valid_uuid).position
    );
    assert_eq!(candidate.load_map(7).unwrap().unwrap().center_x, 42);
    assert!(candidate.load_player(corrupt_uuid).is_err());
    assert!(candidate.load_player_stats(corrupt_uuid).is_err());
    assert!(candidate.load_player_advancements(corrupt_uuid).is_err());
    assert!(candidate.load_map(99).is_err());

    // Saving unrelated changes leaves untouched corrupt offline files intact.
    storage
        .save_snapshot(&candidate, &save_patch_request(Vec::new()))
        .unwrap();
    assert_eq!(fs::read(&corrupt_player).unwrap(), b"not-gzip-playerdata");
    assert_eq!(fs::read(&corrupt_stats).unwrap(), b"{not-json");
    assert_eq!(fs::read(&corrupt_adv).unwrap(), b"{not-json");
    assert_eq!(fs::read(&corrupt_map).unwrap(), b"not-gzip-map");

    storage.close().unwrap();
    fs::remove_dir_all(&root).unwrap();
}
