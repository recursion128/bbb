//! The documents a save is made of when there is no save yet.
//!
//! `--world` can only open a directory that already holds a level, so a
//! headless server configured entirely by its command line cannot run with
//! persistence until some other server has made it one. Vanilla does not have
//! that problem: `Main.createNewWorldData` (`Main.java:226-259`) builds a
//! `PrimaryLevelData` from the properties and hands it to the storage, which
//! writes `level.dat` on the first save.
//!
//! This module is the same seam. It authors the two whole-file documents a new
//! 26.1 save opens with — `level.dat` and
//! `data/minecraft/world_gen_settings.dat` — and nothing else: the writes
//! that publish them and the region chunks that fill them belong to
//! [`super::world_storage`].
//!
//! A `level-type` save carries Vanilla's own preset dimensions verbatim. The
//! fixture save ([`created_save`]) instead rewrites the flat stems to the
//! fixture column and keeps the normal-preset Nether and End.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use bbb_protocol::{
    json_value_to_nbt_summary, nbt_summary::entry, NbtSummaryEntry, NbtSummaryValue,
};

use bbb_server_datapack::dimension_type::builtin_dimension_type_shape;
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::flat_generator::{served_level_sources_from_stems, FlatLevelSource};
use bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot;
use bbb_server_persistence::chunk_writer::full_chunk_document;
use bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings;
use bbb_server_persistence::world_storage::Java26WorldCandidate;
use bbb_server_persistence::{Java26WorldBootstrap, Java26WorldStorage, JAVA_MAIN_SERIES};
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::fixture::FlatFixture;
use bbb_server_types::save::{
    Java26BuiltinDimension, Java26LevelKey, Java26WorldGenSettings, NamedCompound, SaveError,
    SaveResult, JAVA_26_1_DATA_VERSION, JAVA_26_1_STORAGE_VERSION,
};
pub use bbb_server_types::save::{
    Java26CreatedChunk, Java26CreatedSave, Java26WorldCreation, Java26WorldOptions,
    Java26WorldPreset, CREATED_WORLD_SEED,
};
use bbb_server_types::schema_support::invalid_schema;

const SCHEMA: &str = "world_creation";

/// The preset supplies the created stem keys, dimension types and flat source.
/// Each generator is then selected explicitly by [`select_created_generators`].
const FLAT_PRESET_PATH: &str = "worldgen/world_preset/flat.json";
const NORMAL_PRESET_PATH: &str = "worldgen/world_preset/normal.json";
const LARGE_BIOMES_PRESET_PATH: &str = "worldgen/world_preset/large_biomes.json";
const AMPLIFIED_PRESET_PATH: &str = "worldgen/world_preset/amplified.json";
const OVERWORLD_STEM: &str = "minecraft:overworld";
const NETHER_STEM: &str = "minecraft:the_nether";
const END_STEM: &str = "minecraft:the_end";

const fn preset_path(preset: Java26WorldPreset) -> &'static str {
    match preset {
        Java26WorldPreset::Normal => NORMAL_PRESET_PATH,
        Java26WorldPreset::Flat => FLAT_PRESET_PATH,
        Java26WorldPreset::LargeBiomes => LARGE_BIOMES_PRESET_PATH,
        Java26WorldPreset::Amplified => AMPLIFIED_PRESET_PATH,
    }
}

