use bbb_server_types::block_queries::game_type_may_build;
use std::cell::RefCell;
mod admin_surface;
mod advancement_flush;
mod advancement_loot;
mod advancement_rewards;
mod advancement_triggers;
mod beehive_release;
mod block_destroy;
mod block_entity_removal;
mod block_interaction;
mod block_push;
mod block_use;
mod boss_events;
mod brain_debug;
mod cat_spawner;
mod chunk_lifecycle;
mod chunk_updates;
mod combat;
mod command_blocks;
mod command_execution;
mod command_force_load;
mod command_locate;
mod command_minecarts;
mod command_reads;
pub(crate) mod command_storage;
pub(crate) use command_execution::WorldItemAccess;
pub(crate) use command_reads::{CommandDatapacks, EntityReads};
mod containers;
mod creaking_heart_break;
mod datapack_install;
mod debug_dimension;
mod debug_subscriptions;
pub(crate) mod dismount_search;
pub(crate) mod display_names;
mod dragon_fight;
mod enchant_with_levels;
mod ender_pearl;
mod entity_commands;
mod entity_placement;
mod entity_reference;
mod entity_transfer;
mod environment_attributes;
mod explosion;
mod external_player_ride_events;
mod external_player_vehicle;
#[cfg(test)]
mod fluid_tests;
mod game_master_blocks;
mod gamemaster_requests;
pub(crate) mod idle_timeout;
#[cfg(test)]
#[path = "tests/mob_targets/input_entry_bridges.rs"]
mod input_entry_bridges;
mod item_use;
mod item_use_on;
mod level;
mod level_blocks;
mod level_info;
mod level_keys;
mod level_sources;
mod live_feature_place;
pub(crate) mod location_effects;
mod loot_context;
pub(crate) use loot_context::ValueEffectLevel;
pub(crate) mod maps;
mod merchant_trades;
mod merchant_trading;
mod minecart_interaction;
mod mob_spawn_finalize;
mod mount_inventory;
mod movement_collision;
mod natural_spawn;
pub(crate) mod nether_portal;
mod patrol_spawner;
mod phantom_spawner;
mod player_action;
pub(crate) mod player_chat;
mod player_death_drops;
mod player_drop;
mod player_effects;
mod player_ender_pearls;
mod player_environment;
mod player_equipment;
mod player_fall_flying;
mod player_fluid;
mod player_movement;
mod player_packets;
mod player_portal;
mod player_pose;
mod player_potion;
mod player_predicate;
mod player_regeneration;
mod player_save;
mod player_shoulders;
mod player_sleep;
mod player_state;
mod player_tick;
mod player_tracking;
mod poi;
mod portal_tickets;
mod pov_clip;
mod raids;
pub(crate) mod registry_runtime;
mod restore;
mod save;
pub use self::save::{materialize_deferred_chunks, DeferredChunkWrite, DetachedUnloadBatch};
mod scheduled_events;
mod score_criteria;
pub(crate) mod scoreboard;
mod server_functions;
pub(crate) mod server_scoreboard;
mod session;
mod silverfish;
pub mod simulation_level;
pub(crate) mod simulation_tick;
mod spawn;
mod spawn_predicate_level;
mod spawner_minecarts;
mod spectator;
pub(crate) mod stopwatches;
mod structure_templates;
mod tag_query;
mod taming;
#[cfg(test)]
mod test_entities;
#[cfg(test)]
mod test_support;
mod text_display_text;
mod types;
pub(crate) mod using_item;
mod utility_menus;
mod village_siege;
mod villager_golem;
mod wandering_trader;
mod warden;
mod waypoints;
mod weather;
mod world_clock;
use self::debug_subscriptions::{LevelDebugSubscriptions, ServerDebugSubscriptions};
use self::level::*;
use self::level_blocks::{is_in_world_bounds, LoadedBlocks};
pub(crate) use self::level_blocks::{removal_affects_neighbors, stored_block_state};
use self::level_keys::LevelArena;
pub(crate) use self::level_keys::LevelId;
use self::level_sources::{
    bind_level_sources_to_published_dimension_types, level_sources_from_world_gen_settings,
};
use self::maps::ServerMapStore;
pub use self::player_chat::{ChatKind, OutgoingChat};
pub(crate) use self::player_packets::wrap_degrees;
use self::player_packets::{
    block_position, hit_location_is_finite, initializing_player_info,
    initializing_player_info_entry, player_chunk, send, sound_broadcast_range,
};
use self::player_tick::{is_chunk_tracked, ChunkTrackingView};
use self::registry_runtime::{
    load_damage_type_registry, registry_tag_contains, RuntimePersistentRegistryResolver,
};
pub(crate) use self::simulation_level::RULE_SEND_COMMAND_FEEDBACK;
use self::simulation_level::{
    command_feedback_channel, hopper_facing, BlockEventBroadcast, CommandFeedbackChannel, LevelCtx,
    SavedChunkTicket, SimulationChunkLevels, SimulationFeatureFlags, SimulationGameRules,
    CHUNK_BLOCK_TICKING_LEVEL, CHUNK_ENTITY_TICKING_LEVEL, CHUNK_FULL_LEVEL,
    CLIENT_LOADED_TIMEOUT_TICKS, ENTITY_EVENT_DEATH, ENTITY_EVENT_POOF, NO_SOURCE_ENTITY_ID,
    RULE_KEEP_INVENTORY, RULE_NATURAL_HEALTH_REGENERATION, RULE_SHOW_DEATH_MESSAGES,
    RULE_SPECTATORS_GENERATE_CHUNKS,
};
pub(crate) use self::spawn_predicate_level::SpawnPredicateLevel;
use crate::admin::{AdminSettings, AdminState};
use crate::block_behavior::affect_neighbors_after_removal as block_affect_neighbors_after_removal;
use crate::block_behavior::redstone_torch::RedstoneTorchToggle;
use crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS;
use crate::block_entity::BlockEntity;
use crate::block_events::BlockEventQueue;
use crate::chunk::noise_generator::NoiseFinalDensityProgram;
use crate::chunk::ChunkStore;
use crate::chunk_sender::ChunkKey;
use crate::commands;
use crate::commands::CommandPermission;
use crate::enchantment_facts::{load_enchantments_with_effect, PREVENT_EQUIPMENT_DROP_EFFECT};
use crate::entity::{CrossLevelEntities, EntityIdCounter, EntityStore, EntityTracker};
use crate::inventory::MENU_SLOT_COUNT;
use crate::level_event::{LevelEventSink, RecordedGameEvent};
use crate::level_ticks::LevelTicks;
use crate::mailbox::CommandEnvelope;
use crate::neighbor_updater::{NeighborUpdates, UPDATE_LIMIT};
use crate::random::GameRandom;
use crate::random_sequences::RandomSequences;
use crate::registry_sync::SynchronizedRegistries;
use crate::registry_tags::RegistryTagSnapshot;
use crate::tick_rate::TickRateManager;
use crate::world_creation::{default_level_dimension_types, default_level_sources};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::game_types::GameType;
#[cfg(test)]
use bbb_protocol::packets::GameProfile;
use bbb_protocol::packets::{
    decode_play_serverbound_with_content_registry, BlockPos, PlayServerbound, PlayTime,
    PlayerRotation, TickingState, TickingStep, Vec3d,
};
use bbb_protocol::{component_translatable_nbt, ComponentStyle, RichTextComponent};
#[cfg(test)]
use bbb_protocol::{PersistentRegistryResolver, PersistentRegistryTagMembers};
use bbb_server_datapack::damage::DamageTypeFacts;
use bbb_server_datapack::debug_generator::DebugLevelSource;
use bbb_server_datapack::flat_generator::FlatLevelSource;
use bbb_server_datapack::player_advancements::AdvancementTree;
use bbb_server_datapack::recipe::ItemTagIndex;
use bbb_server_persistence::{Java26PlayerAdvancements, Java26WeatherData, CREATED_WORLD_SEED};
use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType};
use bbb_server_types::fixture::FlatFixture;
use bbb_server_types::interaction::WorldBorder;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};
use bbb_server_types::stats::ServerStatsCounter;
use bbb_server_types::{Result, ServerWorldError};
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use uuid::Uuid;
#[cfg(test)]
pub(crate) use weather::Precipitation;

