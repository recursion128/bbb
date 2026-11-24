//! The `SavedData` documents a whole-world read has to answer for:
//! per-profile statistics, the game-rule and filled-map state, the
//! per-dimension chunk tickets, and the scoreboard Packed document.

use crate::atomic_file::io_error;
use crate::level_file::{read_optional_bounded_file, read_optional_gzip_compound};
use crate::schema_support::{
    bool_value, compound_or_empty, field, int_array, int_value, list_or_empty, long_or, long_value,
    require_java_26_1_root,
};
use bbb_protocol::game_rules::{vanilla_game_rule, GameRuleValue, GameRuleValueKind};
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::{
    Java26LevelKey, NamedCompound, SaveError, SaveResult, WorldSaveLimits, WorldSavePaths,
    JAVA_26_1_DATA_VERSION,
};
use bbb_server_types::stats::ServerStatsCounter;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

mod command_storage;
mod custom_boss_events;
mod ender_dragon_fight;
mod maps;
mod raids;
mod scoreboard;

pub(crate) use self::command_storage::load_java_26_1_command_storage;
pub use self::command_storage::{
    command_storage_relative_path, java_26_1_command_storage_document,
    Java26CommandStorageContents, Java26CommandStorageNamespaces,
};

pub use self::custom_boss_events::{
    java_26_1_custom_boss_events_document, load_java_26_1_custom_boss_events,
    save_java_26_1_custom_boss_events, Java26CustomBossEventPacked, Java26CustomBossEventsPacked,
};

pub(crate) use self::ender_dragon_fight::ENDER_DRAGON_FIGHT_FILE;
pub use self::ender_dragon_fight::{
    java_26_1_ender_dragon_fight_document, load_java_26_1_ender_dragon_fight,
    parse_java_26_1_ender_dragon_fight_document, Java26DragonRespawnStage, Java26EnderDragonFight,
};

pub use self::raids::{
    java_26_1_raids_document, load_java_26_1_raids, Java26Raid, Java26RaidStatus, Java26Raids,
};

pub(crate) use self::maps::java_26_1_map_index_document;
pub use self::maps::{
    discover_java_26_1_maps, java_26_1_map_document, load_java_26_1_map, load_java_26_1_maps,
    Java26LoadedMaps, Java26MapBanner, Java26MapFrame, Java26MapSavedData,
};

pub use self::scoreboard::{
    canonical_objective_criteria, canonical_objective_criteria_with_registry,
    java_26_1_scoreboard_document, load_java_26_1_scoreboard,
    load_java_26_1_scoreboard_with_registry, Java26ScoreboardNumberFormat,
    Java26ScoreboardObjective, Java26ScoreboardPacked, Java26ScoreboardPackedScore,
    Java26ScoreboardTeam,
};

/// `WorldBorder.Settings.CODEC` (`WorldBorder.java:460-477`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Java26WorldBorderSettings {
    pub center_x: f64,
    pub center_z: f64,
    pub damage_per_block: f64,
    pub safe_zone: f64,
    pub warning_blocks: i32,
    pub warning_time: i32,
    pub size: f64,
    pub lerp_time: i64,
    pub lerp_target: f64,
}

impl Default for Java26WorldBorderSettings {
    fn default() -> Self {
        Self {
            center_x: 0.0,
            center_z: 0.0,
            damage_per_block: 0.2,
            safe_zone: 5.0,
            warning_blocks: 5,
            warning_time: 300,
            size: 5.999997E7_f32 as f64,
            lerp_time: 0,
            lerp_target: 0.0,
        }
    }
}