/// Completes a bootstrap using the immutable live registry generation.
/// `LevelStorageSource.java:141-167` resolves saved world data in that
/// registry context only after `WorldLoader` has loaded it.
pub fn load_candidate_from_bootstrap(
    storage: &Java26WorldStorage,
    bootstrap: Java26WorldBootstrap,
    registries: &DatapackGeneration,
) -> SaveResult<Java26WorldCandidate> {
    load_candidate_from_bootstrap_with_content_registry(
        storage,
        bootstrap,
        registries,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn load_candidate_from_bootstrap_with_content_registry(
    storage: &Java26WorldStorage,
    bootstrap: Java26WorldBootstrap,
    registries: &DatapackGeneration,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> SaveResult<Java26WorldCandidate> {
    let geometries =
        bbb_server_datapack::dimension_type::dimension_geometries_from_generation_with_content_registry(
            bootstrap.world_gen_settings(),
            registries,
            content_registry,
        );
    let world_clock_ids = registries
        .registry("minecraft:world_clock")
        .ok_or_else(|| SaveError::InvalidNbt {
            path: Some(storage.paths().world_clocks_dat()),
            detail: "live datapack generation has no minecraft:world_clock registry".to_string(),
        })?
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    storage.load_candidate_with_geometries(bootstrap, geometries, &world_clock_ids)
}

/// The `level.dat` document a new save is published with.
///
/// Field for field, `PrimaryLevelData.setTagData` (:122-145) with the
/// new-world constructor's values (:72-86): no `singleplayer_uuid` on a
/// dedicated server, no removed feature flags, an empty brand list, and a game
/// time of zero. `initialized` is the one place this differs and has to: vanilla
/// writes `false` and turns it true once it has generated a spawn, while this
/// save is published complete, with its chunks already in it.
pub fn created_level_document(request: &Java26WorldCreation) -> SaveResult<NamedCompound> {
    level_document(request, true, false)
}

/// `initialized` false makes the loader run `MinecraftServer.setInitialSpawn`
/// before the first tick, which is how a noise save without chunks gets its spawn.
fn level_document(
    request: &Java26WorldCreation,
    initialized: bool,
    hardcore: bool,
) -> SaveResult<NamedCompound> {
    if request.level_name.is_empty() {
        return Err(invalid_schema(SCHEMA, "a created level needs a name"));
    }
    if !(0..=3).contains(&request.game_type) {
        return Err(invalid_schema(
            SCHEMA,
            "level.dat GameType must be in 0..=3",
        ));
    }
    let data = vec![
        entry("ServerBrands", NbtSummaryValue::List(Vec::new())),
        entry("WasModded", NbtSummaryValue::Byte(0)),
        entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        entry("GameType", NbtSummaryValue::Int(request.game_type)),
        entry(
            "spawn",
            NbtSummaryValue::Compound(vec![
                entry("dimension", NbtSummaryValue::string(OVERWORLD_STEM)),
                entry(
                    "pos",
                    NbtSummaryValue::IntArray(request.spawn_position.to_vec()),
                ),
                entry("yaw", NbtSummaryValue::Float(0.0_f32.to_bits())),
                entry("pitch", NbtSummaryValue::Float(0.0_f32.to_bits())),
            ]),
        ),
        entry("Time", NbtSummaryValue::Long(0)),
        entry("LastPlayed", NbtSummaryValue::Long(request.last_played)),
        entry(
            "LevelName",
            NbtSummaryValue::string(request.level_name.clone()),
        ),
        entry("version", NbtSummaryValue::Int(JAVA_26_1_STORAGE_VERSION)),
        // `Main.createNewWorldData` passes `false` for a dedicated server
        // (`Main.java:246`); commands reach this server through its management
        // endpoint rather than through the level's cheat flag.
        entry("allowCommands", NbtSummaryValue::Byte(0)),
        entry("initialized", NbtSummaryValue::Byte(i8::from(initialized))),
        entry(
            "difficulty_settings",
            NbtSummaryValue::Compound(vec![
                entry(
                    "difficulty",
                    NbtSummaryValue::string(request.difficulty.clone()),
                ),
                entry("hardcore", NbtSummaryValue::Byte(i8::from(hardcore))),
                entry("locked", NbtSummaryValue::Byte(0)),
            ]),
        ),
        entry(
            "DataPacks",
            NbtSummaryValue::Compound(vec![
                entry(
                    "Enabled",
                    strings(bbb_server_types::save::BUILTIN_ONLY_ENABLED_PACKS),
                ),
                entry(
                    "Disabled",
                    strings(bbb_server_types::save::BUILTIN_ONLY_DISABLED_PACKS),
                ),
            ]),
        ),
        entry(
            "enabled_features",
            strings(bbb_server_types::save::BUILTIN_ONLY_ENABLED_FEATURES),
        ),
        entry(
            "Version",
            NbtSummaryValue::Compound(vec![
                entry(
                    "Name",
                    NbtSummaryValue::string(bbb_server_types::save::JAVA_26_1_NAME),
                ),
                entry("Id", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("Snapshot", NbtSummaryValue::Byte(0)),
                entry("Series", NbtSummaryValue::string(JAVA_MAIN_SERIES)),
            ]),
        ),
    ];
    Ok(NamedCompound::new(
        "",
        vec![entry("Data", NbtSummaryValue::Compound(data))],
    ))
}

/// The `data/minecraft/world_gen_settings.dat` document a new save is published
/// with. Vanilla builds the Nether and End stems once and reuses them across
/// presets (`WorldPresets.java:83-101`); retaining both whole normal-preset
/// stems keeps this document from becoming another description of them.
pub fn created_world_gen_settings_document(fixture: &FlatFixture) -> SaveResult<NamedCompound> {
    world_gen_settings_document(fixture, CreatedStemSources::PRODUCTION)
}

#[derive(Clone, Copy)]
enum CreatedStemSource {
    NormalNoise,
    FlatFixture,
}

#[derive(Clone, Copy)]
struct CreatedStemSources {
    nether: CreatedStemSource,
    end: CreatedStemSource,
}

impl CreatedStemSources {
    const PRODUCTION: Self = Self {
        nether: CreatedStemSource::NormalNoise,
        end: CreatedStemSource::NormalNoise,
    };
    const FLAT_FIXTURE: Self = Self {
        nether: CreatedStemSource::FlatFixture,
        end: CreatedStemSource::FlatFixture,
    };
}

fn world_gen_settings_document(
    fixture: &FlatFixture,
    sources: CreatedStemSources,
) -> SaveResult<NamedCompound> {
    let snapshot = VanillaDatapackSnapshot::embedded_shared()
        .map_err(|error| invalid_schema(SCHEMA, format!("built-in datapack snapshot: {error}")))?;
    let mut dimensions = preset_dimensions(&snapshot, FLAT_PRESET_PATH)?;
    let normal_dimensions = preset_dimensions(&snapshot, NORMAL_PRESET_PATH)?;
    select_created_generators(&mut dimensions, &normal_dimensions, fixture, sources)?;
    Ok(world_gen_settings(dimensions, CREATED_WORLD_SEED, false))
}

/// `WorldGenSettings` as `WorldGenSettings.CODEC` writes it, with no bonus chest.
fn world_gen_settings(
    dimensions: NbtSummaryValue,
    seed: i64,
    generate_structures: bool,
) -> NamedCompound {
    let data = vec![
        entry("seed", NbtSummaryValue::Long(seed)),
        entry(
            "generate_structures",
            NbtSummaryValue::Byte(i8::from(generate_structures)),
        ),
        entry("bonus_chest", NbtSummaryValue::Byte(0)),
        entry("dimensions", dimensions),
    ];
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("data", NbtSummaryValue::Compound(data)),
        ],
    )
}

fn preset_dimensions(
    snapshot: &VanillaDatapackSnapshot,
    path: &str,
) -> SaveResult<NbtSummaryValue> {
    let preset = snapshot
        .read_path(path)
        .ok_or_else(|| invalid_schema(SCHEMA, format!("built-in pack has no {path}")))?;
    let preset: serde_json::Value = serde_json::from_slice(preset)
        .map_err(|error| invalid_schema(SCHEMA, format!("{path}: {error}")))?;
    let dimensions = preset
        .get("dimensions")
        .ok_or_else(|| invalid_schema(SCHEMA, format!("{path} declares no dimensions")))?;
    json_to_nbt(dimensions, path)
}

/// Selects one built-in source owner per stem. Production Nether and End keep
/// Vanilla's complete normal-preset noise sources; Overworld stays on the
/// fixture source. Unknown stems refuse instead of inheriting that source.
fn select_created_generators(
    dimensions: &mut NbtSummaryValue,
    normal_dimensions: &NbtSummaryValue,
    fixture: &FlatFixture,
    sources: CreatedStemSources,
) -> SaveResult<()> {
    let flat_template = NbtSummaryValue::Compound(
        compound_path_mut(dimensions, &[OVERWORLD_STEM, "generator"])?.clone(),
    );
    let normal_nether = NbtSummaryValue::Compound(
        compound_path(normal_dimensions, &[NETHER_STEM, "generator"])?.to_vec(),
    );
    let normal_end = NbtSummaryValue::Compound(
        compound_path(normal_dimensions, &[END_STEM, "generator"])?.to_vec(),
    );
    let NbtSummaryValue::Compound(stems) = dimensions else {
        return Err(missing_preset_path(&[]));
    };
    let names = stems
        .iter()
        .map(|stem| {
            stem.name
                .try_to_string()
                .map_err(|error| invalid_schema(SCHEMA, error.to_string()))
        })
        .collect::<SaveResult<Vec<_>>>()?;
    for name in names {
        let normal_generator = match name.as_str() {
            NETHER_STEM if matches!(sources.nether, CreatedStemSource::NormalNoise) => {
                Some(&normal_nether)
            }
            END_STEM if matches!(sources.end, CreatedStemSource::NormalNoise) => Some(&normal_end),
            _ => None,
        };
        if let Some(generator) = normal_generator {
            set_field(
                expect_stem(stems, &name)?,
                "generator",
                (*generator).clone(),
            );
            continue;
        }
        if name != OVERWORLD_STEM && name != NETHER_STEM && name != END_STEM {
            return Err(invalid_schema(
                SCHEMA,
                format!("{FLAT_PRESET_PATH} declares stem {name} with no source owner"),
            ));
        }
        let dimension_type = match named_field(expect_stem(stems, &name)?, "type")? {
            NbtSummaryValue::String(value) => value
                .try_to_string()
                .map_err(|error| invalid_schema(SCHEMA, error.to_string()))?,
            _ => {
                return Err(invalid_schema(
                    SCHEMA,
                    format!("{FLAT_PRESET_PATH} stem {name} names no dimension type"),
                ))
            }
        };
        let shape = builtin_dimension_type_shape(&dimension_type)
            .map_err(|error| invalid_schema(SCHEMA, error.to_string()))?;
        let mut generator = flat_template.clone();
        let settings = compound_path_mut(&mut generator, &["settings"])?;
        set_field(
            settings,
            "layers",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("block", NbtSummaryValue::string(fixture.floor_block_name())),
                entry(
                    "height",
                    NbtSummaryValue::Int(created_floor_height(fixture, shape.min_y)?),
                ),
            ])]),
        );
        set_field(
            settings,
            "structure_overrides",
            NbtSummaryValue::List(Vec::new()),
        );
        set_field(expect_stem(stems, &name)?, "generator", generator);
    }
    Ok(())
}

