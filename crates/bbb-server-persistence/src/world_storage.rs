//! Whole-world Java 26.1 discovery and validation before canonical publication.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
};

use uuid::Uuid;

#[cfg(test)]
use bbb_protocol::NbtSummaryValue;
use bbb_protocol::{content_registry::ContentRegistry, game_rules::GameRuleValue};

use bbb_server_types::random_sequences_saved::RandomSequencesSaved;
use bbb_server_types::stats::ServerStatsCounter;

use self::saved_data::{
    command_storage_relative_path, java_26_1_command_storage_document,
    java_26_1_ender_dragon_fight_document, java_26_1_raids_document, java_26_1_scoreboard_document,
    load_java_26_1_command_storage, load_java_26_1_ender_dragon_fight, load_java_26_1_raids,
    load_java_26_1_scoreboard_with_registry, save_java_26_1_custom_boss_events,
    Java26CommandStorageContents, Java26CommandStorageNamespaces, Java26CustomBossEventsPacked,
    Java26EnderDragonFight, Java26Raids, Java26ScoreboardPacked, ENDER_DRAGON_FIGHT_FILE,
};
use super::level_file::{encode_gzip_compound, encode_gzip_owned_compound};
use super::player_advancements::{
    discover_java_26_1_player_advancements, Java26PlayerAdvancements,
};
use super::player_schema::{created_player_document, patch_loaded_player_document};
use super::region_file::{prepare_chunks_parallel, PreparedRegionChunk};
use crate::scheduled_events_schema::{
    java_26_1_scheduled_events_document, load_java_26_1_scheduled_events, Java26ScheduledEvent,
};
use crate::stopwatches_schema::{
    java_26_1_stopwatches_document, load_java_26_1_stopwatches, stopwatches_relative_path,
    Java26Stopwatches,
};

use super::atomic_file::write_atomic;
use super::random_sequences_schema::{
    java_26_1_random_sequences_document, load_java_26_1_random_sequences,
};
use super::wandering_trader_schema::{
    java_26_1_wandering_trader_document, load_java_26_1_wandering_trader, wandering_trader_dat,
    Java26WanderingTraderData,
};
use super::weather_schema::{
    java_26_1_weather_document, load_java_26_1_weather, Java26WeatherData,
};
use super::world_clocks_schema::{
    java_26_1_world_clocks_document, load_java_26_1_world_clocks, Java26WorldClocks,
};
use super::world_gen_settings_schema::load_java_26_1_world_gen_settings;
use super::{
    Java26ChunkContext, Java26ChunkData, Java26CreatedChunk, Java26CreatedSave, Java26DataPacks,
    Java26EntityChunkData, Java26LevelData, Java26LevelSaveState, Java26PlayerData,
    Java26PlayerSavePatch, Java26PoiChunkData, Java26WorldGenSettings, LevelDatStore,
    PlayerDataSource, PlayerDataStore, RegionCompression, RegionFileStore, WorldSessionLock,
};
use crate::schema_support::require_java_26_1_root;
use bbb_server_types::save::{
    Java26LevelKey, NamedCompound, SaveError, SaveResult, WorldSaveLimits, WorldSavePaths,
};

pub mod dimensions;
mod poi_save;
pub mod region_discovery;
mod runtime_columns;
pub mod save_guards;
pub use poi_save::Java26PoiChunkSaveEdits;
pub use runtime_columns::{Java26ColumnReader, Java26RuntimeColumn};
pub mod saved_data;

#[cfg(test)]
pub mod entity_publication_fixtures;
#[cfg(test)]
mod runtime_unload_tests;
#[cfg(test)]
pub mod world_save_tests;

pub use self::saved_data::{Java26MapSavedData, Java26SavedChunkTicket, Java26WorldBorderSettings};
pub use bbb_server_types::save::Java26BuiltinDimension;