/// Loads one level's `data/minecraft/world_border.dat`; absence constructs
/// `WorldBorder.Settings.DEFAULT` (`WorldBorder.java:460-477`).
pub fn load_java_26_1_world_border<D: Into<Java26LevelKey>>(
    paths: &WorldSavePaths,
    dimension: D,
    limits: WorldSaveLimits,
) -> SaveResult<Java26WorldBorderSettings> {
    let dimension = dimension.into();
    let path = paths
        .dimension_dir(&dimension)
        .join("data")
        .join("minecraft")
        .join("world_border.dat");
    let Some((document, _)) = read_optional_gzip_compound(&path, limits)? else {
        return Ok(Java26WorldBorderSettings::default());
    };
    let data = compound_or_empty(require_java_26_1_root(&document, "world_border")?, "data");
    Ok(parse_world_border(data).unwrap_or_default())
}

// A document the codec rejects reads as `Settings.DEFAULT`; a non-finite value
// is refused the same way so the runtime never sees one.
fn parse_world_border(data: &[NbtSummaryEntry]) -> Option<Java26WorldBorderSettings> {
    let double = |name: &str| {
        let value = match field(data, name)? {
            NbtSummaryValue::Double(bits) => f64::from_bits(*bits),
            NbtSummaryValue::Float(bits) => f64::from(f32::from_bits(*bits)),
            value => long_value(value)? as f64,
        };
        value.is_finite().then_some(value)
    };
    const MAX_CENTER: f64 = 29_999_984.0;
    let center = |name: &str| double(name).filter(|value| value.abs() <= MAX_CENTER);
    let non_negative = |name: &str| double(name).filter(|&value| value >= 0.0);
    Some(Java26WorldBorderSettings {
        center_x: center("center_x")?,
        center_z: center("center_z")?,
        damage_per_block: double("damage_per_block")?,
        safe_zone: double("safe_zone")?,
        warning_blocks: field(data, "warning_blocks").and_then(int_value)?,
        warning_time: field(data, "warning_time").and_then(int_value)?,
        size: non_negative("size")?,
        lerp_time: field(data, "lerp_time").and_then(long_value)?.max(0),
        lerp_target: non_negative("lerp_target")?,
    })
}

pub fn java_26_1_world_border_document(settings: Java26WorldBorderSettings) -> NamedCompound {
    let double = |name: &str, value: f64| NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Double(value.to_bits()),
    };
    NamedCompound::new(
        "",
        vec![
            NbtSummaryEntry {
                name: "DataVersion".into(),
                value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
            },
            NbtSummaryEntry {
                name: "data".into(),
                value: NbtSummaryValue::Compound(vec![
                    double("center_x", settings.center_x),
                    double("center_z", settings.center_z),
                    double("damage_per_block", settings.damage_per_block),
                    double("safe_zone", settings.safe_zone),
                    NbtSummaryEntry {
                        name: "warning_blocks".into(),
                        value: NbtSummaryValue::Int(settings.warning_blocks),
                    },
                    NbtSummaryEntry {
                        name: "warning_time".into(),
                        value: NbtSummaryValue::Int(settings.warning_time),
                    },
                    double("size", settings.size),
                    NbtSummaryEntry {
                        name: "lerp_time".into(),
                        value: NbtSummaryValue::Long(settings.lerp_time),
                    },
                    double("lerp_target", settings.lerp_target),
                ]),
            },
        ],
    )
}