fn expect_stem<'a>(
    stems: &'a mut [NbtSummaryEntry],
    name: &str,
) -> SaveResult<&'a mut Vec<NbtSummaryEntry>> {
    match stems
        .iter_mut()
        .find(|stem| stem.name == name)
        .map(|stem| &mut stem.value)
    {
        Some(NbtSummaryValue::Compound(entries)) => Ok(entries),
        _ => Err(missing_preset_path(&[name])),
    }
}

/// The flat-fixture source map used by the world's no-datapack test mode.
/// Production startup consumes [`default_world_gen_settings`] instead.
///
/// The result is memoized because building it decompresses the embedded
/// built-in datapack snapshot, and the answer cannot change within a process:
/// nothing it reads is configurable.
pub fn default_level_sources() -> SaveResult<BTreeMap<Java26LevelKey, FlatLevelSource>> {
    static DEFAULT: OnceLock<BTreeMap<Java26LevelKey, FlatLevelSource>> = OnceLock::new();
    if let Some(sources) = DEFAULT.get() {
        return Ok(sources.clone());
    }
    let sources = build_default_level_sources()?;
    Ok(DEFAULT.get_or_init(|| sources).clone())
}

/// The `dimension_type` every stem of the created world names, keyed by stem id.
///
/// The dimension type is the other half of a `LevelStem`, and it is not
/// recoverable from a [`FlatLevelSource`]: the source keeps the `min_y` and
/// `height` it was sized with and forgets which type declared them. A session's
/// `CommonPlayerSpawnInfo` needs the type itself, because what it puts on the
/// wire is that element's index in the published `minecraft:dimension_type`
/// registry (`ServerPlayer.java:2151-2164`).
///
/// Same document and same parse as [`default_level_sources`], for the same
/// reason: a server that starts with no save on disk names the same dimension
/// types as one that starts from the save it just published.
pub fn default_level_dimension_types() -> SaveResult<BTreeMap<Java26LevelKey, String>> {
    static DEFAULT: OnceLock<BTreeMap<Java26LevelKey, String>> = OnceLock::new();
    if let Some(types) = DEFAULT.get() {
        return Ok(types.clone());
    }
    let fixture = FlatFixture::new().map_err(|error| {
        invalid_schema(SCHEMA, format!("the flat fixture is unavailable: {error}"))
    })?;
    let document = created_world_gen_settings_document(&fixture)?;
    let settings = parse_java_26_1_world_gen_settings(document, 0)?;
    let types = settings
        .dimensions
        .iter()
        .map(|(name, stem)| (name.clone(), stem.dimension_type.clone()))
        .collect::<BTreeMap<_, _>>();
    Ok(DEFAULT.get_or_init(|| types).clone())
}