use self::dimensions::Java26DimensionGeometries;
use self::region_discovery::index_dimension_with_cache;
use self::save_guards::{
    reject_global_managed_symlinks, reject_symlink_root, validate_dimension_storage,
};
use self::saved_data::{
    discover_java_26_1_maps, discover_java_26_1_player_stats, java_26_1_chunk_tickets_document,
    java_26_1_game_rules_document, java_26_1_map_document, java_26_1_map_index_document,
    java_26_1_world_border_document, load_java_26_1_chunk_tickets, load_java_26_1_game_rules,
    load_java_26_1_world_border,
};
use crate::region_file::RegionFileCache;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26LoadedChunk {
    pub timestamp: u32,
    pub data: Java26ChunkData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26LoadedEntityChunk {
    pub timestamp: u32,
    pub data: Java26EntityChunkData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26LoadedPoiChunk {
    pub timestamp: u32,
    pub data: Java26PoiChunkData,
}

/// One level's saved chunks as the world opens them.
///
/// An opened save lists its chunks from the region headers (`stored_*`) and
/// leaves the bodies on disk until a column is needed, the way `ChunkMap`
/// reads a chunk when a ticket first reaches it. The record maps hold records
/// that are already in memory, which only a hand-built candidate has.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26LoadedDimension {
    pub dimension: Java26LevelKey,
    pub chunks: BTreeMap<(i32, i32), Java26LoadedChunk>,
    pub entity_chunks: BTreeMap<(i32, i32), Java26LoadedEntityChunk>,
    pub poi_chunks: BTreeMap<(i32, i32), Java26LoadedPoiChunk>,
    /// Chunks the `region/` headers list, not read yet.
    pub stored_chunks: BTreeSet<(i32, i32)>,
    /// Chunks the `entities/` headers list, not read yet.
    pub stored_entity_chunks: BTreeSet<(i32, i32)>,
    /// Chunks the `poi/` headers list, not read yet.
    pub stored_poi_chunks: BTreeSet<(i32, i32)>,
}

impl Java26LoadedDimension {
    pub fn new(dimension: Java26LevelKey) -> Self {
        Self {
            dimension,
            chunks: BTreeMap::new(),
            entity_chunks: BTreeMap::new(),
            poi_chunks: BTreeMap::new(),
            stored_chunks: BTreeSet::new(),
            stored_entity_chunks: BTreeSet::new(),
            stored_poi_chunks: BTreeSet::new(),
        }
    }
}

/// The shallow, registry-independent half of opening an existing world.
#[derive(Debug)]
pub struct Java26WorldBootstrap {
    loaded_level: super::LoadedJava26LevelData,
    world_gen_settings: Java26WorldGenSettings,
}

impl Java26WorldBootstrap {
    pub fn level(&self) -> &Java26LevelData {
        &self.loaded_level.data
    }

    pub fn world_gen_settings(&self) -> &Java26WorldGenSettings {
        &self.world_gen_settings
    }
}

fn require_compatibility_data_packs(level: &Java26LevelData) -> SaveResult<()> {
    let compatible = level.data_packs().is_none_or(|packs| {
        packs.enabled().len() == 1 && packs.enabled().first().is_some_and(|id| id == "vanilla")
    });
    if compatible {
        return Ok(());
    }
    Err(SaveError::InvalidNbt {
        path: Some(PathBuf::from("level.dat")),
        detail: "active non-Vanilla datapacks require load_candidate_from_bootstrap with the live registry generation".to_string(),
    })
}

fn missing_geometry(dimension: &Java26LevelKey) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("{} has no bound section geometry", dimension.resource_id()),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26WorldCandidate {
    pub level: Java26LevelData,
    pub world_gen_settings: Java26WorldGenSettings,
    pub dimension_geometries: Java26DimensionGeometries,
    pub dimensions: BTreeMap<Java26LevelKey, Java26LoadedDimension>,
    /// Where stored chunks, offline player files and maps are read from on
    /// demand; a hand-built candidate has none.
    pub column_reader: Option<Java26ColumnReader>,
    /// Playerdata records already in memory (typically hand-built candidates).
    pub players: BTreeMap<Uuid, (PlayerDataSource, Java26PlayerData)>,
    /// Discovered `players/data/<uuid>.dat[_old]` profiles left on disk until
    /// needed (on player join or query).
    pub stored_players: BTreeSet<Uuid>,
    /// `PlayerList.stats` (`PlayerList.java:121`) already in memory.
    pub player_stats: BTreeMap<Uuid, ServerStatsCounter>,
    /// Discovered `players/stats/<uuid>.json` profiles left on disk until
    /// needed (`PlayerList.getPlayerStats`, `PlayerList.java:757-763`).
    pub stored_player_stats: BTreeSet<Uuid>,
    /// `PlayerList.advancements` (`PlayerList.java:122`) already in memory.
    pub player_advancements: BTreeMap<Uuid, Java26PlayerAdvancements>,
    /// Discovered `players/advancements/<uuid>.json` profiles left on disk
    /// until needed.
    pub stored_player_advancements: BTreeSet<Uuid>,
    /// `GameRuleMap` as `data/minecraft/game_rules.dat` holds it, see
    /// [`load_java_26_1_game_rules`]. A rule the file leaves out is at its
    /// registered default.
    pub game_rules: BTreeMap<String, GameRuleValue>,
    /// Every live `minecraft:world_clock` holder, ordered by wire id.
    pub world_clocks: Java26WorldClocks,
    /// `data/minecraft/maps/<signed id>.dat` documents already in memory.
    pub maps: BTreeMap<i32, Java26MapSavedData>,
    /// Discovered `data/minecraft/maps/<signed id>.dat` ids left on disk until
    /// looked up.
    pub stored_maps: BTreeSet<i32>,
    pub map_last_id: i32,
    /// `TicketStorage` per dimension, see [`load_java_26_1_chunk_tickets`].
    /// These are the simulation sources the save itself declares, and the only
    /// ones a server with nobody online has.
    pub chunk_tickets: BTreeMap<Java26LevelKey, Vec<Java26SavedChunkTicket>>,
    /// `WorldBorder.TYPE` is one `SavedData` in each level's data storage.
    pub world_borders: BTreeMap<Java26LevelKey, Java26WorldBorderSettings>,
    pub raids: BTreeMap<Java26LevelKey, Java26Raids>,
    /// The dimensions whose data storage holds an `ender_dragon_fight.dat`.
    pub ender_dragon_fights: BTreeMap<Java26LevelKey, Java26EnderDragonFight>,
    /// `RandomSequences` as `data/minecraft/random_sequences.dat` holds it, see
    /// [`load_java_26_1_random_sequences`]. One map for the whole save, because
    /// `MinecraftServer.randomSequences` (`MinecraftServer.java:274`) is one
    /// `SavedData` in the level data folder rather than one per dimension.
    pub random_sequences: RandomSequencesSaved,
    pub scheduled_events: Vec<Java26ScheduledEvent>,
    pub stopwatches: Java26Stopwatches,
    /// `WeatherData` as `data/minecraft/weather.dat` holds it. One document for
    /// the whole save for the same reason the sequences above are:
    /// `MinecraftServer.weatherData` (`MinecraftServer.java:275`, `:344`) is one
    /// `SavedData` in the level data folder, and `ServerLevel.getWeatherData`
    /// returns the server's (`ServerLevel.java:1682-1683`).
    pub weather: Java26WeatherData,
    pub wandering_trader: Option<Java26WanderingTraderData>,
    /// `ScoreboardSaveData` as `data/minecraft/scoreboard.dat` holds it. One
    /// document for the whole save again: `MinecraftServer.scoreboard`
    /// (`MinecraftServer.java:270`) is one `Scoreboard` and
    /// `ServerLevel.getScoreboard` answers with the server's
    /// (`ServerLevel.java:660-662`).
    pub scoreboard: Java26ScoreboardPacked,
    /// `CustomBossEvents` as `data/minecraft/custom_boss_events.dat` holds it.
    pub custom_boss_events: Java26CustomBossEventsPacked,
    /// Every prevalidated `<world>/data/<namespace>/command_storage.dat`.
    /// The runtime separately records which namespaces command execution has
    /// touched, preserving `CommandStorage.keys`' lazy visibility.
    pub command_storage: Java26CommandStorageNamespaces,
}

impl Java26WorldCandidate {
    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        self.column_reader
            .as_ref()
            .map(Java26ColumnReader::content_registry)
            .unwrap_or_else(|| ContentRegistry::vanilla())
    }

    pub fn has_player(&self, uuid: Uuid) -> bool {
        self.players.contains_key(&uuid) || self.stored_players.contains(&uuid)
    }

    pub fn has_player_stats(&self, uuid: Uuid) -> bool {
        self.player_stats.contains_key(&uuid) || self.stored_player_stats.contains(&uuid)
    }

    pub fn has_player_advancements(&self, uuid: Uuid) -> bool {
        self.player_advancements.contains_key(&uuid)
            || self.stored_player_advancements.contains(&uuid)
    }

    pub fn has_map(&self, id: i32) -> bool {
        self.maps.contains_key(&id) || self.stored_maps.contains(&id)
    }

    pub fn load_player(
        &self,
        uuid: Uuid,
    ) -> SaveResult<Option<(PlayerDataSource, Java26PlayerData)>> {
        if let Some(player) = self.players.get(&uuid) {
            return Ok(Some(player.clone()));
        }
        if !self.stored_players.contains(&uuid) {
            return Ok(None);
        }
        let Some(reader) = self.column_reader.as_ref() else {
            return Ok(None);
        };
        reader.read_player(uuid)
    }

    pub fn load_player_stats(&self, uuid: Uuid) -> SaveResult<Option<ServerStatsCounter>> {
        if let Some(stats) = self.player_stats.get(&uuid) {
            return Ok(Some(stats.clone()));
        }
        if !self.stored_player_stats.contains(&uuid) {
            return Ok(None);
        }
        let Some(reader) = self.column_reader.as_ref() else {
            return Ok(None);
        };
        reader.read_player_stats(uuid)
    }

    pub fn load_player_advancements(
        &self,
        uuid: Uuid,
    ) -> SaveResult<Option<Java26PlayerAdvancements>> {
        if let Some(advancements) = self.player_advancements.get(&uuid) {
            return Ok(Some(advancements.clone()));
        }
        if !self.stored_player_advancements.contains(&uuid) {
            return Ok(None);
        }
        let Some(reader) = self.column_reader.as_ref() else {
            return Ok(None);
        };
        reader.read_player_advancements(uuid)
    }

    pub fn load_map(&self, id: i32) -> SaveResult<Option<Java26MapSavedData>> {
        if let Some(map) = self.maps.get(&id) {
            return Ok(Some(map.clone()));
        }
        if !self.stored_maps.contains(&id) {
            return Ok(None);
        }
        let Some(reader) = self.column_reader.as_ref() else {
            return Ok(None);
        };
        reader.read_map(id)
    }
}

/// Live dirty epochs captured when a chunk's whole-column save document is built.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Java26ChunkWriteEpochs {
    pub dirty_epoch: Option<u64>,
    pub whole_rewrite_epoch: Option<u64>,
    pub generation_status: Option<crate::Java26ChunkStatus>,
}

/// One entity-region chunk as the live world would write it.
///
/// `EntityStorage.storeEntities` writes the whole chunk every time, so this is
/// a document rather than a patch; a chunk whose entities all left is written
/// with an empty `Entities` list, which is what removes them from the save.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26EntityChunkWrite {
    pub dimension: Java26LevelKey,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub document: NamedCompound,
}

/// The registry publication [`Java26SaveRequest::player_patches`] inventories
/// resolve their holder ids against. It travels with the request because the
/// frozen ids mean nothing without the generation that issued them.
#[derive(Clone)]
pub struct SaveHolderResolver(
    Arc<dyn bbb_registry_codec::PersistentItemStackHolderResolver + Send + Sync>,
);

impl SaveHolderResolver {
    pub fn new(
        resolver: Arc<dyn bbb_registry_codec::PersistentItemStackHolderResolver + Send + Sync>,
    ) -> Self {
        Self(resolver)
    }

