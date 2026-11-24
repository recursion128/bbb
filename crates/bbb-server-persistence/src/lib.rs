pub mod atomic_file;
pub mod builtin_registry_resolver;
pub mod chunk_schema;
pub mod chunk_writer;
pub mod entity_schema;
pub mod level_file;
pub mod level_schema;
pub mod lz4_block;
pub mod player_advancements;
pub mod player_file;
pub mod player_schema;
pub mod poi_schema;
pub mod random_sequences_schema;
pub mod region_file;
pub mod scheduled_events_schema;
pub(crate) use bbb_server_types::schema_support;
pub mod session_lock;
pub mod stopwatches_schema;
pub mod wandering_trader_schema;
pub mod weather_schema;
pub mod world_clocks_schema;
pub mod world_gen_settings_schema;
pub mod world_storage;

pub use builtin_registry_resolver::java_26_1_persistent_nbt_context;
pub use chunk_schema::{
    replace_block_entities, write_chunk_ticks, Java26BelowZeroRetrogen, Java26BlendingData,
    Java26BlockState, Java26ChunkContext, Java26ChunkData, Java26ChunkSection, Java26ChunkStatus,
    Java26Palette, Java26SavedTick, Java26UpgradeData, JAVA26_BLENDING_HEIGHT_COUNT,
};
pub use chunk_writer::{
    full_chunk_document, full_chunk_document_with_content_registry, full_chunk_document_with_light,
    full_chunk_document_with_light_and_content_registry, generation_state_chunk_document,
    generation_state_chunk_document_with_content_registry,
    generation_state_chunk_document_with_light,
    generation_state_chunk_document_with_light_and_content_registry, pre_carvers_chunk_document,
    pre_carvers_chunk_document_with_content_registry, write_legacy_chunk_fields,
    Java26LegacyChunkFields,
};
pub use entity_schema::{Java26EntityChunkData, Java26EntityRecord, Java26EntityRecordRef};
pub use level_file::{LevelDatSource, LevelDatStore, LoadedLevelDat};
pub use level_schema::{
    Java26DataPacks, Java26DifficultySettings, Java26LevelData, Java26LevelSaveState,
    Java26LevelSpawn, LoadedJava26LevelData,
};
pub use player_advancements::{CriterionProgress, Java26PlayerAdvancements};
pub use player_file::{LoadedPlayerData, PlayerDataCandidate, PlayerDataSource, PlayerDataStore};
pub use player_schema::{
    created_player_document, Java26CreatedPlayer, Java26PlayerAbilities, Java26PlayerData,
    Java26PlayerEnderItems, Java26PlayerExperience, Java26PlayerInventory, Java26PlayerLastHurtBy,
    Java26PlayerRecipeBook, Java26PlayerRespawn, Java26PlayerSavePatch, Java26RootVehicle,
    Java26WardenSpawnTracker,
};
pub use poi_schema::{
    Java26PoiChunkData, Java26PoiRecord, Java26PoiRecordEdit, Java26PoiSection,
    Java26PoiSectionEdit,
};
pub use random_sequences_schema::{
    java_26_1_random_sequences_document, load_java_26_1_random_sequences,
};
pub use region_file::{RegionCompression, RegionFileStore, RegionOccupiedLocalSlot};
pub use session_lock::WorldSessionLock;
pub use wandering_trader_schema::{
    java_26_1_wandering_trader_document, load_java_26_1_wandering_trader, Java26WanderingTraderData,
};
pub use weather_schema::{java_26_1_weather_document, load_java_26_1_weather, Java26WeatherData};
pub use world_clocks_schema::{
    java_26_1_world_clocks_document, load_java_26_1_world_clocks, Java26WorldClockEntry,
    Java26WorldClockState, Java26WorldClocks,
};
pub use world_gen_settings_schema::{Java26LevelStem, Java26WorldGenSettings};
pub use world_storage::dimensions::{
    DimensionSectionGeometry, Java26DimensionGeometries, Java26DimensionGeometry,
};
pub use world_storage::saved_data::{
    canonical_objective_criteria, canonical_objective_criteria_with_registry,
    command_storage_relative_path, java_26_1_chunk_tickets_document,
    java_26_1_command_storage_document, java_26_1_custom_boss_events_document,
    java_26_1_map_document, java_26_1_scoreboard_document, java_26_1_world_border_document,
    load_java_26_1_custom_boss_events, load_java_26_1_maps, load_java_26_1_scoreboard,
    load_java_26_1_scoreboard_with_registry, load_java_26_1_world_border,
    save_java_26_1_custom_boss_events, Java26CommandStorageContents,
    Java26CommandStorageNamespaces, Java26CustomBossEventPacked, Java26CustomBossEventsPacked,
    Java26DragonRespawnStage, Java26EnderDragonFight, Java26LoadedMaps, Java26MapBanner,
    Java26MapFrame, Java26MapSavedData, Java26ScoreboardNumberFormat, Java26ScoreboardObjective,
    Java26ScoreboardPacked, Java26ScoreboardPackedScore, Java26ScoreboardTeam,
    Java26WorldBorderSettings,
};
pub use world_storage::{
    Java26BuiltinDimension, Java26ChunkWriteEpochs, Java26EntityChunkWrite, Java26LoadedChunk,
    Java26LoadedDimension, Java26LoadedEntityChunk, Java26LoadedPoiChunk, Java26SaveRequest,
    Java26SavedChunkTicket, Java26WorldBootstrap, Java26WorldCandidate, Java26WorldStorage,
    SaveHolderResolver,
};

pub use bbb_server_types::save::{
    Java26CreatedChunk, Java26CreatedSave, Java26LevelKey, Java26WorldCreation, Java26WorldOptions,
    Java26WorldPreset, NamedCompound, SaveResult, CREATED_WORLD_SEED, JAVA_26_1_DATA_VERSION,
};

pub const JAVA_MAIN_SERIES: &str = "main";