/// Every `players/stats/<uuid>.json` the save holds, which is
/// `PlayerList.getPlayerStats` (`PlayerList.java:757-763`) run eagerly over the
/// whole directory.
///
/// It is the directory rather than the playerdata uuids, because the two sets
/// are not the same one: `PlayerList.remove` writes a leaver's statistics file
/// (`:302`) whether or not anything ever wrote it a playerdata document, and a
/// profile whose counts were read off the wrong set comes back on an empty
/// counter and then overwrites its own file with it.
///
/// A profile with no file is absent from the map rather than present and empty,
/// so the join can tell "never played" from "played and counted nothing"; both
/// produce the same empty counter, which is what
/// `new ServerStatsCounter(server, file)` leaves behind for a file that is not
/// there (`ServerStatsCounter.java:68`). A file that is not JSON at all is an
/// error rather than vanilla's empty counter (`:72-76`), so this server never
/// overwrites a statistics file it could not read.
/// Discovers the profile uuids present in `players/stats/<uuid>.json` without
/// reading any file bodies.
pub fn discover_java_26_1_player_stats(paths: &WorldSavePaths) -> SaveResult<BTreeSet<Uuid>> {
    let dir = paths.player_stats_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => return Err(io_error("listing player statistics", &dir, error)),
    };
    let mut uuids = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| io_error("listing player statistics", &dir, error))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| io_error("reading player statistics entry type", &path, error))?;
        if file_type.is_symlink() {
            return Err(SaveError::InvalidPath {
                path,
                detail: "managed player statistics directory must not contain symbolic links",
            });
        }
        // `<name>.json` is `locateStatsFile`'s legacy fallback
        // (`PlayerList.java:765-786`), which this reader does not model; a file
        // that is not a canonical uuid document is left alone rather than guessed at.
        let Some(uuid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|stem| {
                let uuid = Uuid::parse_str(stem).ok()?;
                (uuid.hyphenated().to_string() == stem).then_some(uuid)
            })
        else {
            continue;
        };
        if !file_type.is_file() {
            return Err(SaveError::InvalidPath {
                path,
                detail: "canonical player statistics path must be a regular file",
            });
        }
        uuids.insert(uuid);
    }
    Ok(uuids)
}

/// Reads and parses a single `players/stats/<uuid>.json` document on demand
/// using default save budgets.
pub fn load_java_26_1_player_stat(
    paths: &WorldSavePaths,
    uuid: Uuid,
) -> SaveResult<Option<ServerStatsCounter>> {
    load_java_26_1_player_stat_with_limits(paths, WorldSaveLimits::default(), uuid)
}