    /// A resolver that names no publication. Every holder lookup fails closed,
    /// so it only serves requests whose stacks reference no dynamic holder.
    pub fn unavailable() -> Self {
        struct UnavailableSaveHolders;
        impl bbb_protocol::PersistentRegistryResolver for UnavailableSaveHolders {
            fn registry_available(&self, _registry_id: &str) -> bool {
                false
            }

            fn resource_membership(
                &self,
                _registry_id: &str,
                _resource_id: &str,
            ) -> bbb_protocol::PersistentRegistryMembership {
                bbb_protocol::PersistentRegistryMembership::Unavailable
            }

            fn tag_members(
                &self,
                _registry_id: &str,
                _tag_id: &str,
            ) -> bbb_protocol::PersistentRegistryTagMembers {
                bbb_protocol::PersistentRegistryTagMembers::Unavailable
            }
        }
        impl bbb_registry_codec::PersistentItemStackHolderResolver for UnavailableSaveHolders {
            fn holder_id(&self, _registry_id: &str, _resource_id: &str) -> Option<i32> {
                None
            }
        }
        Self(Arc::new(UnavailableSaveHolders))
    }

    pub fn as_dyn(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self.0.as_ref()
    }
}

impl std::fmt::Debug for SaveHolderResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SaveHolderResolver")
    }
}

/// The resolver is not part of the request's data identity.
impl PartialEq for SaveHolderResolver {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// A tick-barrier-frozen save snapshot derived from a loaded candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26SaveRequest {
    /// The `level.dat` `Data` fields the running world owns, as
    /// [`Java26LevelData::raw_with_saved_state`] writes them back.
    pub level_state: Java26LevelSaveState,
    /// Updated `Data.DataPacks` lists (`Enabled`/`Disabled`), or `None` to keep
    /// what `level.dat` already holds.
    pub data_packs: Option<Java26DataPacks>,
    /// Region timestamp written for edited chunks.
    pub region_timestamp: u32,
    /// Compression used for edited chunks (Vanilla default `deflate` = zlib).
    pub region_compression: RegionCompression,
    pub player_patches: Vec<Java26PlayerSavePatch>,
    /// The statistics counters of profiles that are *not* online any more,
    /// keyed by profile uuid.
    ///
    /// Vanilla writes one of these the moment a session leaves: `PlayerList.remove`
    /// counts `Stats.LEAVE_GAME` (`PlayerList.java:301`) and immediately calls
    /// `this.save(player)` (`:302`), whose stats half is `ServerStatsCounter.save`
    /// (`:288-291`), and only then drops the counter out of `PlayerList.stats`
    /// (`:326`). This server owns no file handle, so the flush cannot happen
    /// where vanilla's does; the counter travels here instead and reaches the
    /// same file at the next save. The bytes match vanilla's; the moment they
    /// are written does not, so a crash between a disconnect and the next save
    /// loses what vanilla would already have on disk.
    pub departed_player_stats: BTreeMap<Uuid, ServerStatsCounter>,
    /// The advancement progress of every profile the save has to hold one for,
    /// online or departed.
    ///
    /// `PlayerList.save` writes the statistics and the advancements of one
    /// player together (`PlayerList.java:286-297`), so this travels the same
    /// way [`Self::departed_player_stats`] does and reaches the file at the
    /// same save. One map rather than two, because a session's store is read
    /// off the world's own player state either way.
    pub player_advancements: BTreeMap<Uuid, Java26PlayerAdvancements>,
    pub poi_edits: Vec<Java26PoiChunkSaveEdits>,
    /// Whole chunk documents for every dirty or newly created column, matching
    /// `ChunkStorage.write` (`SerializableChunkData.java:334-476`).
    pub chunk_writes: Vec<Java26CreatedChunk>,
    /// Live dirty epochs associated with entries in [`Self::chunk_writes`].
    pub chunk_write_epochs: BTreeMap<(Java26LevelKey, i32, i32), Java26ChunkWriteEpochs>,
    /// Whole entity-region chunk documents for every chunk whose entity set the
    /// world owns.
    pub entity_writes: Vec<Java26EntityChunkWrite>,
    /// Every game rule the world's feature set enables, with its live value.
    ///
    /// The whole map rather than the rules that moved: vanilla writes
    /// `GameRuleMap.CODEC` over the level's own map
    /// (`SavedDataStorage.encodeUnchecked`, `SavedDataStorage.java:198-204`),
    /// and that map holds every enabled rule because the `GameRules`
    /// constructor `reset`s the ones the file left out
    /// (`GameRules.java:99-110`, `MinecraftServer.java:345`).
    pub game_rules: BTreeMap<String, GameRuleValue>,
    /// Every live world clock, frozen in the loaded registry's holder order.
    pub world_clocks: Java26WorldClocks,
    /// Only maps whose live `SavedData` dirty bit was set at the tick barrier.
    pub map_writes: BTreeMap<i32, Java26MapSavedData>,
    pub map_last_id: Option<i32>,
    /// The live settings of every served level, frozen at this request's game time.
    pub world_borders: BTreeMap<Java26LevelKey, Java26WorldBorderSettings>,
    pub raids: BTreeMap<Java26LevelKey, Java26Raids>,
    pub ender_dragon_fights: BTreeMap<Java26LevelKey, Java26EnderDragonFight>,
    /// The complete persistent ticket set for each listed served level.
    pub chunk_tickets: BTreeMap<Java26LevelKey, Vec<Java26SavedChunkTicket>>,
    /// `MinecraftServer.randomSequences` as the save has to hold it, whole for
    /// the same reason the rule map is: `RandomSequences.CODEC` writes every
    /// live sequence (`RandomSequences.java:17-25`) and a sequence left out is
    /// a loot stream that silently restarts.
    pub random_sequences: RandomSequencesSaved,
    pub scheduled_events: Vec<Java26ScheduledEvent>,
    pub stopwatches: Java26Stopwatches,
    /// The live weather cycle, whole for the same reason: every field of
    /// `WeatherData.CODEC` is required (`WeatherData.java:11-15`).
    pub weather: Java26WeatherData,
    pub wandering_trader: Option<Java26WanderingTraderData>,
    /// The live scoreboard, packed the way `ScoreboardSaveData.setData`
    /// (`ScoreboardSaveData.java:33-38`) receives it: one whole `Packed`, since
    /// an objective or team left out is one the next load has lost.
    pub scoreboard: Java26ScoreboardPacked,
    /// The complete custom boss-event map; the SavedData file is written whole.
    pub custom_boss_events: Java26CustomBossEventsPacked,
    /// Dirty command-storage namespace documents. Unlisted namespaces retain
    /// their baseline files unchanged.
    pub command_storage_writes: BTreeMap<String, Java26CommandStorageContents>,
    /// The publication the patches' inventories resolve holders against.
    pub item_stack_resolver: SaveHolderResolver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WholeFileMode {
    None,
    Explicit,
    Autosave,
}

const MAX_PLAYER_BASELINE_CACHE: usize = 256;

struct WholeFileMutation {
    relative_path: PathBuf,
    bytes: Vec<u8>,
    skip_if_unchanged: bool,
    player_baseline: Option<(Uuid, NamedCompound)>,
}

#[derive(Debug)]
pub struct Java26WorldStorage {
    paths: WorldSavePaths,
    limits: WorldSaveLimits,
    session_lock: Option<WorldSessionLock>,
    unflushed_regions: Mutex<BTreeSet<PathBuf>>,
    /// Last durably written bytes for global and per-dimension SavedData files,
    /// allowing unattended autosaves to skip rewriting unchanged files.
    saved_data_cache: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    /// Last written raw player NBT documents so repeated autosaves patch in
    /// memory instead of re-reading and decompressing `.dat` files from disk.
    player_baseline_cache: Mutex<BTreeMap<Uuid, NamedCompound>>,
    /// Held exclusively while a save or flush touches region files, and shared
    /// by every record read, so a read never sees a header mid-rewrite.
    region_io: Arc<RwLock<()>>,
    region_cache: RegionFileCache,
    content_registry: Arc<ContentRegistry>,
}

impl Java26WorldStorage {
    pub fn open(paths: WorldSavePaths, limits: WorldSaveLimits) -> SaveResult<Self> {
        Self::open_with_content_registry(paths, limits, Arc::clone(ContentRegistry::vanilla()))
    }

