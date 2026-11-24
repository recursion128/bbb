//! One `ServerLevel` (`net/minecraft/server/level/ServerLevel.java`): the
//! dimension-wide state a level owns, the generation view it was handed at
//! construction, and the source it generates columns from.

use super::*;

use crate::chunk::generation_publication::GenerationItemPublication;

use super::registry_runtime::PlayerItemStackResolver;

#[derive(Clone)]
pub(super) enum ServerLevelSource {
    Flat(FlatLevelSource),
    Debug(DebugLevelSource),
    Noise {
        program: Arc<NoiseFinalDensityProgram>,
        generate_structures: bool,
    },
}

/// One `ServerLevel` (`net/minecraft/server/level/ServerLevel.java`): the
/// dimension-wide state a level owns, as opposed to the server-wide state
/// [`ServerWorld`] keeps beside it.
///
/// Everything here is per level in vanilla, so a server running three of them
/// holds three of each. What is deliberately *not* here is what
/// `MinecraftServer` and `WorldData` own once for the whole server: the tick
/// counter, the tick-rate manager, the game rules, the difficulty, the game
/// type, the player list and the save's own `level.dat` state. A
/// `DerivedLevelData` (`MinecraftServer.java:466`) is exactly that split:
/// every non-overworld level reads the overworld's `ServerLevelData` rather
/// than carrying one.
pub(super) struct ServerLevel {
    /// `ServerLevel.dimension()`, the `ResourceKey<Level>` this level is filed
    /// under. It is also the save directory the level reads and writes
    /// (`DimensionType.getStorageFolder`, `DimensionType.java:115-117`).
    pub(super) dimension: std::sync::Arc<Java26LevelKey>,
    pub(super) world_border: WorldBorder,
    /// Per-level SavedData (`ServerLevel.java:275`).
    pub(super) raids: raids::LevelRaids,
    /// `ServerLevel.dragonFight` (`ServerLevel.java:296-299`), made on the
    /// first tick when the save filed none.
    pub(super) dragon_fight: Option<dragon_fight::DragonFight>,
    pub(super) spawn_facts: spawn::SpawnFacts,
    /// `ServerLevel.players` insertion order (`ServerLevel.java:1910-1914`,
    /// `:1945-1949`), observed by framed-map sends.
    pub(super) player_order: Vec<ConnectionId>,
    pub(super) player_chunk_membership: super::player_tracking::PlayerChunkMembership,
    /// Authoritative block storage backed by this level's own stem generator.
    /// A column is generated the first time anything asks for it, and saved
    /// sections replace the generated ones on restore.
    pub(super) chunks: ChunkStore,
    /// The chunks of this level the loaded save already holds a document for,
    /// which is what decides whether a changed chunk is saved as a patch over
    /// that document or written entire.
    pub(super) saved_chunks: BTreeSet<ChunkKey>,
    /// The chunks whose saved document already carried a pending tick, which
    /// have to be written back even when they hold none any more.
    pub(super) saved_tick_chunks: BTreeSet<ChunkKey>,
    /// The chunks the save already holds an entity-region document for, for the
    /// same reason.
    pub(super) saved_entity_chunks: BTreeSet<ChunkKey>,
    /// The run state of `ServerLevel.blockTicks`; the pending ticks live in
    /// the chunks that [`ChunkStore::register_ticks`] registered.
    pub(super) block_ticks: LevelTicks<BlockTickType>,
    pub(super) fluid_ticks: LevelTicks<FluidTickType>,
    /// `ServerLevel.blockEvents` (`ServerLevel.java:213`).
    pub(super) block_events: BlockEventQueue,
    /// The persisted half of `TicketStorage` (`TicketStorage.packTickets`,
    /// `TicketStorage.java:71-79`) for this level: the columns the save
    /// declares, at the level they were filed at.
    ///
    /// `/forceload` and portal arrival are the writers this server has for it
    /// (see [`ServerWorld::set_chunks_forced`] and
    /// [`ServerWorld::place_portal_ticket`]). Everything else only ever
    /// removes, and only by a timed ticket running out
    /// (`TicketStorage.purgeStaleTickets`, `TicketStorage.java:279-289`). See
    /// [`ServerWorld::purge_stale_tickets`].
    pub(super) saved_chunk_tickets: Vec<SavedChunkTicket>,
    /// `ServerChunkCache.chunkMap.distanceManager.simulationChunkTracker`
    /// (`DistanceManager.java:39`), refreshed where vanilla's
    /// `runDistanceManagerUpdates` refreshes it. See
    /// [`ServerWorld::refresh_simulation_chunk_levels`].
    pub(super) simulation_levels: SimulationChunkLevels,
    pub(super) chunk_lifecycle: super::chunk_lifecycle::ChunkLifecycle,
    pub(super) neighbor_updater: NeighborUpdates,
    /// The live half of `ServerLevel.entityManager` (`ServerLevel.java:255`):
    /// every non-player entity in the level.
    pub(super) entities: EntityStore,
    /// `ServerLevel.getEntityInAnyDimension`'s other levels
    /// (`ServerLevel.java:1384-1391`) and the sessions
    /// `getPlayerInAnyDimension` answers (`:1397-1399`); see
    /// [`ServerWorld::refresh_cross_level_entities_by_id`].
    pub(super) cross_level_entities: CrossLevelEntities,
    /// `ServerChunkCache.chunkMap`'s `entityMap` (`ChunkMap.java:146`).
    pub(super) entity_tracker: EntityTracker,
    pub(super) debug_subscriptions: LevelDebugSubscriptions,
    pub(super) waypoint_manager: waypoints::ServerWaypointManager,
    pub(super) boss_bar_entities: BTreeSet<i32>,
    pub(super) boss_bar_entities_seeded: bool,
    /// `Level.subTickCount` (`Level.java:132`), handed out by
    /// `Level.nextSubTickCount` (`:1081-1083`).
    pub(super) sub_tick_count: i64,
    /// The world's gameplay stream.
    pub(super) random: GameRandom,
    /// `ChunkHolder.blockChanged` (`ServerLevel.sendBlockUpdated`,
    /// `ServerLevel.java:1119-1123`): the positions whose new state the
    /// tracking clients have not been told about yet. Vanilla flushes them in
    /// `ServerChunkCache.broadcastChangedChunks`, which runs inside
    /// `ServerLevel.tick` between the scheduled ticks and the block events.
    pub(super) changed_block_broadcasts: BTreeSet<(i32, i32, i32)>,
    /// `Level.levelEvent` and `Level.gameEvent` for this level: the two
    /// broadcasts a mechanism raises and a level slice cannot deliver, because
    /// their recipients are sessions. See [`crate::level_event`].
    pub(super) event_sink: LevelEventSink,
    /// `ServerPlayer.awardKillScore`'s own criterion (`ServerPlayer.java:955`)
    /// for a stored death, held between the flush that can still read the
    /// corpse and the drain that owns this session's packets.
    pub(super) pending_kill_credits: Vec<combat::PendingKillCredit>,
    /// `RedstoneTorchBlock.RECENT_TOGGLES` for this level
    /// (`RedstoneTorchBlock.java:27`).
    ///
    /// Vanilla files it in a `static WeakHashMap<Level, List<Toggle>>`, i.e.
    /// one list per level that the garbage collector drops with the level; a
    /// field of the level is the same lifetime spelled without a weak
    /// reference. **It is deliberately not persisted**, which is not a gap:
    /// the vanilla map is a static of the block class and no `SavedData`, chunk
    /// document or `level.dat` field carries it either, so a restarted official
    /// server also starts every torch's burnout history empty.
    pub(super) recent_torch_toggles: Vec<RedstoneTorchToggle>,
    /// `Level`'s own half of the weather cycle (`Level.java:913-931`); the
    /// timers it advances are [`ServerWorld::weather`], one store for the save.
    pub(super) weather: weather::LevelWeather,
    pub(super) sky_darkening: Option<u8>,
    pub(super) sleep_status: super::player_sleep::SleepStatus,
}