const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
pub(crate) const SPAWN_X: f64 = 0.5;
/// The y a joining player stands at, which is the y the created save files as
/// its spawn.
///
/// `MinecraftServer.setInitialSpawn` (`MinecraftServer.java:492-498`) takes it
/// from the generator: `ChunkGenerator.getSpawnHeight`, which for the
/// `minecraft:flat` generator this world declares is
/// `minY + min(worldHeight, layers.size())`
/// (`FlatLevelSource.java:62-64`). The created overworld's layer list is the
/// fixture's own solid column -- `min_y` up to and including its floor block
/// (`bbb_server_persistence::world_creation`'s `created_floor_height`) -- so the
/// sum is the first *free* y above that floor, and a player placed there
/// stands on it rather than a block above it.
///
/// The number is spelled out rather than computed because it is a `const`, and
/// `the_default_spawn_stands_on_the_fixture_floor` is what keeps it tied to
/// the fixture that derives it.
pub(crate) const SPAWN_Y: f64 = 64.0;
pub(crate) const SPAWN_Z: f64 = 0.5;
/// `Level.MAX_LEVEL_SIZE` (Level.java:98): the half-width of the horizontal
/// block domain `Level.isInWorldBoundsHorizontal` (Level.java:179-181) admits,
/// and the same number `ServerGamePacketListenerImpl` clamps a movement to.
const HORIZONTAL_LIMIT: i32 = 30_000_000;
/// `Level.MAX_ENTITY_SPAWN_Y` (Level.java:102).
const VERTICAL_LIMIT: f64 = 20_000_000.0;
pub(crate) fn level_height(chunks: &ChunkStore, heightmap: i32, x: i32, z: i32) -> Option<i32> {
    if !(-HORIZONTAL_LIMIT..HORIZONTAL_LIMIT).contains(&x)
        || !(-HORIZONTAL_LIMIT..HORIZONTAL_LIMIT).contains(&z)
    {
        return Some(chunks.sea_level() + 1);
    }
    if !chunks.is_ticking_full(x >> 4, z >> 4) {
        return Some(chunks.min_y());
    }
    chunks.final_heightmap_first_available(x >> 4, z >> 4, heightmap, x & 15, z & 15)
}
/// The buffer `handleUseItemOn` (:1339) allows on top of the player's block
/// interaction range.
const BLOCK_INTERACTION_BUFFER: f64 = 1.0;
/// The buffer `handleAttack` (:1793) and `handleInteract` (:1822) allow on top
/// of the player's entity interaction range.
const ENTITY_INTERACTION_BUFFER: f64 = 3.0;
/// `handleAttack` (:1806) hands `cannotAttackWithItem` this tolerance, so a
/// charge-gated weapon may swing slightly before it is fully charged.
const ATTACK_CHARGE_TOLERANCE: i32 = 5;
/// `SoundEvents.PLAYER_ATTACK_NODAMAGE`, played when an attack lands on
/// something it cannot hurt (`Player.attack`, Player.java:996).
const PLAYER_ATTACK_NODAMAGE_SOUND: &str = "minecraft:entity.player.attack.nodamage";
/// `SoundEvents.PLAYER_ATTACK_KNOCKBACK`, played by a sprinting full-strength
/// attack before the hurt is attempted (`Player.attack`, Player.java:965).
const PLAYER_ATTACK_KNOCKBACK_SOUND: &str = "minecraft:entity.player.attack.knockback";
/// `Player.playServerSideSound` (Player.java:1007-1009) plays every sound at
/// full volume and neutral pitch.
const PLAYER_SOUND_VOLUME: f32 = 1.0;
const PLAYER_SOUND_PITCH: f32 = 1.0;
/// `ServerPlayer.lastSentFood`'s initial value (ServerPlayer.java:239), which
/// no food level can equal so that the first `doTick` always announces one.
const UNSENT_FOOD: i32 = -99_999_999;
/// `net/minecraft/world/damagesource/DamageTypes.java:70`, the source
/// `FoodData.tick` starves with (`FoodData.java:64`, through
/// `DamageSources.starve`, `DamageSources.java:123-125`).
const STARVE_DAMAGE_TYPE: &str = "minecraft:starve";
/// `this.getBoundingBox().inflate(1.0, 0.5, 1.0)` (`Player.java:462`): the
/// reach a standing player picks things up at, which is a whole block out on
/// each horizontal side and only half a block above and below.
///
/// It is the *bounding box* that is grown, not a sphere around the eyes, so
/// the reach is not symmetric about the player: the box already runs from the
/// feet to `1.8` above them, and the half block is added to both ends of that.
pub(crate) const PICKUP_INFLATION: [f64; 3] = [1.0, 0.5, 1.0];
/// The radius `SoundEvent.getRange` (SoundEvent.java:40-42) answers for a sound
/// with no fixed range and a volume of at most one.
const DEFAULT_SOUND_RANGE: f32 = 16.0;
/// The gameplay stream's seed; fixed so a world's simulation is reproducible
/// in tests.
const WORLD_RANDOM_SEED: i64 = 0x005E_ED01_E7E1;
/// `handleSetCreativeModeSlot` (:2010) accepts every menu slot but the
/// crafting result.
const CREATIVE_SLOT_MIN: i16 = 1;
const CREATIVE_SLOT_MAX: i16 = MENU_SLOT_COUNT - 1;
/// `Container.DEFAULT_DISTANCE_BUFFER` (`Container.java:20`), the buffer
/// `Container.stillValidBlockEntity` (`:90-92`) adds to a player's block
/// interaction range before it decides an open container is still reachable.
const CONTAINER_DISTANCE_BUFFER: f64 = 4.0;

fn chest_menu_title(key: &str) -> Option<RichTextComponent> {
    RichTextComponent::try_from_component_nbt(
        component_translatable_nbt(key, Vec::new(), &ComponentStyle::default()).ok()?,
    )
    .ok()
}
/// `ChunkMap.getPlayerViewDistance` (ChunkMap.java:821-823) clamps a client's
/// request into this range before the server view distance caps it.
const MIN_PLAYER_VIEW_DISTANCE: i32 = 2;
/// The game type a created fixture `level.dat` records (`ServerWorld::fixture_game_type_id`).
const FIXTURE_GAME_TYPE: GameType = GameType::Creative;
/// `Block.UPDATE_NEIGHBORS` (`Block.java:91`): notify the six neighbours that
/// the block here changed.
const SET_BLOCK_UPDATE_NEIGHBORS: i32 = 1;
/// `Block.UPDATE_KNOWN_SHAPE` (`Block.java:95`): the caller already resolved
/// the surrounding shapes, so `updateNeighbourShapes` is skipped.
const SET_BLOCK_UPDATE_KNOWN_SHAPE: i32 = 16;
/// `Block.UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE` (`Block.java:98`).
pub(crate) const SET_BLOCK_UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE: i32 = 128;
/// `Block.UPDATE_SUPPRESS_DROPS` (`Block.java:96`): the flag
/// `Block.updateOrDestroy` turns into `Level.destroyBlock`'s `dropResources`
/// argument (`Block.java:234`).
const SET_BLOCK_UPDATE_SUPPRESS_DROPS: i32 = 32;
/// `Block.UPDATE_SKIP_ON_PLACE` (`Block.java:100`).
const SET_BLOCK_UPDATE_SKIP_ON_PLACE: i32 = 512;
/// [`SET_BLOCK_UPDATE_SUPPRESS_DROPS`], for the tests that drive
/// `ServerWorld::set_block` from outside this module.
#[cfg(test)]
pub(crate) const fn set_block_update_suppress_drops() -> i32 {
    SET_BLOCK_UPDATE_SUPPRESS_DROPS
}

/// `Block.UPDATE_MOVE_BY_PISTON` (`Block.java:97`): marks a write as one leg of
/// a piston movement, read by `LevelChunk.setBlockState` (`LevelChunk.java:305`)
/// before invoking `affectNeighborsAfterRemoval`.
const SET_BLOCK_UPDATE_MOVE_BY_PISTON: i32 = 64;
/// `LightEngine.MAX_LEVEL` — the brightest a light array cell can be.
const MAX_LIGHT_LEVEL: i32 = 15;
/// The principal the save's own game rules are written under. Vanilla has no
/// principal for them at all — `LevelStorageSource.readGameRules` hands the
/// level data a `GameRuleMap` directly — so this names the file rather than a
/// caller.
const GAME_RULE_RESTORE_PRINCIPAL: &str = "level.game_rules";

impl ServerWorld {
    /// The game type the fixture [`ServerWorldConfig`] serves, as `level.dat`
    /// stores it.
    ///
    pub fn fixture_game_type_id() -> i32 {
        FIXTURE_GAME_TYPE.id()
    }