    pub fn open_with_content_registry(
        paths: WorldSavePaths,
        limits: WorldSaveLimits,
        content_registry: Arc<ContentRegistry>,
    ) -> SaveResult<Self> {
        let limits = limits.validate()?;
        reject_symlink_root(paths.root())?;
        let session_lock = WorldSessionLock::acquire(&paths)?;
        Ok(Self {
            paths,
            limits,
            session_lock: Some(session_lock),
            unflushed_regions: Mutex::new(BTreeSet::new()),
            saved_data_cache: Mutex::new(BTreeMap::new()),
            player_baseline_cache: Mutex::new(BTreeMap::new()),
            region_io: Arc::default(),
            region_cache: RegionFileCache::default(),
            content_registry,
        })
    }

    pub fn with_content_registry(mut self, content_registry: Arc<ContentRegistry>) -> Self {
        self.content_registry = content_registry;
        self
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.content_registry
    }

    pub(super) fn region_store(
        &self,
        path: impl Into<PathBuf>,
        external_dir: impl Into<PathBuf>,
    ) -> SaveResult<RegionFileStore> {
        RegionFileStore::with_cache(path, external_dir, self.limits, self.region_cache.clone())
    }

    pub fn paths(&self) -> &WorldSavePaths {
        &self.paths
    }

    /// Saves a tick-barrier-frozen snapshot derived from a previously loaded
    /// candidate.
    ///
    /// Ordering mirrors Vanilla `MinecraftServer.saveEverything`: edited chunks
    /// are written in place through the Anvil region store first, then each
    /// whole file (level.dat, global SavedData, dirty maps, playerdata) is
    /// replaced through its own temp file and rename.
    ///
    /// Playerdata follows `PlayerList.saveAll`: every player in the request gets
    /// a document. A player the baseline holds keeps its loaded document with
    /// only the motion fields rewritten, which is how the fields this server
    /// does not model survive. A player the baseline does not hold gets a
    /// complete document authored from the live state
    /// ([`created_player_document`]), because vanilla's first save of a player
    /// creates the file rather than patching one.
    pub fn save_snapshot(
        &self,
        baseline: &Java26WorldCandidate,
        request: &Java26SaveRequest,
    ) -> SaveResult<()> {
        self.save_scoped(baseline, request, WholeFileMode::Explicit)
    }

    /// Saves an unattended autosave snapshot, skipping global and dimension
    /// `SavedData` rewrites whose serialized bytes are unchanged since the last
    /// durable write on this storage handle.
    pub fn save_autosave(
        &self,
        baseline: &Java26WorldCandidate,
        request: &Java26SaveRequest,
    ) -> SaveResult<()> {
        self.save_scoped(baseline, request, WholeFileMode::Autosave)
    }

    pub fn save_runtime_unloads(
        &self,
        baseline: &Java26WorldCandidate,
        request: &Java26SaveRequest,
    ) -> SaveResult<()> {
        self.save_scoped(baseline, request, WholeFileMode::None)
    }