/// Reads and parses a single `players/stats/<uuid>.json` document on demand
/// under `limits`.
pub fn load_java_26_1_player_stat_with_limits(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    uuid: Uuid,
) -> SaveResult<Option<ServerStatsCounter>> {
    load_java_26_1_player_stat_with_limits_and_registry(
        paths,
        limits,
        uuid,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

/// `ContentRegistry`-aware variant of [`load_java_26_1_player_stat_with_limits`].
pub fn load_java_26_1_player_stat_with_limits_and_registry(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    uuid: Uuid,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> SaveResult<Option<ServerStatsCounter>> {
    let path = paths.player_stats_file(uuid);
    let Some(bytes) = read_optional_bounded_file(
        &path,
        "reading player statistics",
        "player statistics JSON",
        limits.max_decompressed_nbt_bytes,
    )?
    else {
        return Ok(None);
    };
    let counter =
        ServerStatsCounter::parse_bytes_with_registry(&bytes, registry).map_err(|error| {
            SaveError::InvalidNbt {
                path: Some(path.clone()),
                detail: error.to_string(),
            }
        })?;
    Ok(Some(counter))
}

pub fn load_java_26_1_player_stats(
    paths: &WorldSavePaths,
) -> SaveResult<BTreeMap<Uuid, ServerStatsCounter>> {
    let uuids = discover_java_26_1_player_stats(paths)?;
    let mut loaded = BTreeMap::new();
    for uuid in uuids {
        if let Some(counter) = load_java_26_1_player_stat(paths, uuid)? {
            loaded.insert(uuid, counter);
        }
    }
    Ok(loaded)
}

/// `GameRuleMap.TYPE` (`GameRuleMap.java:21-23`) as
/// `LevelStorageSource.writeSavedData` lays it out
/// (`LevelStorageSource.java:195-202`): `data/minecraft/game_rules.dat`,
/// gzipped, `{ DataVersion, data: { <rule id>: <value> } }`.
///
/// `GameRuleMap.CODEC` is
/// `Codec.dispatchedMap(BuiltInRegistries.GAME_RULE.byNameCodec(), GameRule::valueCodec)`
/// (`GameRuleMap.java:19-20`), so the keys are registry ids and each value is
/// the rule's own codec — `Codec.BOOL`, which `NbtOps` writes as a byte
/// (`GameRules.java:173`), or `Codec.intRange` (`:197`).
///
/// The file is absent from a save that never wrote one, and a rule the file
/// leaves out keeps its registered default. An entry the codec cannot decode --
/// an unknown rule, a value of the wrong type or outside the rule's range -- is
/// dropped (`SavedDataStorage.java:92-95`, `resultOrPartial`).
pub fn load_java_26_1_game_rules(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<BTreeMap<String, GameRuleValue>> {
    let path = paths.game_rules_dat();
    let Some((document, _)) = read_optional_gzip_compound(&path, limits)? else {
        return Ok(BTreeMap::new());
    };
    let data = compound_or_empty(require_java_26_1_root(&document, "game_rules")?, "data");
    let mut rules = BTreeMap::new();
    for entry in data {
        let Some(rule) = entry
            .name
            .try_to_string()
            .ok()
            .and_then(|name| normalize_resource_location(name).ok())
            .and_then(|id| vanilla_game_rule(&id))
        else {
            continue;
        };
        let value = match rule.value_kind {
            GameRuleValueKind::Boolean => bool_value(&entry.value).map(GameRuleValue::Boolean),
            GameRuleValueKind::Integer { .. } => {
                int_value(&entry.value).map(GameRuleValue::Integer)
            }
        };
        if let Some(value) = value.filter(|value| rule.accepts(*value)) {
            rules.insert(rule.resource_id.to_string(), value);
        }
    }
    Ok(rules)
}

/// The `data/minecraft/game_rules.dat` document one save is written with.
///
/// The inverse of [`load_java_26_1_game_rules`], and deliberately the whole
/// map rather than the entries that moved: `SavedDataStorage.encodeUnchecked`
/// (`SavedDataStorage.java:198-204`) runs `GameRuleMap.CODEC`
/// (`GameRuleMap.java:19-20`) over the level's own map and puts the result
/// under `data` beside `NbtUtils.addCurrentDataVersion`. That map holds every
/// enabled rule, at its default or otherwise, because the `GameRules`
/// constructor `reset`s each rule the file left out and `remove`s each rule the
/// feature set disabled (`GameRules.java:99-110`), and the server hands it
/// exactly that map (`MinecraftServer.java:345`).
pub fn java_26_1_game_rules_document(rules: &BTreeMap<String, GameRuleValue>) -> NamedCompound {
    // `NbtOps` writes `Codec.BOOL` as a byte (`GameRules.java:173`) and
    // `Codec.intRange` as an int (`:197`).
    let entries = rules
        .iter()
        .map(|(key, value)| NbtSummaryEntry {
            name: key.as_str().into(),
            value: match value {
                GameRuleValue::Boolean(value) => NbtSummaryValue::Byte(i8::from(*value)),
                GameRuleValue::Integer(value) => NbtSummaryValue::Int(*value),
            },
        })
        .collect();
    NamedCompound::new(
        "",
        vec![
            NbtSummaryEntry {
                name: "DataVersion".into(),
                value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
            },
            NbtSummaryEntry {
                name: "data".into(),
                value: NbtSummaryValue::Compound(entries),
            },
        ],
    )
}

/// One persisted `Ticket` together with the column it was filed against.
///
/// `TicketStorage.TICKET_ENTRY` (`TicketStorage.java:36`) is
/// `Codec.mapPair(ChunkPos.CODEC.fieldOf("chunk_pos"), Ticket.CODEC)`, and
/// `Ticket.CODEC` (`Ticket.java:11-18`) is `{ type, level, ticks_left }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Java26SavedChunkTicket {
    pub chunk_x: i32,
    pub chunk_z: i32,
    /// `Ticket.getTicketLevel`, written through `ExtraCodecs.NON_NEGATIVE_INT`
    /// (`Ticket.java:14`).
    pub level: i32,
    /// `Ticket.ticksLeft` (`Ticket.java:21`) when the type has a timeout, and
    /// `None` when it does not -- `TicketType.hasTimeout` is `timeout != 0`
    /// (`TicketType.java:51-53`), and `Ticket.decreaseTicksLeft` (`:60-64`)
    /// and `isTimedOut` (`:66-68`) both answer with it.
    ///
    /// `Ticket.CODEC` defaults the field to `0L` rather than to the type's
    /// timeout (`Ticket.java:15`), so a timed ticket the save wrote without one
    /// is already spent: the next `purgeStaleTickets` takes it to `-1`, which
    /// `isTimedOut` answers true for.
    pub ticks_left: Option<i64>,
}

/// The ticket types a 26.1 save can carry, with the `TicketType.timeout` each
/// was registered with.
///
/// `TicketStorage.packTickets` (`TicketStorage.java:71-79`) writes only tickets
/// whose type answers `persist()`, and in the 26.1 registry
/// (`TicketType.java:17-25`) exactly two do: `FORCED`
/// (`register("forced", 0L, 15)`, `:22`) and `PORTAL`
/// (`register("portal", 300L, 15)`, `:23`). Flags 15 is
/// `FLAG_PERSIST | FLAG_LOADING | FLAG_SIMULATION | FLAG_KEEP_DIMENSION_ACTIVE`
/// (`TicketType.java:12-15`), so both load and both simulate; the only
/// difference is that `PORTAL` expires, which is why the timeout travels with
/// the name.
///
/// A `PORTAL` ticket reaches a save whenever the server stops within 300 ticks
/// of somebody stepping through a portal, so refusing one would refuse a save
/// vanilla opens without complaint.
const JAVA_26_1_PERSISTED_TICKET_TYPES: [(&str, i64); 2] =
    [("minecraft:forced", 0), ("minecraft:portal", 300)];

/// The widest column a persisted ticket can name.
///
/// `ForceLoadCommand.changeForceLoad` refuses a range whose block coordinates
/// leave `-30000000 ..= 29999999` (`ForceLoadCommand.java:135`) before it turns
/// them into columns with `SectionPos.blockToSectionCoord` (`:136-139`), an
/// arithmetic shift by four; `TicketType.PORTAL` is filed from an entity
/// position, which `Level.MAX_LEVEL_SIZE` bounds the same way. So no ticket a
/// 26.1 server writes ever leaves `-1875000 ..= 1874999`.
///
/// Dropping anything wider is what keeps the ticket-level propagation's column
/// arithmetic inside `i32`: it walks up to 32 columns out from every source, so
/// a coordinate near `i32::MAX` would be a save that decides between a debug
/// panic and a wrapped column.
pub const JAVA_26_1_TICKET_CHUNK_LIMIT: std::ops::RangeInclusive<i32> = -1_875_000..=1_874_999;

/// Authors the document owned by `TicketStorage.CODEC`.
///
/// `TicketStorage.packTickets` persists only `FORCED` and `PORTAL`
/// (`TicketStorage.java:31-84`); `Ticket.CODEC` writes their type, non-negative
/// level, and optional `ticks_left` (`Ticket.java:11-18`).
pub fn java_26_1_chunk_tickets_document(tickets: &[Java26SavedChunkTicket]) -> NamedCompound {
    let mut packed = Vec::with_capacity(tickets.len());
    for ticket in tickets {
        let mut fields = vec![
            NbtSummaryEntry {
                name: "chunk_pos".into(),
                value: NbtSummaryValue::IntArray(vec![ticket.chunk_x, ticket.chunk_z]),
            },
            NbtSummaryEntry {
                name: "type".into(),
                value: NbtSummaryValue::string(if ticket.ticks_left.is_some() {
                    "minecraft:portal"
                } else {
                    "minecraft:forced"
                }),
            },
            NbtSummaryEntry {
                name: "level".into(),
                value: NbtSummaryValue::Int(ticket.level),
            },
        ];
        if let Some(ticks_left) = ticket.ticks_left.filter(|ticks_left| *ticks_left != 0) {
            fields.push(NbtSummaryEntry {
                name: "ticks_left".into(),
                value: NbtSummaryValue::Long(ticks_left),
            });
        }
        packed.push(NbtSummaryValue::Compound(fields));
    }
    NamedCompound::new(
        "",
        vec![
            NbtSummaryEntry {
                name: "DataVersion".into(),
                value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
            },
            NbtSummaryEntry {
                name: "data".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "tickets".into(),
                    value: NbtSummaryValue::List(packed),
                }]),
            },
        ],
    )
}