    /// Where this world spawns a player, as the block position `level.dat`
    /// records. The world spawns inside the block, so the recorded position is
    /// the block it stands in.
    pub fn fixture_spawn_block() -> [i32; 3] {
        [
            SPAWN_X.floor() as i32,
            SPAWN_Y.floor() as i32,
            SPAWN_Z.floor() as i32,
        ]
    }
}
/// `LastSeenMessages.LAST_SEEN_MESSAGES_MAX_LENGTH` (LastSeenMessages.java:16),
/// the window `LastSeenMessagesValidator` is built with
/// (ServerGamePacketListenerImpl.java:270).
const LAST_SEEN_MESSAGES_MAX_LENGTH: usize = 20;
/// `MinecraftServer.tickServer` time-sync cadence (`MinecraftServer.java:1093-1097`).
const TIME_SYNC_INTERVAL_TICKS: u64 = 20;
/// `MinecraftServer.TICK_STATS_SPAN` (`MinecraftServer.java:198`), which is also
/// the length of the ring it indexes (`:249`).
pub(crate) const TICK_STATS_SPAN: usize = 100;

/// `MinecraftServer.AUTOSAVE_INTERVAL` (MinecraftServer.java:209), which is
/// also the initial value of `ticksUntilAutosave` (:243): the first unattended
/// save is a full interval after startup, not immediately.
const AUTOSAVE_INTERVAL_TICKS: i32 = 6000;
/// `MinecraftServer.MIMINUM_AUTOSAVE_TICKS` (MinecraftServer.java:210), the
/// floor `computeNextAutosaveInterval` clamps to (:1037).
const MINIMUM_AUTOSAVE_TICKS: i32 = 100;
/// The `intendedIntervalInSeconds` local in `computeNextAutosaveInterval`
/// (MinecraftServer.java:1037-1038).
const AUTOSAVE_INTERVAL_SECONDS: f32 = 300.0;
/// `TimeUtil.NANOSECONDS_PER_SECOND` (TimeUtil.java:9), which
/// `computeNextAutosaveInterval` divides the measured tick time into
/// (MinecraftServer.java:1032).
const NANOSECONDS_PER_SECOND: f32 = 1_000_000_000.0;

const _: () = assert!(
    AUTOSAVE_INTERVAL_TICKS == 6000 && MINIMUM_AUTOSAVE_TICKS == 100,
    "the autosave interval and its floor are the two MinecraftServer constants",
);

pub(crate) use self::player_state::*;
use self::types::WorldSpawn;
pub use self::types::*;
pub use crate::connection::{ClientboundPlayPacket, ConnectionId};
pub(in crate::world) struct WorldCold {
    pub(super) config: ServerWorldConfig,
    pub(super) fixture: FlatFixture,
    pub(super) tick_times_nanos: [i64; TICK_STATS_SPAN],
    pub(super) aggregated_tick_times_nanos: i64,
    /// The last loop iteration's `TpsDebugDimensions` sample, sent next tick.
    pub(super) pending_tick_time_sample: Option<[i64; 4]>,
    pub(super) pending_sprint_report: Option<crate::tick_rate::SprintReport>,
    pub(super) debug_profiler_delay_start: bool,
    pub(super) debug_profiler: Option<(i64, i32)>,
    /// The `minecraft:damage_type` registry this world resolves damage sources
    /// against, in the registry order `RegistryDataLoader` built it in. The
    /// catalog is fixed for the live server: Vanilla reloads tags/resources
    /// against the existing registry access, so a successful reload can only
    /// change the tag memberships used to derive these facts.
    pub(super) damage_types: std::sync::Arc<Vec<DamageTypeFacts>>,
    /// Ingredient lookups use the tags of the installed datapack generation.
    pub(super) loot_item_tags: ItemTagIndex,
    /// `FuelValues.vanillaBurnTimes` over [`Self::loot_item_tags`].
    pub(super) fuel_values: bbb_server_datapack::recipe::FuelValues,
    pub(super) crafting_item_tags: Option<Arc<ItemTagIndex>>,
    pub(super) functions: server_functions::ServerFunctionManager,
    pub(super) mob_spawns: Option<natural_spawn::MobSpawnCatalog>,
    pub(super) dimension_sky_lights: environment_attributes::DimensionSkyLights,
    pub(super) random_sequences: RefCell<RandomSequences>,
    pub(super) wandering_trader: wandering_trader::WanderingTraderSpawner,
    pub(super) village_siege: village_siege::VillageSiege,
    pub(super) patrol_spawner: patrol_spawner::PatrolSpawner,
    pub(super) phantom_spawner: phantom_spawner::PhantomSpawner,
    pub(super) cat_spawner: cat_spawner::CatSpawner,
    pub(super) boss_events: boss_events::ServerBossEvents,
    pub(super) command_storage: command_storage::ServerCommandStorage,
    pub(super) structure_templates: structure_templates::StructureTemplateManager,
    /// Declared dimension stems whose generator type is not implemented by this
    /// server (for example, a modded or custom datapack generator), keyed to
    /// their generator id. These dimensions are logged as warnings at world
    /// startup/restore and are not ticked, written, or joined.
    pub(super) unserved_levels: BTreeMap<Java26LevelKey, String>,
    /// The `dimension_type` each level's own stem names, which is not the same
    /// thing as the level's id: a save is free to file
    /// `minecraft:overworld_caves` under the overworld, and vanilla describes
    /// that level by the type rather than by the name.
    pub(super) level_dimension_types: BTreeMap<LevelId, String>,
    pub(super) infiniburn_cache: simulation_tick::InfiniburnCache,
    pub(super) difficulty_locked: bool,
    pub(super) hardcore: bool,
    /// A dedicated server has three writers for it: startup (`DedicatedServer.java:237`),
    /// `/defaultgamemode` (`DefaultGameModeCommands.java:22-23`), and
    /// `DedicatedServer.setGameMode` (`:809-813`) via `serversettings/game_mode/set`,
    /// which also updates the persisted `gamemode` property.
    pub(super) default_game_type: GameType,
    pub(super) player_idle_timeout: i32,
    pub(super) stopwatches: stopwatches::Stopwatches,
    pub(super) scheduled_events: scheduled_events::ScheduledEvents,
    pub(super) seed: i64,
    pub(super) generate_structures: bool,
    pub(super) default_spawn: WorldSpawn,
    /// The most recent wall-clock instant a command carried into the world, in
    /// epoch milliseconds.
    pub(super) wall_clock_millis: i64,
    pub(super) debug_subscriptions: ServerDebugSubscriptions,
    /// What the save already held about a player, keyed by profile uuid: the
    /// fields `ServerPlayer.readAdditionalSaveData` reads back and this world
    /// projects into a session at its next join.
    pub(super) restored_players: BTreeMap<Uuid, RestoredPlayer>,
    pub(super) restored_player_stats: BTreeMap<Uuid, ServerStatsCounter>,
    pub(super) restored_player_advancements: BTreeMap<Uuid, Java26PlayerAdvancements>,
    /// Lazy disk reader for offline playerdata/stats/advancements when the
    /// world was restored from storage.
    pub(super) stored_data_reader:
        Option<bbb_server_persistence::world_storage::Java26ColumnReader>,
    /// Offline playerdata/stats/advancements discovered on disk and not yet
    /// loaded into memory.
    pub(super) stored_players: BTreeSet<Uuid>,
    pub(super) stored_player_stats: BTreeSet<Uuid>,
    pub(super) stored_player_advancements: BTreeSet<Uuid>,
    pub(super) advancement_tree: AdvancementTree,
    pub(super) advancement_triggers: advancement_triggers::AdvancementTriggers,
    /// Full modeled save snapshots frozen before a disconnect tears down the
    /// Player's vehicle relation. They remain save inputs until that profile
    /// rejoins and takes ownership of the snapshot again.
    pub(super) departed_player_snapshots: BTreeMap<Uuid, player_save::DepartedPlayerSnapshot>,
    /// Trackers read from the documents of players who are not online; a join
    /// moves the value into the session.
    pub(super) restored_warden_trackers: BTreeMap<Uuid, crate::entity::WardenSpawnTracker>,
    pub(super) autosave_staging: Option<save::AutosaveStaging>,
    #[cfg(test)]
    pub(super) publication: Arc<WorldSnapshot>,
}