    fn save_scoped(
        &self,
        baseline: &Java26WorldCandidate,
        request: &Java26SaveRequest,
        whole_file_mode: WholeFileMode,
    ) -> SaveResult<()> {
        if !self
            .session_lock
            .as_ref()
            .is_some_and(WorldSessionLock::is_valid)
        {
            return Err(SaveError::InvalidPath {
                path: self.paths.session_lock(),
                detail: "world session lock is no longer valid",
            });
        }
        let dimension_geometries = &baseline.dimension_geometries;
        let level_keys = dimension_geometries.keys().cloned().collect::<Vec<_>>();
        validate_dimension_storage(&self.paths, &level_keys)?;

        // Encode whole files before acquiring `region_io` so reader threads
        // never wait behind gzip compression or atomic whole-file writes.
        let whole_file_inputs = if whole_file_mode != WholeFileMode::None {
            let mut prepared_map_writes = Vec::with_capacity(request.map_writes.len());
            for (id, map) in &request.map_writes {
                let path = self.paths.map_data_dat(*id);
                prepared_map_writes.push((
                    *id,
                    encode_gzip_compound(&path, &java_26_1_map_document(map), self.limits)?,
                ));
            }
            let prepared_map_index_write = request
                .map_last_id
                .map(|last_id| {
                    encode_gzip_compound(
                        &self.paths.map_index_dat(),
                        &java_26_1_map_index_document(last_id),
                        self.limits,
                    )
                })
                .transpose()?;
            let mut prepared_dimension_writes =
                Vec::with_capacity(request.world_borders.len() + request.raids.len());
            for (dimension, settings) in &request.world_borders {
                dimension_geometries
                    .get(dimension)
                    .ok_or_else(|| missing_geometry(dimension))?;
                let relative_path = PathBuf::from("dimensions")
                    .join(dimension.storage_relative_path())
                    .join("data")
                    .join("minecraft")
                    .join("world_border.dat");
                let path = self.paths.root().join(&relative_path);
                prepared_dimension_writes.push((
                    relative_path,
                    encode_gzip_compound(
                        &path,
                        &java_26_1_world_border_document(*settings),
                        self.limits,
                    )?,
                ));
            }
            for (dimension, raids) in &request.raids {
                dimension_geometries
                    .get(dimension)
                    .ok_or_else(|| missing_geometry(dimension))?;
                let relative_path = PathBuf::from("dimensions")
                    .join(dimension.storage_relative_path())
                    .join("data")
                    .join("minecraft")
                    .join("raids.dat");
                let path = self.paths.root().join(&relative_path);
                prepared_dimension_writes.push((
                    relative_path,
                    encode_gzip_compound(&path, &java_26_1_raids_document(raids), self.limits)?,
                ));
            }
            for (dimension, fight) in &request.ender_dragon_fights {
                dimension_geometries
                    .get(dimension)
                    .ok_or_else(|| missing_geometry(dimension))?;
                let relative_path = PathBuf::from("dimensions")
                    .join(dimension.storage_relative_path())
                    .join("data")
                    .join("minecraft")
                    .join(ENDER_DRAGON_FIGHT_FILE);
                let path = self.paths.root().join(&relative_path);
                prepared_dimension_writes.push((
                    relative_path,
                    encode_gzip_compound(
                        &path,
                        &java_26_1_ender_dragon_fight_document(fight),
                        self.limits,
                    )?,
                ));
            }
            let mut prepared_command_storage_writes =
                Vec::with_capacity(request.command_storage_writes.len());
            for (namespace, contents) in &request.command_storage_writes {
                let relative_path = command_storage_relative_path(namespace)?;
                let path = self.paths.root().join(&relative_path);
                prepared_command_storage_writes.push((
                    relative_path,
                    encode_gzip_compound(
                        &path,
                        &java_26_1_command_storage_document(contents),
                        self.limits,
                    )?,
                ));
            }
            Some(PreparedWholeFiles {
                dimensions: prepared_dimension_writes,
                maps: prepared_map_writes,
                map_index: prepared_map_index_write,
                command_storage: prepared_command_storage_writes,
            })
        } else {
            None
        };

        let mut edited_chunks = HashSet::new();
        // Validate every whole-chunk header and duplicate set before any in-place region write.
        for chunk in &request.chunk_writes {
            dimension_geometries
                .get(&chunk.dimension)
                .ok_or_else(|| missing_geometry(&chunk.dimension))?;
            if !edited_chunks.insert((&chunk.dimension, chunk.chunk_x, chunk.chunk_z)) {
                return Err(SaveError::InvalidNbt {
                    path: None,
                    detail: format!(
                        "duplicate chunk write for {} {},{}",
                        chunk.dimension.resource_id(),
                        chunk.chunk_x,
                        chunk.chunk_z
                    ),
                });
            }
            require_java_26_1_root(&chunk.document, "chunk")?;
        }

        // The entity region is its own file tree, so its duplicate set is
        // independent from the block-chunk groups above.
        let mut written_entity_chunks = HashSet::new();
        for chunk in &request.entity_writes {
            dimension_geometries
                .get(&chunk.dimension)
                .ok_or_else(|| missing_geometry(&chunk.dimension))?;
            if !written_entity_chunks.insert((&chunk.dimension, chunk.chunk_x, chunk.chunk_z)) {
                return Err(SaveError::InvalidNbt {
                    path: None,
                    detail: format!(
                        "duplicate entity chunk write for {} {},{}",
                        chunk.dimension.resource_id(),
                        chunk.chunk_x,
                        chunk.chunk_z
                    ),
                });
            }
        }

        // Encode whole files before any region file is changed in place.
        let whole_files = whole_file_inputs
            .map(|prepared_files| {
                self.prepare_whole_file_mutations(
                    baseline,
                    request,
                    dimension_geometries,
                    prepared_files,
                )
            })
            .transpose()?;

        // Compress block and entity region chunks in parallel on rayon before
        // taking `region_io` so reader threads are never blocked on compression.
        let chunk_targets: Vec<(PathBuf, PathBuf)> = request
            .chunk_writes
            .iter()
            .map(|chunk| {
                let dir = self.paths.dimension_dir(&chunk.dimension).join("region");
                let path = dir.join(format!(
                    "r.{}.{}.mca",
                    chunk.chunk_x >> 5,
                    chunk.chunk_z >> 5
                ));
                (dir, path)
            })
            .collect();
        let entity_targets: Vec<(PathBuf, PathBuf)> = request
            .entity_writes
            .iter()
            .map(|chunk| {
                let dir = self.paths.dimension_dir(&chunk.dimension).join("entities");
                let path = dir.join(format!(
                    "r.{}.{}.mca",
                    chunk.chunk_x >> 5,
                    chunk.chunk_z >> 5
                ));
                (dir, path)
            })
            .collect();
        // Read durable POI state under a shared read lock, then encode and
        // compress chunks, entities, and POI in parallel outside `region_io.write()`.
        let edited_poi = {
            let _reading = self
                .region_io
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.prepare_poi_edits(request, dimension_geometries)?
        };
        let poi_targets: Vec<(PathBuf, PathBuf)> = edited_poi
            .iter()
            .map(|(dimension, x, z, _)| {
                let dir = self.paths.dimension_dir(dimension).join("poi");
                let path = dir.join(format!("r.{}.{}.mca", x >> 5, z >> 5));
                (dir, path)
            })
            .collect();
        let compression_items: Vec<(
            &std::path::Path,
            WorldSaveLimits,
            &NamedCompound,
            RegionCompression,
        )> =
            request
                .chunk_writes
                .iter()
                .zip(&chunk_targets)
                .map(|(chunk, (_, path))| {
                    (
                        path.as_path(),
                        self.limits,
                        &chunk.document,
                        request.region_compression,
                    )
                })
                .chain(request.entity_writes.iter().zip(&entity_targets).map(
                    |(chunk, (_, path))| {
                        (
                            path.as_path(),
                            self.limits,
                            &chunk.document,
                            request.region_compression,
                        )
                    },
                ))
                .chain(edited_poi.iter().zip(&poi_targets).map(
                    |((_, _, _, document), (_, path))| {
                        (
                            path.as_path(),
                            self.limits,
                            document,
                            request.region_compression,
                        )
                    },
                ))
                .collect();
        let prepared_payloads = prepare_chunks_parallel(&compression_items)?;
        let mut prepared_iter = prepared_payloads.into_iter();
        let prepared_chunks: Vec<_> = prepared_iter
            .by_ref()
            .take(request.chunk_writes.len())
            .collect();
        let prepared_entities: Vec<_> = prepared_iter
            .by_ref()
            .take(request.entity_writes.len())
            .collect();
        let prepared_poi_payloads: Vec<_> = prepared_iter.collect();

        {
            let _writing = self
                .region_io
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            validate_dimension_storage(&self.paths, &level_keys)?;
            self.preflight_target_region_headers(
                &request.chunk_writes,
                &edited_poi,
                &request.entity_writes,
            )?;

            for ((chunk, region_target), prepared) in request
                .chunk_writes
                .iter()
                .zip(chunk_targets)
                .zip(prepared_chunks)
            {
                self.write_prepared_region_chunk(
                    region_target,
                    (chunk.chunk_x, chunk.chunk_z),
                    prepared,
                    request.region_compression,
                    request.region_timestamp,
                )?;
            }

            for (((_, x, z, _), region_target), prepared) in edited_poi
                .into_iter()
                .zip(poi_targets)
                .zip(prepared_poi_payloads)
            {
                self.write_prepared_region_chunk(
                    region_target,
                    (x, z),
                    prepared,
                    request.region_compression,
                    request.region_timestamp,
                )?;
            }

            for ((chunk, region_target), prepared) in request
                .entity_writes
                .iter()
                .zip(entity_targets)
                .zip(prepared_entities)
            {
                self.write_prepared_region_chunk(
                    region_target,
                    (chunk.chunk_x, chunk.chunk_z),
                    prepared,
                    request.region_compression,
                    request.region_timestamp,
                )?;
            }
        }

        if let Some(files) = whole_files {
            self.write_whole_files(files, whole_file_mode == WholeFileMode::Autosave)?;
        }
        if !self
            .session_lock
            .as_ref()
            .is_some_and(WorldSessionLock::is_valid)
        {
            return Err(SaveError::InvalidPath {
                path: self.paths.session_lock(),
                detail: "world session lock changed during save",
            });
        }
        Ok(())
    }