/// `TicketStorage.TYPE` (`TicketStorage.java:40-42`) as
/// `SavedDataStorage.getDataFile` (`SavedDataStorage.java:56-63`) lays it out
/// under the *dimension* data folder `ServerChunkCache` builds
/// (`ServerChunkCache.java:94`, `:102-103`):
/// `dimensions/<ns>/<path>/data/minecraft/chunk_tickets.dat`, gzipped,
/// `{ DataVersion, data: { tickets: [ { chunk_pos, level, type } ] } }`.
///
/// Every ticket in the file is a simulation source: it was written by
/// `packTickets`, which keeps only `persist()` types, and both persisted types
/// carry `FLAG_SIMULATION` (see [`JAVA_26_1_PERSISTED_TICKET_TYPES`]). Both the
/// active and the deactivated table are written (`TicketStorage.java:81-84`)
/// and `MinecraftServer.prepareLevels` reactivates the deactivated ones before
/// the first tick (`MinecraftServer.java:543-553`), so the file is a flat list
/// of live tickets either way.
///
/// A save that never wrote the file has no tickets rather than an error
/// (`SavedDataStorage.readSavedData` answers null and `computeIfAbsent` builds
/// an empty `TicketStorage`). A malformed entry, a type no `packTickets` could
/// have written, or a column outside [`JAVA_26_1_TICKET_CHUNK_LIMIT`] is
/// dropped.
pub fn load_java_26_1_chunk_tickets<D: Into<Java26LevelKey>>(
    paths: &WorldSavePaths,
    dimension: D,
    limits: WorldSaveLimits,
) -> SaveResult<Vec<Java26SavedChunkTicket>> {
    let dimension = dimension.into();
    let path = paths
        .dimension_dir(&dimension)
        .join("data")
        .join("minecraft")
        .join("chunk_tickets.dat");
    let Some((document, _)) = read_optional_gzip_compound(&path, limits)? else {
        return Ok(Vec::new());
    };
    let data = compound_or_empty(require_java_26_1_root(&document, "chunk_tickets")?, "data");
    Ok(list_or_empty(data, "tickets")
        .iter()
        .filter_map(parse_chunk_ticket)
        .collect())
}