/// The overworld's own entry of [`default_level_sources`], which is what a
/// caller that only needs the column a save-less server publishes wants.
pub fn default_overworld_level_source() -> SaveResult<FlatLevelSource> {
    default_level_sources()?
        .remove(&Java26LevelKey::from(Java26BuiltinDimension::Overworld))
        .ok_or_else(|| invalid_schema(SCHEMA, "the created settings declare no overworld stem"))
}

fn build_default_level_sources() -> SaveResult<BTreeMap<Java26LevelKey, FlatLevelSource>> {
    let fixture = FlatFixture::new().map_err(|error| {
        invalid_schema(SCHEMA, format!("the flat fixture is unavailable: {error}"))
    })?;
    let settings = parse_java_26_1_world_gen_settings(
        world_gen_settings_document(&fixture, CreatedStemSources::FLAT_FIXTURE)?,
        0,
    )?;
    Ok(served_level_sources_from_stems(&settings.dimensions, None, SCHEMA)?.served)
}

/// The parsed settings both save-less creation and immediate reopen consume.
pub fn default_world_gen_settings() -> SaveResult<Java26WorldGenSettings> {
    let fixture = FlatFixture::new().map_err(|error| {
        invalid_schema(SCHEMA, format!("the flat fixture is unavailable: {error}"))
    })?;
    parse_java_26_1_world_gen_settings(created_world_gen_settings_document(&fixture)?, 0)
}