    fn prepare_whole_file_mutations(
        &self,
        baseline: &Java26WorldCandidate,
        request: &Java26SaveRequest,
        dimension_geometries: &Java26DimensionGeometries,
        prepared_files: PreparedWholeFiles,
    ) -> SaveResult<Vec<WholeFileMutation>> {
        let PreparedWholeFiles {
            dimensions: prepared_dimension_writes,
            maps: prepared_map_writes,
            map_index: prepared_map_index_write,
            command_storage: prepared_command_storage_writes,
        } = prepared_files;
        let level_document = baseline.level.raw_with_saved_state_and_data_packs(
            &request.level_state,
            request.data_packs.as_ref(),
        )?;
        let game_rules_path = self.paths.game_rules_dat();
        let saved_data = |relative_path: PathBuf, bytes: Vec<u8>| WholeFileMutation {
            relative_path,
            bytes,
            skip_if_unchanged: true,
            player_baseline: None,
        };
        let always_write = |relative_path: PathBuf, bytes: Vec<u8>| WholeFileMutation {
            relative_path,
            bytes,
            skip_if_unchanged: false,
            player_baseline: None,
        };
        let mut mutations = vec![
            always_write(
                PathBuf::from("level.dat"),
                encode_gzip_owned_compound(&self.paths.level_dat(), level_document, self.limits)?,
            ),
            saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("world_gen_settings.dat"),
                encode_gzip_compound(
                    &self.paths.world_gen_settings_dat(),
                    &baseline.world_gen_settings.document,
                    self.limits,
                )?,
            ),
            saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("game_rules.dat"),
                encode_gzip_owned_compound(
                    &game_rules_path,
                    java_26_1_game_rules_document(&request.game_rules),
                    self.limits,
                )?,
            ),
            saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("random_sequences.dat"),
                encode_gzip_owned_compound(
                    &self.paths.random_sequences_dat(),
                    java_26_1_random_sequences_document(&request.random_sequences),
                    self.limits,
                )?,
            ),
            saved_data(
                stopwatches_relative_path(),
                encode_gzip_owned_compound(
                    &self.paths.root().join(stopwatches_relative_path()),
                    java_26_1_stopwatches_document(&request.stopwatches),
                    self.limits,
                )?,
            ),
            saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("scheduled_events.dat"),
                encode_gzip_owned_compound(
                    &self
                        .paths
                        .saved_data_dir()
                        .join("minecraft")
                        .join("scheduled_events.dat"),
                    java_26_1_scheduled_events_document(&request.scheduled_events),
                    self.limits,
                )?,
            ),
            saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("world_clocks.dat"),
                encode_gzip_owned_compound(
                    &self.paths.world_clocks_dat(),
                    java_26_1_world_clocks_document(&request.world_clocks),
                    self.limits,
                )?,
            ),
            saved_data(
                PathBuf::from("data").join("minecraft").join("weather.dat"),
                encode_gzip_owned_compound(
                    &self
                        .paths
                        .saved_data_dir()
                        .join("minecraft")
                        .join("weather.dat"),
                    java_26_1_weather_document(request.weather),
                    self.limits,
                )?,
            ),
            saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("scoreboard.dat"),
                encode_gzip_owned_compound(
                    &self
                        .paths
                        .saved_data_dir()
                        .join("minecraft")
                        .join("scoreboard.dat"),
                    java_26_1_scoreboard_document(&request.scoreboard),
                    self.limits,
                )?,
            ),
            saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("custom_boss_events.dat"),
                save_java_26_1_custom_boss_events(
                    &self.paths,
                    &request.custom_boss_events,
                    self.limits,
                )?,
            ),
        ];
        if let Some(data) = request.wandering_trader {
            mutations.push(saved_data(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("wandering_trader.dat"),
                encode_gzip_owned_compound(
                    &wandering_trader_dat(&self.paths),
                    java_26_1_wandering_trader_document(data),
                    self.limits,
                )?,
            ));
        }
        for (relative_path, bytes) in prepared_dimension_writes {
            mutations.push(saved_data(relative_path, bytes));
        }
        for (dimension, tickets) in &request.chunk_tickets {
            dimension_geometries
                .get(dimension)
                .ok_or_else(|| missing_geometry(dimension))?;
            let relative_path = PathBuf::from("dimensions")
                .join(dimension.storage_relative_path())
                .join("data")
                .join("minecraft")
                .join("chunk_tickets.dat");
            let path = self.paths.root().join(&relative_path);
            mutations.push(saved_data(
                relative_path,
                encode_gzip_owned_compound(
                    &path,
                    java_26_1_chunk_tickets_document(tickets),
                    self.limits,
                )?,
            ));
        }
        for (id, bytes) in prepared_map_writes {
            mutations.push(always_write(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("maps")
                    .join(format!("{id}.dat")),
                bytes,
            ));
        }
        if let Some(bytes) = prepared_map_index_write {
            mutations.push(always_write(
                PathBuf::from("data")
                    .join("minecraft")
                    .join("maps")
                    .join("last_id.dat"),
                bytes,
            ));
        }
        for (relative_path, bytes) in prepared_command_storage_writes {
            mutations.push(always_write(relative_path, bytes));
        }

        let mut patched_uuids = HashSet::new();
        for patch in &request.player_patches {
            if !patched_uuids.insert(patch.uuid) {
                return Err(SaveError::InvalidNbt {
                    path: Some(self.paths.player_data_dir()),
                    detail: format!("duplicate player save patch for {}", patch.uuid),
                });
            }
        }
        // Only write dirty/modified player `.dat`s; untouched historical offline
        // player files stay untouched on disk without re-encoding or GZIP writes.
        let player_store = PlayerDataStore::new(self.paths.clone(), self.limits)?;
        let stats_writes = request
            .player_patches
            .iter()
            .map(|patch| (patch.uuid, &patch.stats))
            .chain(
                request
                    .departed_player_stats
                    .iter()
                    .map(|(uuid, stats)| (*uuid, stats)),
            );
        for (uuid, stats) in stats_writes {
            mutations.push(always_write(
                PathBuf::from("players")
                    .join("stats")
                    .join(format!("{}.json", uuid.hyphenated())),
                stats.to_file_bytes_with_registry(&self.content_registry),
            ));
        }
        for (uuid, advancements) in &request.player_advancements {
            mutations.push(always_write(
                PathBuf::from("players")
                    .join("advancements")
                    .join(format!("{}.json", uuid.hyphenated())),
                advancements.to_file_bytes(),
            ));
        }
        for patch in &request.player_patches {
            let document = match baseline.players.get(&patch.uuid) {
                Some((_source, player)) if player.has_retained_raw() => player
                    .raw_with_player_save_patch(patch, request.item_stack_resolver.as_dyn())?,
                baseline_entry => {
                    let cached_doc = self
                        .player_baseline_cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get(&patch.uuid)
                        .cloned();
                    let loaded_doc = match cached_doc {
                        Some(doc) => Some(doc),
                        None => player_store.load(patch.uuid)?.map(|loaded| loaded.document),
                    };
                    match loaded_doc {
                        Some(mut doc) => {
                            patch_loaded_player_document(
                                &mut doc,
                                baseline_entry.map(|(_, player)| player.warden_spawn_tracker()),
                                patch,
                                request.item_stack_resolver.as_dyn(),
                            )?;
                            doc
                        }
                        None => {
                            created_player_document(patch, request.item_stack_resolver.as_dyn())?
                        }
                    }
                }
            };
            let player_path = self
                .paths
                .player_data_dir()
                .join(format!("{}.dat", patch.uuid.hyphenated()));
            let bytes = encode_gzip_compound(&player_path, &document, self.limits)?;
            mutations.push(WholeFileMutation {
                relative_path: PathBuf::from("players")
                    .join("data")
                    .join(format!("{}.dat", patch.uuid.hyphenated())),
                bytes,
                skip_if_unchanged: false,
                player_baseline: Some((patch.uuid, document)),
            });
        }
        Ok(mutations)
    }

    fn preflight_target_region_headers(
        &self,
        chunk_writes: &[Java26CreatedChunk],
        prepared_poi: &[(Java26LevelKey, i32, i32, NamedCompound)],
        entity_writes: &[Java26EntityChunkWrite],
    ) -> SaveResult<()> {
        let mut checked = HashSet::new();
        for chunk in chunk_writes {
            let dir = self.paths.dimension_dir(&chunk.dimension).join("region");
            let path = dir.join(format!(
                "r.{}.{}.mca",
                chunk.chunk_x >> 5,
                chunk.chunk_z >> 5
            ));
            if checked.insert(path.clone()) {
                self.region_store(&path, &dir)?
                    .validate_existing_header_for_write()?;
            }
        }
        for (dimension, x, z, _) in prepared_poi {
            let dir = self.paths.dimension_dir(dimension).join("poi");
            let path = dir.join(format!("r.{}.{}.mca", x >> 5, z >> 5));
            if checked.insert(path.clone()) {
                self.region_store(&path, &dir)?
                    .validate_existing_header_for_write()?;
            }
        }
        for chunk in entity_writes {
            let dir = self.paths.dimension_dir(&chunk.dimension).join("entities");
            let path = dir.join(format!(
                "r.{}.{}.mca",
                chunk.chunk_x >> 5,
                chunk.chunk_z >> 5
            ));
            if checked.insert(path.clone()) {
                self.region_store(&path, &dir)?
                    .validate_existing_header_for_write()?;
            }
        }
        Ok(())
    }

    /// Reads every canonical save candidate without publishing it to `ServerWorld`.
    ///
    /// Stage 3 deep component, block-entity/entity subtype (including nested
    /// items), registry, and cross-tree UUID validation must consume this
    /// candidate before it can become canonical world state.
    /// Whether this directory already holds a level.
    ///
    /// Vanilla's discriminator is the file's existence rather than whether it
    /// reads: `LevelStorageAccess.getDataTag` returns nothing for a directory
    /// with no `level.dat`, and `Main` creates a world from the properties
    /// (`Main.java:162-172`), while a `level.dat` that is there and unreadable
    /// is an error nobody papers over. Keeping that split is what lets an empty
    /// directory become a save without letting a corrupt one be replaced by
    /// one.
    pub fn holds_level_data(&self) -> bool {
        self.paths.level_dat().exists() || self.paths.level_dat_old().exists()
    }

    /// Publishes a first save into a directory that has none.
    ///
    /// The chunks are written the way `save_snapshot` writes them: in place
    /// through the Anvil store, before the whole files.
    pub fn create_world(
        &self,
        save: &Java26CreatedSave,
        region_compression: RegionCompression,
        region_timestamp: u32,
    ) -> SaveResult<()> {
        if !self
            .session_lock
            .as_ref()
            .is_some_and(WorldSessionLock::is_valid)
        {
            return Err(SaveError::InvalidPath {
                path: self.paths.session_lock(),
                detail: "world session lock is no longer valid",
            });
        }
        if self.holds_level_data() {
            return Err(SaveError::InvalidPath {
                path: self.paths.level_dat(),
                detail: "this directory already holds a level and is not this server's to create",
            });
        }
        let settings = super::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
            save.world_gen_settings.clone(),
            0,
        )?;
        let level_keys = settings.dimensions.keys().cloned().collect::<Vec<_>>();
        validate_dimension_storage(&self.paths, &level_keys)?;

        let mut written = HashSet::new();
        let mut targets = Vec::with_capacity(save.chunks.len());
        for chunk in &save.chunks {
            if !settings.dimensions.contains_key(&chunk.dimension) {
                return Err(SaveError::InvalidNbt {
                    path: None,
                    detail: format!(
                        "created chunk names undeclared level {}",
                        chunk.dimension.resource_id()
                    ),
                });
            }
            if !written.insert((&chunk.dimension, chunk.chunk_x, chunk.chunk_z)) {
                return Err(SaveError::InvalidNbt {
                    path: None,
                    detail: format!(
                        "duplicate created chunk for {} {},{}",
                        chunk.dimension.resource_id(),
                        chunk.chunk_x,
                        chunk.chunk_z
                    ),
                });
            }
            let dir = self.paths.dimension_dir(&chunk.dimension).join("region");
            let path = dir.join(format!(
                "r.{}.{}.mca",
                chunk.chunk_x >> 5,
                chunk.chunk_z >> 5
            ));
            targets.push((dir, path));
        }
        let compression_items: Vec<(
            &std::path::Path,
            WorldSaveLimits,
            &NamedCompound,
            RegionCompression,
        )> = save
            .chunks
            .iter()
            .zip(&targets)
            .map(|(chunk, (_, path))| {
                (
                    path.as_path(),
                    self.limits,
                    &chunk.document,
                    region_compression,
                )
            })
            .collect();
        let prepared_chunks = prepare_chunks_parallel(&compression_items)?;
        for ((chunk, region_target), prepared) in
            save.chunks.iter().zip(targets).zip(prepared_chunks)
        {
            self.write_prepared_region_chunk(
                region_target,
                (chunk.chunk_x, chunk.chunk_z),
                prepared,
                region_compression,
                region_timestamp,
            )?;
        }

        // Sync initial .mca files before level.dat appears on disk.
        self.flush()?;

        // level.dat goes last: a creation interrupted earlier leaves no level.
        self.write_whole_files(
            vec![
                WholeFileMutation {
                    relative_path: PathBuf::from("data")
                        .join("minecraft")
                        .join("world_gen_settings.dat"),
                    bytes: encode_gzip_compound(
                        &self.paths.world_gen_settings_dat(),
                        &save.world_gen_settings,
                        self.limits,
                    )?,
                    skip_if_unchanged: true,
                    player_baseline: None,
                },
                WholeFileMutation {
                    relative_path: PathBuf::from("level.dat"),
                    bytes: encode_gzip_compound(&self.paths.level_dat(), &save.level, self.limits)?,
                    skip_if_unchanged: false,
                    player_baseline: None,
                },
            ],
            false,
        )
    }

    fn write_prepared_region_chunk(
        &self,
        (region_dir, region_path): (PathBuf, PathBuf),
        (chunk_x, chunk_z): (i32, i32),
        prepared: PreparedRegionChunk,
        compression: RegionCompression,
        timestamp: u32,
    ) -> SaveResult<()> {
        self.region_store(&region_path, &region_dir)?
            .write_prepared_chunk(chunk_x, chunk_z, prepared, compression, timestamp)?;
        self.unflushed_regions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(region_path);
        Ok(())
    }

    /// Forces every region file written since the last flush, for `save-all
    /// flush` and shutdown (`RegionFileStorage.flush`, `RegionFileStorage.java:96-100`).
    pub fn flush(&self) -> SaveResult<()> {
        let _writing = self
            .region_io
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let regions: Vec<PathBuf> = std::mem::take(
            &mut *self
                .unflushed_regions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .into_iter()
        .collect();
        for (index, region_path) in regions.iter().enumerate() {
            let region_dir = region_path
                .parent()
                .expect("region paths are built inside a directory")
                .to_path_buf();
            if let Err(error) = self
                .region_store(region_path, &region_dir)
                .and_then(|store| store.flush())
            {
                self.unflushed_regions
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .extend(regions[index..].iter().cloned());
                return Err(error);
            }
        }
        Ok(())
    }

    /// Replaces each file through a temp file and rename; level.dat and
    /// playerdata keep their previous content as `*_old` the way
    /// `LevelStorageSource.saveLevelData` and `PlayerDataStorage.save` do.
    fn write_whole_files(
        &self,
        files: Vec<WholeFileMutation>,
        skip_unchanged: bool,
    ) -> SaveResult<()> {
        for WholeFileMutation {
            relative_path,
            bytes,
            skip_if_unchanged,
            player_baseline,
        } in files
        {
            let path = self.paths.root().join(&relative_path);
            if skip_unchanged && skip_if_unchanged {
                let unchanged = self
                    .saved_data_cache
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&relative_path)
                    .is_some_and(|cached| cached == &bytes);
                if unchanged && path.is_file() {
                    continue;
                }
            }
            let backup = (relative_path == std::path::Path::new("level.dat")
                || relative_path.starts_with("players/data"))
            .then(|| {
                let mut name = path.as_os_str().to_owned();
                name.push("_old");
                PathBuf::from(name)
            });
            write_atomic(&path, backup.as_deref(), &bytes)?;
            if let Some((uuid, document)) = player_baseline {
                let mut cache = self
                    .player_baseline_cache
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if cache.len() >= MAX_PLAYER_BASELINE_CACHE && !cache.contains_key(&uuid) {
                    let _ = cache.pop_first();
                }
                cache.insert(uuid, document);
            }
            if skip_if_unchanged {
                self.saved_data_cache
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(relative_path, bytes);
            }
        }
        Ok(())
    }

    /// Reads the level configuration that selects packs before those packs
    /// build the registries used for deep world data (`Main.java:271-286`;
    /// `WorldLoader.java:35-54`).
    pub fn load_bootstrap(&self) -> SaveResult<Java26WorldBootstrap> {
        self.require_session_lock("world session lock is no longer valid")?;
        reject_global_managed_symlinks(&self.paths)?;
        let loaded_level = LevelDatStore::new(self.paths.clone(), self.limits)?.load_java_26_1()?;
        let world_gen_settings = load_java_26_1_world_gen_settings(&self.paths, self.limits)?;
        Ok(Java26WorldBootstrap {
            loaded_level,
            world_gen_settings,
        })
    }

    /// Compatibility path for saves whose active selection is exactly Vanilla.
    pub fn load_candidate(&self) -> SaveResult<Java26WorldCandidate> {
        let bootstrap = self.load_bootstrap()?;
        require_compatibility_data_packs(bootstrap.level())?;
        let geometries =
            Java26DimensionGeometries::from_builtin_settings(bootstrap.world_gen_settings());
        let world_clock_ids = bbb_protocol::world_clock_types::VANILLA_WORLD_CLOCK_TYPES
            .iter()
            .map(|entry| entry.resource_id.to_string())
            .collect::<Vec<_>>();
        self.load_candidate_with_geometries(bootstrap, geometries, &world_clock_ids)
    }

    fn validate_bootstrap_for_deep(&self) -> SaveResult<()> {
        self.require_session_lock("world session lock is no longer valid")?;
        reject_global_managed_symlinks(&self.paths)
    }

    pub fn load_candidate_with_geometries(
        &self,
        bootstrap: Java26WorldBootstrap,
        geometries: Java26DimensionGeometries,
        world_clock_ids: &[String],
    ) -> SaveResult<Java26WorldCandidate> {
        self.load_candidate_with_geometries_and_content_registry(
            bootstrap,
            geometries,
            world_clock_ids,
            Arc::clone(&self.content_registry),
        )
    }

    pub fn load_candidate_with_geometries_and_content_registry(
        &self,
        bootstrap: Java26WorldBootstrap,
        geometries: Java26DimensionGeometries,
        world_clock_ids: &[String],
        content_registry: Arc<ContentRegistry>,
    ) -> SaveResult<Java26WorldCandidate> {
        self.validate_bootstrap_for_deep()?;
        let level_keys = geometries.keys().cloned().collect::<Vec<_>>();
        validate_dimension_storage(&self.paths, &level_keys)?;
        let game_rules = load_java_26_1_game_rules(&self.paths, self.limits)?;
        let world_clocks = load_java_26_1_world_clocks(&self.paths, self.limits, world_clock_ids)?;
        let random_sequences = load_java_26_1_random_sequences(&self.paths, self.limits)?;
        let scheduled_events = load_java_26_1_scheduled_events(&self.paths, self.limits)?;
        let stopwatches = load_java_26_1_stopwatches(&self.paths, self.limits)?;
        let weather = load_java_26_1_weather(&self.paths, self.limits)?;
        let wandering_trader = load_java_26_1_wandering_trader(&self.paths, self.limits)?;
        let scoreboard =
            load_java_26_1_scoreboard_with_registry(&self.paths, self.limits, &content_registry)?;
        let custom_boss_events =
            saved_data::load_java_26_1_custom_boss_events(&self.paths, self.limits)?;
        let loaded_command_storage = load_java_26_1_command_storage(&self.paths, self.limits)?;
        let (stored_maps, map_last_id) = discover_java_26_1_maps(&self.paths, self.limits)?;

        let mut dimensions = BTreeMap::new();
        let mut chunk_tickets = BTreeMap::new();
        let mut world_borders = BTreeMap::new();
        let mut raids = BTreeMap::new();
        let mut ender_dragon_fights = BTreeMap::new();
        for dimension in geometries.keys() {
            dimensions.insert(
                dimension.clone(),
                index_dimension_with_cache(
                    &self.paths,
                    self.limits,
                    dimension,
                    &self.region_cache,
                )?,
            );
            chunk_tickets.insert(
                dimension.clone(),
                load_java_26_1_chunk_tickets(&self.paths, dimension.clone(), self.limits)?,
            );
            raids.insert(
                dimension.clone(),
                load_java_26_1_raids(&self.paths, dimension, self.limits)?,
            );
            if let Some((fight, _)) =
                load_java_26_1_ender_dragon_fight(&self.paths, dimension, self.limits)?
            {
                ender_dragon_fights.insert(dimension.clone(), fight);
            }
            world_borders.insert(
                dimension.clone(),
                load_java_26_1_world_border(&self.paths, dimension.clone(), self.limits)?,
            );
        }

        let player_store = PlayerDataStore::new(self.paths.clone(), self.limits)?;
        let stored_players = player_store
            .discover_candidates()?
            .into_iter()
            .map(|candidate| candidate.uuid)
            .collect::<BTreeSet<_>>();
        let stored_player_stats = discover_java_26_1_player_stats(&self.paths)?;
        let stored_player_advancements = discover_java_26_1_player_advancements(&self.paths)?;

        let Java26WorldBootstrap {
            loaded_level,
            world_gen_settings,
        } = bootstrap;
        let command_storage = loaded_command_storage.namespaces;
        Ok(Java26WorldCandidate {
            level: loaded_level.data,
            world_gen_settings,
            column_reader: Some(Java26ColumnReader::new(
                self.paths.clone(),
                self.limits,
                geometries.clone(),
                Arc::clone(&self.region_io),
                self.region_cache.clone(),
                content_registry,
            )),
            dimension_geometries: geometries,
            dimensions,
            players: BTreeMap::new(),
            stored_players,
            player_stats: BTreeMap::new(),
            stored_player_stats,
            player_advancements: BTreeMap::new(),
            stored_player_advancements,
            game_rules,
            world_clocks,
            maps: BTreeMap::new(),
            stored_maps,
            map_last_id,
            chunk_tickets,
            world_borders,
            raids,
            ender_dragon_fights,
            random_sequences,
            scheduled_events,
            stopwatches,
            weather,
            wandering_trader,
            scoreboard,
            custom_boss_events,
            command_storage,
        })
    }

    fn require_session_lock(&self, detail: &'static str) -> SaveResult<()> {
        if self
            .session_lock
            .as_ref()
            .is_some_and(WorldSessionLock::is_valid)
        {
            return Ok(());
        }
        Err(SaveError::InvalidPath {
            path: self.paths.session_lock(),
            detail,
        })
    }

    pub fn close(mut self) -> SaveResult<()> {
        let flush_result = self.flush();
        self.region_cache.evict_all();
        let release_result = match self.session_lock.take() {
            Some(lock) => lock.release(),
            None => Ok(()),
        };
        flush_result?;
        release_result
    }
}