pub struct ServerWorld {
    tick: u64,
    game_time: i64,
    tick_rate: TickRateManager,
    /// The `preRemoveSideEffects` overrides that reach past the item queue,
    /// with the level they were raised in.
    pending_removal_effects: Vec<(
        LevelId,
        simulation_level::pending_spawns::PendingBlockEntityRemovalEffect,
    )>,
    /// Delivery packets held between the level frame that ran a listener and
    /// the tick phase that drains them; some frames carry no `events` list of
    /// their own. See [`simulation_tick::ServerWorld::post_pending_game_events`].
    deferred_game_event_events: Vec<WorldEvent>,
    pending_camera_transitions: Vec<(
        LevelId,
        commands::CommandCameraTarget,
        TeleportTransition<LevelId>,
    )>,
    /// Command-triggered player disconnects deferred until the current packet
    /// or tick sub-step finishes (`ServerCommonPacketListenerImpl.java:181-185`).
    pending_disconnects: Vec<(ConnectionId, bbb_protocol::RichTextComponent)>,
    last_command_sequence: u64,
    pub(in crate::world) datapack_tables: Arc<datapack_install::DatapackRuntimeTables>,
    entity_ids: EntityIdCounter,
    next_teleport_id: i32,
    stopping: bool,
    light_failed: bool,
    /// The gameplay stream every level, entity store and player shares.
    pub(crate) random: GameRandom,
    /// A `SUCCESS_SERVER` answered inside the current `handleUseItemOn`.
    use_item_on_server_swing: bool,
    weather: Java26WeatherData,
    scoreboard: server_scoreboard::ServerScoreboard,
    maps: ServerMapStore,
    levels: LevelArena,
    published_player_list_revision: u64,
    /// The dynamic registries the configuration phase publishes, and the only
    /// place a `dimension_type` wire index comes from.
    registries: SynchronizedRegistries,
    /// The game rules the world simulation reads, see [`SimulationGameRules`].
    game_rules: SimulationGameRules,
    /// Enabled experimental gameplay feature flags.
    feature_flags: SimulationFeatureFlags,
    /// World clocks indexed by the published dynamic-registry holder id.
    world_clocks: world_clock::ServerWorldClocks,
    ticks_until_autosave: i32,
    auto_save: bool,
    /// `MinecraftServer.emptyTicks` (`MinecraftServer.java:290`).
    empty_ticks: i32,
    /// The four stored user lists, the mutable settings and the game rules —
    /// everything `PlayerList` and `DedicatedServerProperties` own in vanilla.
    admin: AdminState,
    sessions: FxHashMap<ConnectionId, PlayerSession>,
    cold: Box<WorldCold>,
}

impl ServerWorld {
    /// The loaded tags of the installed datapack generation.
    pub(crate) fn tags(&self) -> &crate::registry_tags::RegistryTagSnapshot {
        self.registries.registry_tags()
    }

    pub fn plugins(&self) -> &Arc<crate::plugin::ServerPluginRegistry> {
        &self.cold.config.plugins
    }

    pub(super) fn record_unserved_levels(&mut self, unserved: BTreeMap<Java26LevelKey, String>) {
        for (dimension, generator) in &unserved {
            tracing::warn!(
                event = "world.level_unserved",
                dimension = dimension.resource_id(),
                generator = generator.as_str(),
                custom_dimension = dimension.builtin().is_none(),
                "declared dimension uses an unimplemented generator and will not be served"
            );
        }
        self.cold.unserved_levels = unserved;
    }

    #[cfg(test)]
    pub(crate) fn set_redstone_experiments_for_test(&mut self, enabled: bool) {
        self.feature_flags.redstone_experiments = enabled;
    }

    #[cfg(test)]
    pub(crate) fn set_minecart_improvements_for_test(&mut self, enabled: bool) {
        self.feature_flags.minecart_improvements = enabled;
    }