fn parse_chunk_ticket(value: &NbtSummaryValue) -> Option<Java26SavedChunkTicket> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let Some(NbtSummaryValue::String(ticket_type)) = field(fields, "type") else {
        return None;
    };
    let resource_id = ticket_type
        .try_to_string()
        .ok()
        .and_then(|raw| normalize_resource_location(raw).ok())?;
    let (_, timeout) = JAVA_26_1_PERSISTED_TICKET_TYPES
        .iter()
        .copied()
        .find(|(name, _)| *name == resource_id)?;
    let level = field(fields, "level")
        .and_then(int_value)
        .filter(|level| *level >= 0)?;
    // `Codec.LONG.optionalFieldOf("ticks_left", 0L)` (`Ticket.java:15`), kept
    // only for a type that reads it (`TicketType.hasTimeout`).
    let ticks_left = (timeout != 0).then(|| long_or(fields, "ticks_left", 0));
    let [chunk_x, chunk_z] = field(fields, "chunk_pos").and_then(int_array)?;
    if !JAVA_26_1_TICKET_CHUNK_LIMIT.contains(&chunk_x)
        || !JAVA_26_1_TICKET_CHUNK_LIMIT.contains(&chunk_z)
    {
        return None;
    }
    Some(Java26SavedChunkTicket {
        chunk_x,
        chunk_z,
        level,
        ticks_left,
    })
}