/// How tall the created world's one layer is in a dimension whose floor is
/// `min_y`.
///
/// Flat layers stack from the dimension's floor, so the height is the fixture's
/// own solid column: `min_y` up to and including its floor.
fn created_floor_height(fixture: &FlatFixture, min_y: i32) -> SaveResult<i32> {
    fixture
        .floor_y()
        .checked_sub(min_y)
        .and_then(|height| height.checked_add(1))
        .filter(|height| *height > 0)
        .ok_or_else(|| invalid_schema(SCHEMA, "the fixture column has no height"))
}

fn json_to_nbt(value: &serde_json::Value, path: &str) -> SaveResult<NbtSummaryValue> {
    json_value_to_nbt_summary(value)
        .map_err(|error| invalid_schema(SCHEMA, format!("{path}: {error}")))
}

fn compound_path<'a>(
    value: &'a NbtSummaryValue,
    path: &[&str],
) -> SaveResult<&'a [NbtSummaryEntry]> {
    let mut current = value;
    for (index, name) in path.iter().enumerate() {
        let NbtSummaryValue::Compound(entries) = current else {
            return Err(missing_preset_path(&path[..index]));
        };
        current = entries
            .iter()
            .find(|entry| entry.name == *name)
            .map(|entry| &entry.value)
            .ok_or_else(|| missing_preset_path(&path[..=index]))?;
    }
    match current {
        NbtSummaryValue::Compound(entries) => Ok(entries),
        _ => Err(missing_preset_path(path)),
    }
}

fn compound_path_mut<'a>(
    value: &'a mut NbtSummaryValue,
    path: &[&str],
) -> SaveResult<&'a mut Vec<NbtSummaryEntry>> {
    let mut current = value;
    for (index, name) in path.iter().enumerate() {
        let NbtSummaryValue::Compound(entries) = current else {
            return Err(missing_preset_path(&path[..index]));
        };
        current = entries
            .iter_mut()
            .find(|entry| entry.name == *name)
            .map(|entry| &mut entry.value)
            .ok_or_else(|| missing_preset_path(&path[..=index]))?;
    }
    match current {
        NbtSummaryValue::Compound(entries) => Ok(entries),
        _ => Err(missing_preset_path(path)),
    }
}

fn missing_preset_path(path: &[&str]) -> SaveError {
    invalid_schema(
        SCHEMA,
        format!("{FLAT_PRESET_PATH} has no compound at {}", path.join(".")),
    )
}

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    match entries.iter_mut().find(|entry| entry.name == name) {
        Some(existing) => existing.value = value,
        None => entries.push(entry(name, value)),
    }
}

/// Builds every document a new save is published with.
///
/// The columns are the ones the world holds, so the save is a save of the world
/// that is about to serve it rather than of a world described twice.
pub fn created_save(
    request: &Java26WorldCreation,
    fixture: &FlatFixture,
    columns: &[(i32, i32, &ChunkColumn)],
) -> SaveResult<Java26CreatedSave> {
    let level = created_level_document(request)?;
    let world_gen_settings = created_world_gen_settings_document(fixture)?;
    flat_created_save(level, world_gen_settings, fixture, columns)
}