    #[cfg(test)]
    pub(crate) fn spawn_minecart_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        position: Vec3d,
        delta_movement: Vec3d,
    ) -> Option<i32> {
        let store = &mut self.level_mut(dimension).entities;
        let id = crate::entity::minecart::spawn_minecart(store, position, 0.0)?;
        store.get_mut(id)?.set_delta_movement(delta_movement);
        Some(id)
    }

    /// A world with no save behind it, generating every production level from
    /// the same parsed settings document a created save publishes.
    pub fn new(config: ServerWorldConfig) -> Result<Self> {
        let mut world = if config.datapacks.is_some() {
            let settings = crate::world_creation::default_world_gen_settings()
                .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
            let declared =
                level_sources_from_world_gen_settings(&settings, config.datapacks.as_ref(), true)
                    .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
            let mut world = Self::with_level_sources(
                config,
                &declared.served,
                &declared.dimension_types,
                settings.seed,
                settings.generate_structures,
            )?;
            world.record_unserved_levels(declared.unserved);
            world
        } else {
            let sources = default_level_sources()
                .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?
                .into_iter()
                .map(|(dimension, source)| (dimension, ServerLevelSource::Flat(source)))
                .collect();
            let dimension_types = default_level_dimension_types()
                .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
            Self::with_level_sources(
                config,
                &sources,
                &dimension_types,
                CREATED_WORLD_SEED,
                false,
            )?
        };
        world.finish_debug_dimension_initialization(false)?;
        world.refresh_all_sky_darkening();
        Ok(world)
    }

    /// One `ServerLevel` per declared stem, in the order
    /// `MinecraftServer.createLevels` builds them.
    ///
    fn with_level_sources(
        config: ServerWorldConfig,
        sources: &BTreeMap<Java26LevelKey, ServerLevelSource>,
        dimension_types: &BTreeMap<Java26LevelKey, String>,
        generation_seed: i64,
        generate_structures: bool,
    ) -> Result<Self> {
        config.validate()?;
        let fixture = FlatFixture::new()?;
        if !sources.contains_key(&Java26LevelKey::from(Java26BuiltinDimension::Overworld)) {
            return Err(ServerWorldError::InvalidConfiguration(
                "Overworld settings missing".to_string(),
            ));
        }
        // Every holder id comes from the registry transaction the session receives.
        let registries = SynchronizedRegistries::load_with_content_registry(
            config.datapacks.as_deref(),
            Arc::clone(config.plugins.content_registry()),
        )
        .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
        let (sources, level_dimension_types) =
            bind_level_sources_to_published_dimension_types(sources, dimension_types, &registries)?;
        let entity_ids = EntityIdCounter::new();
        let prevent_equipment_drop_enchantments = load_enchantments_with_effect(
            config.datapacks.as_deref(),
            PREVENT_EQUIPMENT_DROP_EFFECT,
        );
        let prevent_armor_change_enchantments = load_enchantments_with_effect(
            config.datapacks.as_deref(),
            crate::enchantment_facts::PREVENT_ARMOR_CHANGE_EFFECT,
        );
        let datapack_tables =
            datapack_install::DatapackRuntimeTables::load(config.datapacks.as_ref());
        let enchantment_attributes = crate::enchantment_facts::load_enchantment_attribute_effects(
            config.datapacks.as_deref(),
        );
        let enchantment_location_effects =
            crate::enchantment_location_effects::load_enchantment_location_effects(
                config.datapacks.as_deref(),
            );
        let enchantment_tick_effects =
            crate::enchantment_location_effects::load_enchantment_tick_effects(
                config.datapacks.as_deref(),
            );
        let weapon_effects =
            crate::enchantment_weapon_effects::load_weapon_effects(config.datapacks.as_ref());
        let fluid_shapes = crate::fluid::SharedFluidShapeCache::default();
        let item_publication = crate::entity::ItemPublication::new(Arc::new(
            registry_runtime::PlayerItemStackResolver::new(config.datapacks.as_ref(), &registries),
        ));
        let random = GameRandom::with_seed(WORLD_RANDOM_SEED);
        let mut levels = sources
            .iter()
            .map(|(dimension, source)| {
                let element_id = level_dimension_types
                    .get(dimension)
                    .expect("bound source has a dimension type");
                let spawn_facts =
                    spawn::dimension_spawn_facts(config.datapacks.as_deref(), element_id, source)?;
                let mut level = ServerLevel::new(
                    dimension.clone(),
                    source.clone(),
                    spawn_facts,
                    &registries,
                    config.datapacks.as_ref(),
                    entity_ids.clone(),
                    random.clone(),
                )?;
                if !level.chunks.has_noise_source() {
                    if let Some(startup) = &config.datapacks {
                        let shape = bbb_server_datapack::dimension_type::dimension_type_shape(
                            &startup.registries,
                            element_id,
                        )
                        .map_err(|error| {
                            ServerWorldError::InvalidConfiguration(error.to_string())
                        })?;
                        level
                            .chunks
                            .configure_ordinary_structure_generation(
                                Arc::clone(startup),
                                generation_seed,
                                shape,
                                generate_structures,
                                Some(crate::chunk::generation_publication::GenerationItemPublication::new(
                                    Arc::new(registry_runtime::PlayerItemStackResolver::new(config.datapacks.as_ref(), &registries)),
                                )),
                            )
                            .map_err(|error| {
                                ServerWorldError::InvalidConfiguration(error.to_string())
                            })?;
                    }
                }
                level.chunks.install_fluid_shape_cache(fluid_shapes.clone());
                level
                    .chunks
                    .install_content_registry(Arc::clone(config.plugins.content_registry()));
                level
                    .entities
                    .set_prevent_equipment_drop_enchantments(Arc::clone(
                        &prevent_equipment_drop_enchantments,
                    ));
                level
                    .entities
                    .set_prevent_armor_change_enchantments(Arc::clone(
                        &prevent_armor_change_enchantments,
                    ));
                level
                    .entities
                    .set_enchantment_slots(Arc::clone(&datapack_tables.enchantment_slots));
                level
                    .entities
                    .set_enchantment_attribute_effects(Arc::clone(&enchantment_attributes));
                level
                    .entities
                    .set_enchantment_location_effects(Arc::clone(&enchantment_location_effects));
                level
                    .entities
                    .set_enchantment_tick_effects(Arc::clone(&enchantment_tick_effects));
                level.entities.set_weapon_effects(Arc::clone(&weapon_effects));
                level.entities.set_item_publication(item_publication.clone());
                level.entities.set_plugins(Arc::clone(&config.plugins));
                Ok((dimension.clone(), level))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        debug_dimension::install_global_debug_view(&mut levels);
        let levels = LevelArena::new(levels)?;
        let level_dimension_types = level_dimension_types
            .into_iter()
            .map(|(key, value)| {
                (
                    levels.names.id(&key).expect("bound level dimension type"),
                    value,
                )
            })
            .collect();
        let world_clocks =
            world_clock::ServerWorldClocks::defaults(&registries, config.datapacks.as_deref())?;
        // `GameRules(FeatureFlagSet)` keeps only the rules the world's enabled
        // features allow.
        let enabled_features = config
            .datapacks
            .as_ref()
            .map_or_else(
                bbb_server_datapack::startup::default_enabled_features_26_1,
                |startup| startup.enabled_features.clone(),
            )
            .into_iter()
            .collect::<Vec<_>>();
        let feature_flags =
            SimulationFeatureFlags::from_feature_ids(enabled_features.iter().map(String::as_str));
        let player_idle_timeout = config.player_idle_timeout;
        let admin = AdminState::new(
            AdminSettings {
                // `DedicatedServer.forceDifficulty` applies the property at
                // startup, so the setting starts at the CLI value.
                difficulty: config.difficulty,
                game_mode: config.game_type,
                enforce_allowlist: config.enforce_allowlist,
                use_allowlist: config.use_allowlist,
                max_players: config.max_players as i32,
                motd: config.motd.clone(),
                spawn_protection_radius: config.spawn_protection_radius,
                operator_user_permission_level: config.operator_user_permission_level,
                view_distance: config.chunk_radius,
                simulation_distance: config.simulation_distance,
                hide_online_players: config.hide_online_players,
                status_replies: config.status_replies,
                accepts_transfers: config.accepts_transfers,
                allow_flight: config.allow_flight,
                pause_when_empty_seconds: config.pause_when_empty_seconds,
                entity_broadcast_range_percentage: config.entity_broadcast_range_percentage,
            },
            config.admin_lists.clone(),
            &enabled_features
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );
        let default_game_type = config.game_type;
        let damage_types = load_damage_type_registry(config.datapacks.as_deref());
        let mob_spawns = natural_spawn::load_mob_spawns(config.datapacks.as_deref())?;
        let dimension_sky_lights =
            environment_attributes::load_sky_lights(config.datapacks.as_deref())?;
        let content_registry = registries.content_registry();
        let advancement_tree = config
            .datapacks
            .as_deref()
            .map(|datapacks| {
                AdvancementTree::from_loaded_with_content_registry(
                    &datapacks.advancements,
                    content_registry,
                )
            })
            .unwrap_or_default();
        let advancement_triggers = advancement_triggers::AdvancementTriggers::compile(
            &advancement_tree,
            config.datapacks.as_deref(),
        );
        let structure_templates =
            structure_templates::StructureTemplateManager::new_with_content_registry(
                config.generated_structure_directory.clone(),
                Arc::clone(registries.content_registry_arc()),
            );
        let loot_item_tags = config
            .datapacks
            .as_deref()
            .and_then(|packs| packs.tags.registry("minecraft:item"))
            .map(|tags| ItemTagIndex::from_loaded_with_content_registry(tags, content_registry))
            .unwrap_or_default();
        let fuel_values =
            bbb_server_datapack::recipe::FuelValues::vanilla_burn_times_with_content_registry(
                &loot_item_tags,
                content_registry,
            )
            .values;
        let crafting_item_tags = Self::crafting_item_tags_of(config.datapacks.as_deref());
        let mut world = Self {
            tick: 0,
            game_time: 0,
            tick_rate: TickRateManager::default(),
            pending_removal_effects: Vec::new(),
            deferred_game_event_events: Vec::new(),
            pending_camera_transitions: Vec::new(),
            pending_disconnects: Vec::new(),
            last_command_sequence: 0,
            datapack_tables,
            entity_ids,
            next_teleport_id: 1,
            stopping: false,
            light_failed: false,
            random,
            use_item_on_server_swing: false,
            weather: Java26WeatherData::default(),
            scoreboard: server_scoreboard::ServerScoreboard::with_content_registry(Arc::clone(
                registries.content_registry_arc(),
            )),
            maps: ServerMapStore::default(),
            levels,
            published_player_list_revision: 0,
            registries,
            game_rules: SimulationGameRules::vanilla_defaults(),
            feature_flags,
            world_clocks,
            ticks_until_autosave: AUTOSAVE_INTERVAL_TICKS,
            auto_save: true,
            empty_ticks: 0,
            admin,
            sessions: FxHashMap::default(),
            cold: Box::new(WorldCold {
                config,
                fixture,
                tick_times_nanos: [0; TICK_STATS_SPAN],
                aggregated_tick_times_nanos: 0,
                pending_tick_time_sample: None,
                pending_sprint_report: None,
                debug_profiler_delay_start: false,
                debug_profiler: None,
                damage_types,
                loot_item_tags,
                fuel_values,
                crafting_item_tags,
                // Filled a few lines below `new` returns: compiling the library
                // needs the projection only a built world can make, which is the
                // same order vanilla builds them in -- `ReloadableServerResources`
                // is loaded before `MinecraftServer` constructs the manager over it.
                functions: server_functions::ServerFunctionManager::default(),
                mob_spawns,
                dimension_sky_lights,
                random_sequences: RefCell::new(RandomSequences::new()),
                wandering_trader: wandering_trader::WanderingTraderSpawner::default(),
                village_siege: village_siege::VillageSiege::default(),
                patrol_spawner: patrol_spawner::PatrolSpawner::default(),
                phantom_spawner: phantom_spawner::PhantomSpawner::default(),
                cat_spawner: cat_spawner::CatSpawner::default(),
                boss_events: boss_events::ServerBossEvents::default(),
                command_storage: command_storage::ServerCommandStorage::default(),
                structure_templates,
                unserved_levels: BTreeMap::new(),
                level_dimension_types,
                infiniburn_cache: simulation_tick::InfiniburnCache::default(),
                difficulty_locked: false,
                hardcore: false,
                default_game_type,
                player_idle_timeout,
                stopwatches: stopwatches::Stopwatches::default(),
                scheduled_events: scheduled_events::ScheduledEvents::default(),
                seed: generation_seed,
                generate_structures,
                default_spawn: WorldSpawn {
                    dimension: LevelId::from(Java26BuiltinDimension::Overworld),
                    position: Vec3d {
                        x: SPAWN_X,
                        y: SPAWN_Y,
                        z: SPAWN_Z,
                    },
                    rotation: PlayerRotation {
                        y_rot: 0.0,
                        x_rot: 0.0,
                    },
                },
                wall_clock_millis: 0,
                debug_subscriptions: ServerDebugSubscriptions::default(),
                restored_players: BTreeMap::new(),
                restored_player_stats: BTreeMap::new(),
                restored_player_advancements: BTreeMap::new(),
                stored_data_reader: None,
                stored_players: BTreeSet::new(),
                stored_player_stats: BTreeSet::new(),
                stored_player_advancements: BTreeSet::new(),
                advancement_tree,
                advancement_triggers,
                departed_player_snapshots: BTreeMap::new(),
                restored_warden_trackers: BTreeMap::new(),
                autosave_staging: None,
                #[cfg(test)]
                publication: Arc::new(WorldSnapshot {
                    tick: 0,
                    stopping: false,
                    changed_blocks: Vec::new(),
                    players: Vec::new(),
                }),
            }),
        };
        if let Some(datapacks) = world.cold.config.datapacks.clone() {
            world.replace_function_library(&datapacks.functions);
        }
        Ok(world)
    }

    fn has_advancement_manager(&self) -> bool {
        self.cold.config.datapacks.is_some()
    }

    /// The `Level.gameEvent` calls one level raised, oldest first, and how many
    /// it has raised in total.
    ///
    #[must_use]
    pub fn game_events(&self, dimension: Java26BuiltinDimension) -> (Vec<RecordedGameEvent>, u64) {
        let sink = &self.level(dimension).event_sink;
        (
            sink.recent_game_events().copied().collect(),
            sink.game_event_count(),
        )
    }

    fn overworld(&self) -> &ServerLevel {
        self.level(Java26BuiltinDimension::Overworld)
    }

    /// The block range one level's columns cover, as `(min_y, section_count)`.
    #[cfg(test)]
    pub(crate) fn level_column_shape(&self, dimension: Java26BuiltinDimension) -> (i32, usize) {
        let chunks = &self.level(dimension).chunks;
        (chunks.min_y(), chunks.section_count())
    }

    #[cfg(test)]
    pub(crate) fn level_sea_level(&self, dimension: Java26BuiltinDimension) -> i32 {
        self.level(dimension).chunks.sea_level()
    }

    pub fn tick(&mut self, commands: Vec<CommandEnvelope>) -> TickResult {
        if self.light_failed {
            return self.finish_tick(0, Vec::new());
        }
        let mut events = Vec::new();
        if self.paused_while_empty(&mut events) {
            let processed_commands = commands.len();
            self.apply_commands(commands, &mut events);
            return self.finish_tick(processed_commands, events);
        }
        self.promote_debug_time_profiler();
        self.tick = self.tick.wrapping_add(1);
        self.tick_rate.tick();
        let processed_commands = commands.len();
        self.flush_camera_transitions(&mut events);
        if let Some(report) = self.cold.pending_sprint_report.take() {
            self.report_sprint(report, &mut events);
        }
        if let Some(sample) = self.cold.pending_tick_time_sample.take() {
            self.send_tick_time_sample(sample, &mut events);
        }
        self.apply_commands(commands, &mut events);
        self.run_server_tasks();
        self.tick_command_functions(&mut events);
        self.flush_pending_disconnects(&mut events);
        // A tick function can reach `/setblock` or `/fill`, including shape
        // cascades that create item entities. It is a separate owner boundary
        // from the mailbox above, so publish its journal before clocks and
        // levels advance.
        self.apply_all_level_entity_lifecycle(&mut events);
        if self.tick_rate.runs_normally() {
            self.tick_world_clocks();
        }
        if self.tick.is_multiple_of(TIME_SYNC_INTERVAL_TICKS) {
            self.broadcast(
                ClientboundPlayPacket::SetTime(PlayTime {
                    game_time: self.game_time(),
                    clock_updates: Vec::new(),
                }),
                &mut events,
            );
        }
        for player in self.levels.players_mut() {
            player.block_contact.old_position = player.position();
        }
        self.tick_world_simulation(&mut events);
        self.flush_pending_disconnects(&mut events);
        if self.light_failed {
            return self.finish_tick(processed_commands, events);
        }
        self.tick_block_change_acks(&mut events);
        self.tick_chunk_batches(&mut events);
        self.tick_keep_alives(&mut events);
        self.tick_player_latency_broadcast(&mut events);
        self.tick_idle_timeout(&mut events);
        self.tick_player_floating(&mut events);
        self.tick_external_player_vehicle_listeners(&mut events);
        self.tick_players(&mut events);
        self.flush_pending_disconnects(&mut events);
        self.tick_player_food(&mut events);
        self.flush_pending_disconnects(&mut events);
        self.tick_player_map_items_and_special_updates(&mut events);
        self.tick_player_health(&mut events);
        self.flush_pending_disconnects(&mut events);
        self.tick_player_score_criteria(&mut events);
        self.tick_player_experience(&mut events);
        self.tick_player_advancement_criteria(&mut events);
        self.flush_pending_disconnects(&mut events);
        self.tick_player_advancement_flush(&mut events);
        self.tick_container_menus(&mut events);
        self.flush_pending_disconnects(&mut events);
        self.refresh_debug_subscriptions();
        self.tick_autosave(&mut events);
        self.finish_tick(processed_commands, events)
    }

    fn apply_commands(&mut self, commands: Vec<CommandEnvelope>, events: &mut Vec<WorldEvent>) {
        for envelope in commands {
            assert!(
                envelope.sequence > self.last_command_sequence,
                "world command sequence must be strictly increasing"
            );
            self.last_command_sequence = envelope.sequence;
            self.apply_command(envelope.command, events);
            self.flush_pending_disconnects(events);
            // Packet handlers and administration commands run on the server
            // owner before `tickChildren`. Vanilla entity callbacks publish
            // synchronously from those mutations; draining every level here
            // keeps one mailbox command's additions/removals from crossing
            // the next command boundary.
            self.apply_all_level_entity_lifecycle(events);
        }
    }

    /// `MinecraftServer.tickServer`'s empty-server pause (`:954-972`): once
    /// nobody has been online for the configured time, the tick only runs the
    /// connections and the console, which here are the mailbox commands. The
    /// tick that starts the pause saves once.
    fn paused_while_empty(&mut self, events: &mut Vec<WorldEvent>) -> bool {
        let seconds = self.admin.settings().pause_when_empty_seconds;
        let threshold = seconds.saturating_mul(20);
        if threshold <= 0 {
            return false;
        }
        if self.levels.player_count() == 0 && !self.tick_rate.is_sprinting() {
            self.empty_ticks = self.empty_ticks.saturating_add(1);
        } else {
            self.empty_ticks = 0;
        }
        if self.empty_ticks < threshold {
            return false;
        }
        if self.empty_ticks == threshold {
            tracing::info!(event = "world.paused_when_empty", seconds);
            self.ticks_until_autosave = self.compute_next_autosave_interval();
            if self.auto_save {
                events.push(WorldEvent::SaveRequested {
                    flush: false,
                    trigger: SaveTrigger::Autosave,
                });
            }
        }
        true
    }

    fn finish_tick(&mut self, processed_commands: usize, events: Vec<WorldEvent>) -> TickResult {
        #[cfg(test)]
        self.publish();
        let player_list_revision = self.levels.player_list_revision();
        let online_players =
            (player_list_revision != self.published_player_list_revision).then(|| {
                self.published_player_list_revision = player_list_revision;
                // MinecraftServer.java:217,1076: a player whose client opts out of
                // listing shows as the anonymous profile.
                self.levels
                    .players()
                    .map(|player| {
                        if player.cold.client_information.allows_listing {
                            (player.profile.uuid, player.profile.name.clone())
                        } else {
                            (Uuid::nil(), "Anonymous Player".to_string())
                        }
                    })
                    .collect()
            });
        TickResult {
            tick: self.tick,
            processed_commands,
            events,
            online_players,
            online_player_count: self.levels.player_count(),
            #[cfg(test)]
            snapshot: Arc::clone(&self.cold.publication),
        }
    }

    ///
    /// `autoSave` calls `saveEverything(true, false, false)`, so the unattended
    /// save is the silent, unflushed, unforced one; `flush` is false here for
    /// that reason. The runtime's single save writer serializes this request
    /// behind any autosave still being written, which is Vanilla's `isSaving`
    /// guard on this side.
    ///
    fn tick_autosave(&mut self, events: &mut Vec<WorldEvent>) {
        self.ticks_until_autosave -= 1;
        if self.ticks_until_autosave > 0 {
            return;
        }
        self.ticks_until_autosave = self.compute_next_autosave_interval();
        if !self.auto_save {
            return;
        }
        events.push(WorldEvent::SaveRequested {
            flush: false,
            trigger: SaveTrigger::Autosave,
        });
    }

    ///
    /// A sprinting server has no meaningful rate to read, so vanilla estimates
    /// one from how long its ticks are actually taking (`:1030-1032`) — which
    /// is why the ring [`Self::record_tick_time`] fills is not only `/tick
    /// query`'s.
    fn compute_next_autosave_interval(&self) -> i32 {
        let ticks_per_second = if self.tick_rate.is_sprinting() {
            let estimated_tick_time_nanos = self.average_tick_time_nanos().saturating_add(1);
            NANOSECONDS_PER_SECOND / estimated_tick_time_nanos as f32
        } else {
            self.tick_rate.tickrate()
        };
        MINIMUM_AUTOSAVE_TICKS.max((ticks_per_second * AUTOSAVE_INTERVAL_SECONDS) as i32)
    }

    fn on_tick_rate_changed(&mut self) {
        let next = self.compute_next_autosave_interval();
        if next < self.ticks_until_autosave {
            self.ticks_until_autosave = next;
        }
    }

    /// How many ticks are left before the unattended save, which is the state
    /// a test reads to prove the counter is the thing driving the save.
    pub fn ticks_until_autosave(&self) -> i32 {
        self.ticks_until_autosave
    }

    #[cfg(test)]
    pub fn publication(&self) -> Arc<WorldSnapshot> {
        Arc::clone(&self.cold.publication)
    }

    pub fn fixture(&self) -> &FlatFixture {
        &self.cold.fixture
    }

    pub fn game_time(&self) -> i64 {
        self.game_time
    }

    pub fn tick_rate(&self) -> &TickRateManager {
        &self.tick_rate
    }

    ///
    /// `smoothedTickTimeMillis` (`:995`) is not kept: its only readers in
    /// vanilla are the JFR profiler (`:774`) and `getCurrentSmoothedTickTime`
    /// (`:1795-1797`), neither of which this server has.
    pub fn record_tick_time(&mut self, tick_time_nanos: i64) {
        let index = (self.tick as usize) % TICK_STATS_SPAN;
        self.cold.aggregated_tick_times_nanos -= self.cold.tick_times_nanos[index];
        self.cold.aggregated_tick_times_nanos += tick_time_nanos;
        self.cold.tick_times_nanos[index] = tick_time_nanos;
    }

    /// `RemoteSampleLogger.logFullSample` for `TICK_TIME`: full tick, tick
    /// method, scheduled tasks and idle, in `TpsDebugDimensions` order
    /// (`MinecraftServer.java:800-821`, `:1022-1026`).
    pub fn log_tick_time_sample(&mut self, sample: [i64; 4]) {
        self.cold.pending_tick_time_sample = Some(sample);
    }

    pub fn average_tick_time_nanos(&self) -> i64 {
        let filled = i64::try_from(TICK_STATS_SPAN)
            .expect("the span is a small constant")
            .min(i64::try_from(self.tick).unwrap_or(i64::MAX).max(1));
        self.cold.aggregated_tick_times_nanos / filled
    }

    ///
    /// A sprint that ends here is not announced from this call: vanilla
    /// announces it from inside `finishTickSprint`, which runs before
    /// `processPacketsAndTick`, so the report is held for the next `tick` to
    /// emit in the order the clients would have seen it.
    pub fn check_should_sprint_this_tick(&mut self, now_nanos: i64) -> bool {
        if !self.tick_rate.is_sprinting() {
            return false;
        }
        let (sprinting, report) = self.tick_rate.check_should_sprint_this_tick(now_nanos);
        if let Some(report) = report {
            self.cold.pending_sprint_report = Some(report);
        }
        sprinting
    }

    pub fn end_tick_work(&mut self, now_nanos: i64) {
        self.tick_rate.end_tick_work(now_nanos);
    }

    fn broadcast_ticking_state(&self, events: &mut Vec<WorldEvent>) {
        let packet = ClientboundPlayPacket::TickingState(TickingState {
            tick_rate: self.tick_rate.tickrate(),
            frozen: self.tick_rate.is_frozen(),
        });
        crate::world::player_packets::broadcast_to(
            self.levels.player_keys().copied(),
            packet,
            events,
        );
    }

    fn broadcast_ticking_step(&self, events: &mut Vec<WorldEvent>) {
        let packet = ClientboundPlayPacket::TickingStep(TickingStep {
            tick_steps: self.tick_rate.frozen_ticks_to_run(),
        });
        crate::world::player_packets::broadcast_to(
            self.levels.player_keys().copied(),
            packet,
            events,
        );
    }

    fn report_sprint(
        &mut self,
        report: crate::tick_rate::SprintReport,
        events: &mut Vec<WorldEvent>,
    ) {
        events.push(WorldEvent::ConsoleMessage(commands::sprint_report(
            report.ticks_per_second,
            &report.milliseconds_per_tick,
        )));
        self.broadcast_ticking_state(events);
        // `finishTickSprint` closes on `onTickRateChanged` (`:94`), after the
        // manager has already put the frozen flag back, so the interval it
        // computes is the one the server is about to run at rather than the
        // sprint's.
        self.on_tick_rate_changed();
    }

    /// `ServerTickRateManager.setFrozen` (ServerTickRateManager.java:28-31).
    pub(crate) fn set_tick_frozen(&mut self, frozen: bool, events: &mut Vec<WorldEvent>) {
        self.tick_rate.set_frozen(frozen);
        self.broadcast_ticking_state(events);
    }

    /// `ServerTickRateManager.setTickRate` (ServerTickRateManager.java:115-119),
    /// which runs `onTickRateChanged` between the write and the broadcast.
    pub(crate) fn set_tick_rate(&mut self, rate: f32, events: &mut Vec<WorldEvent>) {
        self.tick_rate.set_tick_rate(rate);
        self.on_tick_rate_changed();
        self.broadcast_ticking_state(events);
    }

    /// `ServerTickRateManager.stepGameIfPaused` (ServerTickRateManager.java:41-49).
    pub(crate) fn step_ticks(&mut self, ticks: i32, events: &mut Vec<WorldEvent>) -> bool {
        let stepped = self.tick_rate.step_game_if_paused(ticks);
        if stepped {
            self.broadcast_ticking_step(events);
        }
        stepped
    }

    /// `ServerTickRateManager.stopStepping` (ServerTickRateManager.java:51-59).
    pub(crate) fn stop_stepping(&mut self, events: &mut Vec<WorldEvent>) -> bool {
        let stopped = self.tick_rate.stop_stepping();
        if stopped {
            self.broadcast_ticking_step(events);
        }
        stopped
    }

    /// `ServerTickRateManager.requestGameToSprint` (ServerTickRateManager.java:70-78),
    /// which unfreezes and therefore tells the clients. `true` is a sprint that
    /// was already running.
    pub(crate) fn sprint_ticks(&mut self, ticks: i32, events: &mut Vec<WorldEvent>) -> bool {
        let interrupted = self.tick_rate.request_game_to_sprint(ticks);
        self.broadcast_ticking_state(events);
        interrupted
    }

    /// `ServerTickRateManager.stopSprinting` (ServerTickRateManager.java:61-68).
    pub(crate) fn stop_sprinting(&mut self, events: &mut Vec<WorldEvent>) -> bool {
        let Some(report) = self.tick_rate.stop_sprinting() else {
            return false;
        };
        self.report_sprint(report, events);
        true
    }

    #[cfg(test)]
    pub(crate) fn tick_rate_mut(&mut self) -> &mut TickRateManager {
        &mut self.tick_rate
    }

    /// The live store/tracker counts for a level. This is a test-only view of
    /// the owner-boundary invariant: every restored store entry must already
    /// have a `ChunkMap` tracker before the first running boundary.
    #[cfg(test)]
    pub(crate) fn entity_tracking_counts(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> (usize, usize) {
        self.level_if_present(dimension)
            .map(|level| (level.entities.len(), level.entity_tracker.len()))
            .unwrap_or_default()
    }

    /// The datapack surface loaded at startup, when the runtime wired one in.
    pub fn datapacks(&self) -> Option<&bbb_server_datapack::startup::DatapackStartup> {
        self.cold.config.datapacks.as_deref()
    }

    /// The installed generation's item tags, or `None` without an item tag
    /// registry, where an invented empty index would refuse tagged ingredients.
    pub(crate) fn item_tags(&self) -> Option<&ItemTagIndex> {
        self.datapacks()?.tags.registry("minecraft:item")?;
        Some(&self.cold.loot_item_tags)
    }

    /// The permissions a `.mcfunction` line is parsed with.
    pub(crate) fn function_compilation_permission(&self) -> CommandPermission {
        self.cold.config.function_permission_level
    }

    pub fn datapacks_shared(&self) -> Option<Arc<bbb_server_datapack::startup::DatapackStartup>> {
        self.cold.config.datapacks.clone()
    }

    pub(crate) fn install_level_registry_tags(&mut self, registry_tags: Arc<RegistryTagSnapshot>) {
        for level in self.levels.values_mut() {
            level
                .chunks
                .install_registry_tags(Arc::clone(&registry_tags));
        }
    }

    /// The live management settings. The runtime reads them to build the status
    /// response and to decide when to rewrite the stored user lists, which is
    /// why they are published rather than kept private.
    pub fn admin_settings(&self) -> &AdminSettings {
        self.admin.settings()
    }

    /// The stored user list revision, which changes exactly when one of the
    /// four documents needs rewriting.
    pub fn admin_list_revision(&self) -> u64 {
        self.admin.list_revision()
    }

    /// Bumps whenever a setting projected by [`Self::persisted_settings`] changes.
    pub fn admin_settings_revision(&self) -> (u64, i32) {
        (
            self.admin.settings_revision(),
            self.cold.player_idle_timeout,
        )
    }

    /// The four documents as the runtime should write them.
    pub fn admin_documents(&self) -> crate::admin::StoredUserListDocuments {
        self.admin.documents()
    }

    /// The login check the login phase runs before configuration.
    pub fn admission_snapshot(&self) -> crate::admin::AdmissionSnapshot {
        self.admin.admission_snapshot(self.levels.player_count())
    }

    /// The `server.properties` values a running server rewrites, as the
    /// runtime saves them.
    pub fn persisted_settings(&self) -> crate::admin::PersistedSettings {
        let settings = self.admin.settings();
        crate::admin::PersistedSettings {
            white_list: Some(settings.use_allowlist),
            enforce_whitelist: Some(settings.enforce_allowlist),
            player_idle_timeout: Some(self.cold.player_idle_timeout),
            motd: Some(settings.motd.clone()),
            max_players: Some(settings.max_players),
            difficulty: Some(self.admin.saved_difficulty()),
            gamemode: Some(self.admin.saved_game_mode()),
            spawn_protection: Some(settings.spawn_protection_radius),
            op_permission_level: Some(settings.operator_user_permission_level),
            view_distance: Some(settings.view_distance),
            simulation_distance: Some(settings.simulation_distance),
            hide_online_players: Some(settings.hide_online_players),
            enable_status: Some(settings.status_replies),
            accepts_transfers: Some(settings.accepts_transfers),
            allow_flight: Some(settings.allow_flight),
            pause_when_empty_seconds: Some(settings.pause_when_empty_seconds),
            entity_broadcast_range_percentage: Some(settings.entity_broadcast_range_percentage),
        }
    }

    ///
    /// Vanilla's `EntityCreationType.IMMEDIATE` (`:699`) builds the object on
    /// demand from the block standing there. This world builds it at the two
    /// places a block entity can come into existence — the block write and the
    /// save restore — so there is nothing left for a lazy build to do.
    pub(crate) fn get_block_entity(
        &self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> Option<&BlockEntity> {
        self.level_by_id(&dimension.into())
            .chunks
            .block_entities()
            .get(pos)
    }

    #[cfg(test)]
    pub(crate) fn get_block_entity_by_key(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> Option<&BlockEntity> {
        self.level_by_id(dimension).chunks.block_entities().get(pos)
    }

    pub(crate) fn block_state(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
    ) -> Option<i32> {
        self.block_state_by_key(&dimension.into(), pos)
    }

    pub(crate) fn block_state_by_key(&mut self, dimension: &LevelId, pos: BlockPos) -> Option<i32> {
        if !self.is_in_world_bounds_by_key(dimension, pos) {
            return None;
        }
        self.install_full_chunk_for_read(dimension, (pos.x >> 4, pos.z >> 4))
            .ok()?;
        let state = self
            .level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z);
        self.reconcile_saved_column_accesses(dimension);
        state
    }

    fn apply_command(&mut self, command: WorldCommand, events: &mut Vec<WorldEvent>) {
        match command {
            WorldCommand::Join {
                connection_id,
                profile,
                client_information,
                context,
            } => self.join(connection_id, profile, client_information, context, events),
            WorldCommand::PlayPacket {
                connection_id,
                packet_id,
                payload,
            } => match decode_play_serverbound_with_content_registry(
                packet_id,
                &payload,
                self.cold.config.plugins.content_registry(),
            ) {
                Ok(packet) => self.apply_player_packet(connection_id, packet, events),
                Err(error) => {
                    tracing::warn!(
                        event = "connection.play_decode_failed",
                        connection_id = connection_id.0,
                        packet_id,
                        %error,
                    );
                    self.remove_player(connection_id, events);
                    events.push(WorldEvent::Close {
                        connection_id,
                        reason: WorldCloseReason::PacketOutsideActiveContract,
                    });
                }
            },
            #[cfg(test)]
            WorldCommand::Play {
                connection_id,
                packet,
            } => self.apply_player_packet(connection_id, packet, events),
            WorldCommand::Disconnect { connection_id } => {
                self.remove_player(connection_id, events);
            }
            WorldCommand::Admin { envelope, request } => {
                self.apply_admin(envelope, request, events);
            }
            WorldCommand::Datapacks { startup } => self.install_datapacks(startup, events),
            WorldCommand::Shutdown => self.shutdown(events),
        }
    }

    fn apply_player_packet(
        &mut self,
        connection_id: ConnectionId,
        packet: PlayServerbound,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.is_pending_disconnect(connection_id) {
            return;
        }
        let activity = idle_timeout::packet_activity(&packet);
        let before = self
            .levels
            .player(connection_id)
            .map(|player| player.position());
        self.apply_play(connection_id, packet, events);
        self.note_player_activity(connection_id, activity, before);
    }

    fn shutdown(&mut self, events: &mut Vec<WorldEvent>) {
        self.stopping = true;
        let connection_ids = self.levels.player_keys().copied().collect::<Vec<_>>();
        for &connection_id in &connection_ids {
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::ServerShutdown,
            });
        }
        for connection_id in connection_ids {
            self.remove_player_for_shutdown(connection_id);
        }
        self.sessions.clear();
    }

    #[cfg(test)]
    fn publish(&mut self) {
        let players = self
            .levels
            .player_entries()
            .map(|(&connection_id, player)| {
                Ok(PlayerSnapshot {
                    connection_id,
                    uuid: player.profile.uuid,
                    name: player.profile.name.clone(),
                    entity_id: player.entity_id(),
                    position: player.position(),
                    rotation: player.rotation(),
                    on_ground: player.on_ground(),
                    client_loaded: player.client_loaded(),
                    inventory: InventorySnapshot {
                        selected_slot: player.inventory.selected_slot(),
                        slots: player
                            .inventory
                            .inventory_slots()
                            .iter()
                            .map(|stack| stack.summary().clone())
                            .collect(),
                    },
                })
            })
            .collect::<bbb_server_types::Result<Vec<_>>>();
        let mut players = match players {
            Ok(players) => players,
            Err(error) => {
                tracing::warn!("world inventory snapshot publication: {error}");
                return;
            }
        };
        players.sort_unstable_by_key(|player| player.connection_id);
        // BlockSnapshot has no dimension field, so only overworld edits are represented.
        let mut changed_blocks = self
            .overworld()
            .chunks
            .changed_blocks()
            .map(|((x, y, z), block_state_id)| BlockSnapshot {
                pos: BlockPos { x, y, z },
                block_state_id,
            })
            .collect::<Vec<_>>();
        changed_blocks.sort_unstable_by_key(|block| (block.pos.x, block.pos.y, block.pos.z));
        self.cold.publication = Arc::new(WorldSnapshot {
            tick: self.tick,
            stopping: self.stopping,
            changed_blocks,
            players,
        });
    }
}

#[cfg(test)]
pub(crate) use self::command_reads::CommandTestWorld;