impl ServerLevel {
    pub(super) fn block_state(&self, pos: bbb_protocol::packets::BlockPos) -> i32 {
        super::stored_block_state(&self.chunks, pos)
    }

    /// An unloaded level for one stem; saved tickets, tracking views and block
    /// access load columns (`MinecraftServer.java:430-432`, `:467-469`, `:543-553`).
    pub(super) fn new(
        dimension: Java26LevelKey,
        source: ServerLevelSource,
        spawn_facts: spawn::SpawnFacts,
        registries: &SynchronizedRegistries,
        datapacks: Option<&Arc<bbb_server_datapack::startup::DatapackStartup>>,
        entity_ids: EntityIdCounter,
        random: GameRandom,
    ) -> Result<Self> {
        let mut chunks = match source {
            ServerLevelSource::Flat(source) => {
                let biome_registry_len = registries
                    .registry_len(BIOME_REGISTRY)
                    .expect("biome registry");
                let biome_id = registries
                    .require_registry_element_index(BIOME_REGISTRY, source.settings().biome())
                    .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
                spawn::flat_chunks(source, biome_id, biome_registry_len, spawn_facts)
            }
            ServerLevelSource::Debug(source) => {
                debug_dimension::debug_chunks(source, registries, spawn_facts)?
            }
            ServerLevelSource::Noise {
                program,
                generate_structures,
            } => {
                // `ServerLevel.java:236` hands the level its registry access at
                // construction; generation reads it back at every block-entity
                // seam it runs (`StructureTemplate.java:305-306`).
                program.publish_item_registry(GenerationItemPublication::new(Arc::new(
                    PlayerItemStackResolver::new(datapacks, registries),
                )));
                ChunkStore::new_noise_carvers_with_structure_generation(
                    program,
                    generate_structures,
                )
                .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?
            }
        };
        chunks.install_registry_tags(registries.registry_tags_shared());
        let is_end = dimension.builtin() == Some(Java26BuiltinDimension::End);
        Ok(Self {
            dimension: std::sync::Arc::new(dimension),
            world_border: WorldBorder::DEFAULT,
            raids: Default::default(),
            dragon_fight: None,
            spawn_facts,
            player_order: Vec::new(),
            player_chunk_membership: Default::default(),
            chunks,
            saved_chunks: BTreeSet::new(),
            saved_tick_chunks: BTreeSet::new(),
            saved_entity_chunks: BTreeSet::new(),
            block_ticks: LevelTicks::new(),
            fluid_ticks: LevelTicks::new(),
            block_events: BlockEventQueue::new(),
            saved_chunk_tickets: Vec::new(),
            simulation_levels: SimulationChunkLevels::default(),
            chunk_lifecycle: Default::default(),
            neighbor_updater: NeighborUpdates::default(),
            entities: EntityStore::new_managed(random.clone(), entity_ids),
            cross_level_entities: CrossLevelEntities::default(),
            entity_tracker: super::registry_runtime::new_item_stack_tracker(registries),
            debug_subscriptions: LevelDebugSubscriptions::default(),
            waypoint_manager: waypoints::ServerWaypointManager::default(),
            boss_bar_entities: BTreeSet::new(),
            boss_bar_entities_seeded: false,
            sub_tick_count: 0,
            random,
            changed_block_broadcasts: BTreeSet::new(),
            event_sink: LevelEventSink::default(),
            pending_kill_credits: Vec::new(),
            recent_torch_toggles: Vec::new(),
            sky_darkening: None,
            sleep_status: Default::default(),
            weather: weather::LevelWeather::new(
                spawn_facts.has_skylight,
                spawn_facts.has_ceiling,
                is_end,
                weather::biome_climates(registries, datapacks.map(Arc::as_ref)),
            ),
        })
    }
}