/// Builds a new save from a `level-type` preset (`Main.createNewWorldData`,
/// `Main.java:226-259`): the preset's dimensions verbatim, published without
/// chunks and uninitialized, so its first load runs `setInitialSpawn` and the
/// generator makes the terrain. An empty `generator-settings` does not parse as
/// `FlatLevelGeneratorSettings` (`DedicatedServerProperties.java:269-276`), so
/// `flat` keeps the preset's classic superflat overworld.
pub fn created_preset_save(
    request: &Java26WorldCreation,
    options: &Java26WorldOptions,
) -> SaveResult<Java26CreatedSave> {
    let snapshot = VanillaDatapackSnapshot::embedded_shared()
        .map_err(|error| invalid_schema(SCHEMA, format!("built-in datapack snapshot: {error}")))?;
    let dimensions = preset_dimensions(&snapshot, preset_path(options.preset))?;
    Ok(Java26CreatedSave {
        level: level_document(request, false, options.hardcore)?,
        world_gen_settings: world_gen_settings(
            dimensions,
            options.seed,
            options.generate_structures,
        ),
        chunks: Vec::new(),
    })
}

fn flat_created_save(
    level: NamedCompound,
    world_gen_settings: NamedCompound,
    fixture: &FlatFixture,
    columns: &[(i32, i32, &ChunkColumn)],
) -> SaveResult<Java26CreatedSave> {
    // The biome the created world advertises is the one its own generator
    // names, read back out of the document just written rather than repeated
    // here: a chunk claiming a different biome would be a second answer to a
    // question the save already answers.
    let biome = created_overworld_biome(&world_gen_settings)?;
    let chunks = columns
        .iter()
        .map(|(chunk_x, chunk_z, column)| {
            Ok(Java26CreatedChunk {
                dimension: Java26BuiltinDimension::Overworld.into(),
                chunk_x: *chunk_x,
                chunk_z: *chunk_z,
                document: created_chunk_document(
                    column,
                    *chunk_x,
                    *chunk_z,
                    &|id| (column.biome_id() == Some(id)).then(|| biome.clone()),
                    fixture.floor_y(),
                )?,
            })
        })
        .collect::<SaveResult<Vec<_>>>()?;
    Ok(Java26CreatedSave {
        level,
        world_gen_settings,
        chunks,
    })
}

/// The biome the created overworld generates, out of its own flat settings.
fn created_overworld_biome(world_gen_settings: &NamedCompound) -> SaveResult<String> {
    let data = compound_field(world_gen_settings.entries(), "data")?;
    let dimensions = compound_field(data, "dimensions")?;
    let overworld = compound_field(dimensions, OVERWORLD_STEM)?;
    let generator = compound_field(overworld, "generator")?;
    let settings = compound_field(generator, "settings")?;
    match named_field(settings, "biome")? {
        NbtSummaryValue::String(value) => value
            .try_to_string()
            .map_err(|error| invalid_schema(SCHEMA, error.to_string())),
        _ => Err(invalid_schema(SCHEMA, "the flat settings name no biome")),
    }
}

/// One chunk of a created save, in the shape `Java26ChunkData::parse` reads.
///
/// A created chunk is a finished one: `minecraft:full` status, no proto entity
/// list, no carving mask, no structures. Light is left out on purpose — a save
/// without `isLightOn` is one every reader relights, which is cheaper to get
/// right than a light array that claims to be correct.
pub fn created_chunk_document(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    surface_y: i32,
) -> SaveResult<NamedCompound> {
    full_chunk_document(column, chunk_x, chunk_z, biome_name, surface_y)
}

fn compound_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> SaveResult<&'a [NbtSummaryEntry]> {
    match named_field(entries, name)? {
        NbtSummaryValue::Compound(entries) => Ok(entries),
        _ => Err(invalid_schema(SCHEMA, format!("{name} is not a compound"))),
    }
}

fn named_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> SaveResult<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
        .ok_or_else(|| invalid_schema(SCHEMA, format!("a created document has no {name}")))
}

fn strings(values: &[&str]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|value| NbtSummaryValue::string(*value))
            .collect(),
    )
}

#[cfg(test)]
pub mod tests;