impl Drop for Java26WorldStorage {
    fn drop(&mut self) {
        self.region_cache.evict_all();
    }
}

struct PreparedWholeFiles {
    dimensions: Vec<(PathBuf, Vec<u8>)>,
    maps: Vec<(i32, Vec<u8>)>,
    map_index: Option<Vec<u8>>,
    command_storage: Vec<(PathBuf, Vec<u8>)>,
}

#[cfg(test)]
mod item_root_vehicle_input_tests;

#[cfg(test)]
mod region_sync_tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID;
    use bbb_server_types::save::Java26BuiltinDimension;

    use super::entity_publication_fixtures::{test_created_save, test_flat_column};
    use super::*;

    static NEXT_DIR: AtomicU64 = AtomicU64::new(1);

    fn temp_world_root(name: &str) -> PathBuf {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bbb-world-storage-sync-{name}-{}-{id}",
            std::process::id()
        ))
    }

    #[test]
    fn create_world_and_close_sync_dirty_region_files() {
        let root = temp_world_root("create-and-close");
        let _ = fs::remove_dir_all(&root);
        let paths = WorldSavePaths::new(&root).unwrap();
        let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
        let syncs = storage.region_cache.sync_counter();

        let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
        let column = test_flat_column(false, &fixture);
        let created = test_created_save(
            &crate::Java26WorldCreation {
                level_name: "sync-test".to_string(),
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
            .create_world(&created, RegionCompression::Zlib, 1)
            .unwrap();
        assert_eq!(syncs.load(Ordering::Relaxed), 1);
        assert!(storage
            .unflushed_regions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty());

        let overworld = Java26LevelKey::from_builtin(Java26BuiltinDimension::Overworld);
        let region_dir = storage.paths().dimension_dir(&overworld).join("region");
        let region_path = region_dir.join("r.0.0.mca");
        let prepared = crate::region_file::prepare_chunk_payload(
            &region_path,
            WorldSaveLimits::default(),
            &created.chunks[0].document,
            RegionCompression::Zlib,
        )
        .unwrap();
        storage
            .write_prepared_region_chunk(
                (region_dir, region_path.clone()),
                (1, 0),
                prepared,
                RegionCompression::Zlib,
                2,
            )
            .unwrap();
        assert_eq!(syncs.load(Ordering::Relaxed), 1);

        storage.close().unwrap();
        assert_eq!(syncs.load(Ordering::Relaxed), 2);

        let reopened = RegionFileStore::new(
            &region_path,
            region_path.parent().unwrap(),
            WorldSaveLimits::default(),
        )
        .unwrap();
        assert!(reopened.read_chunk(0, 0).unwrap().is_some());
        assert!(reopened.read_chunk(1, 0).unwrap().is_some());
        drop(reopened);
        fs::remove_dir_all(&root).unwrap();
    }
}
