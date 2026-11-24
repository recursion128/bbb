use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use bbb_client_assets::KnownPackDataSource;
use bbb_protocol::content_registry::ContentRegistry;
use serde::{Deserialize, Serialize};

use crate::chunks::{RuntimeLightUpdateQueue, RuntimeLocalLightUpdates};
use crate::item_profiles::ItemProfiles;
use crate::terrain::TerrainChunkRenderRevisions;
use crate::{
    BlockChangedAckState, BlockDestructionProgress, BlockEventRecord, ChunkPos, ChunkViewState,
    ClientAdvancementsState, ClientAudioState, ClientChatState, ClientCombatState,
    ClientDebugGameState, ClientDebugQueryState, ClientEffectsState, ClientFeatureState,
    ClientHudState, ClientRecipesState, ClientStatsState, ClientToastState, ClientTutorialState,
    ClientUiState, ClientWaypointsState, CommandSuggestionsState, CommandTreeState, InventoryState,
    ItemCooldownState, LastMapColorPatchState, LevelEventRecord, LocalBlockPredictionState,
    LocalInteractionParticleAction, LocalPlayerState, MapItemState, PlayerInfoState,
    PlayerSocialState, ProjectilePowerUpdateState, PublicationShared, RegistrySet, ScoreboardState,
    ServerPresentationState, WorldApplyDiagnosticsState, WorldBorderState, WorldCounters,
    WorldDimension, WorldGameplayState, WorldLevelInfo, WorldLevelLoadState,
};

/// Exact runtime-only invalidation key for the block-entity projections whose
/// complete inputs are stable between render frames.
///
/// Terrain covers loaded block states and light. `block_entity_data` covers
/// accepted NBT updates and chunk loads before their deferred light applies.
/// Client-level identity prevents a numerically equal fresh level from
/// selecting an owner retained for the previous level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StableBlockEntityRenderRevision {
    pub client_level_generation: u64,
    pub terrain: u64,
    pub block_entity_data: u64,
}

#[derive(Debug)]
pub struct BlockEntityRenderPresence {
    pub item_displays: bool,
    pub chest_columns: Vec<ChunkPos>,
    pub spawner_columns: Vec<ChunkPos>,
    pub bell: bool,
    pub shulker_box: bool,
    pub decorated_pot: bool,
    pub banner: bool,
    pub enchanting_table: bool,
    pub conduit: bool,
    pub copper_golem_statue: bool,
    pub end_portal: bool,
    pub skull: bool,
    pub spawner: bool,
}

/// Which client block-entity ticker families have a loaded block. Vanilla
/// keeps a loaded block-entity collection and does O(1) work for an absent
/// family; bbb derives the owners from block states, so it reads the chunk
/// map's cached per-chunk family masks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ClientTickingBlockEntityPresence {
    pub(crate) beacon: bool,
    pub(crate) enchanting_table: bool,
    pub(crate) conduit: bool,
    pub(crate) end_gateway: bool,
    pub(crate) skull: bool,
    pub(crate) spawner: bool,
    pub(crate) trial_spawner: bool,
    pub(crate) vault: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientTickingBlockEntityReconciliation {
    revision: Option<StableBlockEntityRenderRevision>,
    families: u32,
}

/// O(1) position index parallel to a dense `Vec<T>` of block-entity states.
#[derive(Debug, Clone, Default)]
pub(crate) struct BlockPosIndex(pub(crate) rustc_hash::FxHashMap<crate::BlockPos, usize>);

impl BlockPosIndex {
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    pub(crate) fn rebuild<T>(&mut self, slice: &[T], pos_of: impl Fn(&T) -> crate::BlockPos) {
        self.0.clear();
        self.0.reserve(slice.len());
        for (index, item) in slice.iter().enumerate() {
            self.0.insert(pos_of(item), index);
        }
    }

    pub(crate) fn sync_if_stale<T>(&mut self, slice: &[T], pos_of: impl Fn(&T) -> crate::BlockPos) {
        if self.0.len() != slice.len() {
            self.rebuild(slice, pos_of);
        }
    }

    pub(crate) fn get<T>(
        &self,
        slice: &[T],
        pos: crate::BlockPos,
        pos_of: impl Fn(&T) -> crate::BlockPos,
    ) -> Option<usize> {
        if self.0.len() == slice.len() {
            return self
                .0
                .get(&pos)
                .copied()
                .filter(|&index| slice.get(index).is_some_and(|item| pos_of(item) == pos));
        }
        slice.iter().position(|item| pos_of(item) == pos)
    }

    pub(crate) fn swap_remove<T>(
        &mut self,
        vec: &mut Vec<T>,
        pos: crate::BlockPos,
        pos_of: impl Fn(&T) -> crate::BlockPos,
    ) -> Option<T> {
        self.sync_if_stale(vec, &pos_of);
        let index = self.0.remove(&pos)?;
        if index >= vec.len() {
            self.rebuild(vec, &pos_of);
            return None;
        }
        let removed = vec.swap_remove(index);
        if let Some(moved) = vec.get(index) {
            self.0.insert(pos_of(moved), index);
        }
        Some(removed)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStore {
    pub(crate) dimension: WorldDimension,
    pub(crate) level: Option<WorldLevelInfo>,
    #[serde(default)]
    pub(crate) gameplay: WorldGameplayState,
    #[serde(default)]
    pub(crate) world_border: WorldBorderState,
    #[serde(default)]
    pub(crate) world_time: Option<crate::WorldTimeState>,
    #[serde(default)]
    pub(crate) weather: crate::WorldWeatherState,
    #[serde(default)]
    pub(crate) sky_flash_time: i32,
    #[serde(default)]
    pub(crate) ticking: crate::WorldTickingState,
    #[serde(default)]
    pub(crate) level_load: WorldLevelLoadState,
    /// Monotonic client-level identity used to reject level-bound async
    /// responses after Login, dimension-changing Respawn, or clear/terminal
    /// transitions. Same-dimension Respawn retains Vanilla's ClientLevel.
    #[serde(default)]
    pub(crate) client_level_generation: u64,
    /// Session level membership from ClientboundLoginPacket. Vanilla exposes
    /// set membership to client suggestions; its shuffled iteration order is
    /// deliberately not canonical state.
    #[serde(default)]
    pub(crate) available_levels: PublicationShared<BTreeSet<String>>,
    /// Raw server declaration from ClientboundLoginPacket. Effective secure
    /// chat additionally depends on profile-key validation support.
    #[serde(default)]
    pub(crate) server_enforces_secure_chat: Option<bool>,
    /// X.509 DER-encoded RSA keys that authenticate signed profile texture
    /// properties. These survive level replacement and reconfiguration like
    /// Vanilla's process-owned `Services` instance.
    #[serde(default)]
    pub(crate) services_profile_property_keys: PublicationShared<Vec<Vec<u8>>>,
    /// X.509 DER-encoded RSA keys that authenticate player certificates.
    #[serde(default)]
    pub(crate) services_player_certificate_keys: PublicationShared<Vec<Vec<u8>>>,
    /// Vanilla carries this once-per-connection-chain warning guard through
    /// reconfiguration and TransferState.
    #[serde(default)]
    pub(crate) seen_insecure_chat_warning: bool,
    /// Monotonic presentation edge for the native toast owner.
    #[serde(default)]
    pub(crate) insecure_chat_warning_count: usize,
    pub(crate) registries: PublicationShared<RegistrySet>,
    pub(crate) chunks: PublicationShared<crate::chunks::ChunkMap>,
    /// Vanilla's ClientLevel light-update queue. Pending work is runtime-only:
    /// canonical snapshots publish light only after the bounded frame poll.
    #[serde(skip)]
    pub(crate) queued_light_updates: RuntimeLightUpdateQueue,
    #[serde(skip)]
    pub(crate) local_light_updates: RuntimeLocalLightUpdates,
    /// Chunks whose packet-load light is still queued — the inverse of vanilla's
    /// per-chunk `LevelLightEngine.setLightEnabled(pos, true)` flag, which only
    /// flips once the queued `applyLightData` closure has run. Unlike the queue
    /// itself this crosses the publication boundary, so the mesh consumer can
    /// withhold a chunk's first renderability until its light applies. A
    /// restored snapshot has an empty queue, so every resident chunk restores
    /// as enabled.
    #[serde(skip)]
    pub(crate) chunks_pending_initial_light: PublicationShared<BTreeSet<ChunkPos>>,
    /// Runtime extraction revisions for the chunk terrain cache. These are
    /// deterministic derivatives of canonical chunk mutations, but are not
    /// part of serialized world snapshots: a restored runtime starts with an
    /// empty render cache and rebuilds every resident chunk once.
    #[serde(skip)]
    pub(crate) terrain_revision: u64,
    #[serde(skip)]
    pub(crate) terrain_chunk_content_revisions: PublicationShared<BTreeMap<ChunkPos, u64>>,
    #[serde(skip)]
    pub(crate) terrain_chunk_mesh_revisions:
        PublicationShared<BTreeMap<ChunkPos, PublicationShared<TerrainChunkRenderRevisions>>>,
    /// Runtime-only edge for accepted block-entity data and chunk loads.
    /// Chunk loads precede their deferred initial light/terrain revision.
    #[serde(skip)]
    pub(crate) block_entity_render_revision: u64,
    #[serde(skip)]
    pub(crate) ticking_block_entity_reconciliation: ClientTickingBlockEntityReconciliation,
    #[serde(default)]
    pub(crate) first_chunk: Option<ChunkPos>,
    #[serde(default)]
    pub(crate) chunk_view: ChunkViewState,
    #[serde(default)]
    pub(crate) block_destructions: Vec<BlockDestructionProgress>,
    #[serde(default)]
    pub(crate) block_destruction_render_tick: u32,
    #[serde(default)]
    pub(crate) block_events: Vec<BlockEventRecord>,
    #[serde(default)]
    pub(crate) moving_pistons: PublicationShared<Vec<crate::MovingPistonState>>,
    #[serde(skip)]
    pub(crate) moving_piston_index: BlockPosIndex,
    #[serde(default)]
    pub(crate) chest_lids: PublicationShared<Vec<crate::ChestLidState>>,
    #[serde(skip)]
    pub(crate) chest_lid_index: BlockPosIndex,
    #[serde(default)]
    pub(crate) bell_shakes: PublicationShared<Vec<crate::BellShakeState>>,
    #[serde(skip)]
    pub(crate) bell_shake_index: BlockPosIndex,
    #[serde(skip)]
    pub(crate) bell_resonations:
        rustc_hash::FxHashMap<crate::BlockPos, crate::blocks::bell_blocks::BellResonationState>,
    #[serde(default)]
    pub(crate) shulker_box_lids: PublicationShared<Vec<crate::ShulkerBoxLidState>>,
    #[serde(skip)]
    pub(crate) shulker_box_lid_index: BlockPosIndex,
    #[serde(default)]
    pub(crate) decorated_pot_wobbles: PublicationShared<Vec<crate::DecoratedPotWobbleState>>,
    #[serde(skip)]
    pub(crate) decorated_pot_wobble_index: BlockPosIndex,
    #[serde(default)]
    pub(crate) skull_blocks: PublicationShared<Vec<crate::SkullBlockState>>,
    #[serde(default)]
    pub(crate) beacon_blocks: PublicationShared<Vec<crate::BeaconBlockState>>,
    pub(crate) conduit_blocks: PublicationShared<Vec<crate::ConduitBlockState>>,
    #[serde(default)]
    pub(crate) end_gateways: PublicationShared<Vec<crate::EndGatewayBlockState>>,
    #[serde(default)]
    pub(crate) spawner_blocks: PublicationShared<Vec<crate::SpawnerBlockState>>,
    #[serde(default)]
    pub(crate) trial_spawner_blocks: PublicationShared<Vec<crate::TrialSpawnerBlockRenderState>>,
    #[serde(default)]
    pub(crate) vault_blocks: PublicationShared<Vec<crate::VaultBlockState>>,
    #[serde(default)]
    pub(crate) enchanting_table_books: PublicationShared<Vec<crate::EnchantingTableBookState>>,
    #[serde(default)]
    pub(crate) enchanting_book_random: crate::EnchantingBookRandom,
    /// Vanilla `ScreenEffectRenderer` item-activation animation. Cleared by
    /// `GameRenderer.resetData` on clearLevel/disconnect only; a dimension
    /// respawn (`Minecraft.setLevel`) keeps it running.
    #[serde(default)]
    pub(crate) item_activation: Option<crate::ItemActivationState>,
    #[serde(default)]
    pub(crate) item_activation_random: crate::ItemActivationRandomState,
    #[serde(default)]
    pub(crate) level_events: Vec<LevelEventRecord>,
    #[serde(default)]
    pub(crate) last_block_changed_ack: Option<BlockChangedAckState>,
    /// Vanilla `ClientLevel.blockStatePredictionHandler` state. This stays
    /// level-bound when a same-dimension respawn replaces only LocalPlayer.
    #[serde(default)]
    pub(crate) local_prediction_sequence: i32,
    #[serde(default)]
    pub(crate) local_prediction_last_teleport_sequence: Option<i32>,
    #[serde(default)]
    pub(crate) local_block_predictions: Vec<LocalBlockPredictionState>,
    pub(crate) entities: crate::entities::EntityStore,
    /// Vanilla `options.keySpectatorOutlines.isDown()`, synchronized from the
    /// input runtime each simulation step for
    /// `Minecraft.shouldEntityAppearGlowing`'s spectator branch.
    #[serde(default)]
    pub(crate) spectator_outlines_key_down: bool,
    #[serde(default)]
    pub(crate) scoreboard: PublicationShared<ScoreboardState>,
    #[serde(default)]
    pub(crate) client_hud: ClientHudState,
    #[serde(default)]
    pub(crate) client_audio: ClientAudioState,
    #[serde(default)]
    pub(crate) client_effects: ClientEffectsState,
    /// Prediction-time effects that must survive a `PASS` result so the item
    /// fallback can continue after the client-visible block callback.
    #[serde(skip)]
    pub(crate) pending_local_interaction_particle_actions: Vec<LocalInteractionParticleAction>,
    /// Client-owned particle spawns produced by local player effects, entity
    /// client ticks, and block-entity resonance effects.
    #[serde(skip)]
    pub(crate) pending_client_particle_spawns: Vec<crate::particles::ParticleSpawn>,
    /// Client-local level events (e.g. local block break `2001`, wax/scrape
    /// `3003`..=`3005`) emitted on actions where Vanilla excludes the acting player.
    #[serde(skip)]
    pub(crate) pending_local_level_events: Vec<bbb_protocol::packets::LevelEvent>,
    /// One-shot portal-transition side effects produced by the client tick:
    /// the `PORTAL_TRIGGER` ambience on the 0→rising edge and the vanilla
    /// screen-close request while standing inside the portal.
    pub(crate) pending_portal_transition_trigger_sound: bool,
    pub(crate) pending_portal_transition_container_close: bool,
    #[serde(default)]
    pub(crate) client_ui: ClientUiState,
    #[serde(default)]
    pub(crate) waypoints: PublicationShared<ClientWaypointsState>,
    #[serde(default)]
    pub(crate) client_chat: PublicationShared<ClientChatState>,
    #[serde(default)]
    pub(crate) client_combat: ClientCombatState,
    #[serde(default)]
    pub(crate) client_debug_game: PublicationShared<ClientDebugGameState>,
    #[serde(default)]
    pub(crate) debug_query: ClientDebugQueryState,
    #[serde(default)]
    pub(crate) features: ClientFeatureState,
    #[serde(skip)]
    pub(crate) known_packs_manager: Option<std::sync::Arc<bbb_client_assets::KnownPacksManager>>,
    #[serde(skip)]
    pub(crate) selected_known_pack_data: Option<PublicationShared<KnownPackDataSource>>,
    #[serde(skip)]
    pub(crate) known_pack_data_error: Option<String>,
    /// Runtime-only exact invalidation key for the scoreboard sidebar
    /// projection. It is cloned into immutable publications but intentionally
    /// rebuilt behind the client-level generation after snapshot restore.
    #[serde(skip)]
    pub(crate) scoreboard_display_revision: u64,
    /// Runtime-only exact invalidation key for the player-list static payload.
    #[serde(skip)]
    pub(crate) player_list_display_revision: u64,
    #[serde(default)]
    pub(crate) player_info: PublicationShared<PlayerInfoState>,
    #[serde(default)]
    pub(crate) player_social: PublicationShared<PlayerSocialState>,
    #[serde(default)]
    pub(crate) presentation: PublicationShared<ServerPresentationState>,
    #[serde(default)]
    pub(crate) client_stats: PublicationShared<ClientStatsState>,
    #[serde(default)]
    pub(crate) cooldowns: BTreeMap<String, ItemCooldownState>,
    #[serde(default)]
    pub(crate) command_suggestions: CommandSuggestionsState,
    #[serde(default)]
    pub(crate) commands: PublicationShared<CommandTreeState>,
    #[serde(skip)]
    pub(crate) command_completion_sources_revision: u64,
    /// Resource-pack sound event ids exposed by the native sound manager to
    /// the `minecraft:available_sounds` command suggestion provider. The
    /// derived resource view is publication-shared and intentionally omitted
    /// from serialized canonical snapshots.
    #[serde(skip)]
    pub(crate) available_command_sounds: PublicationShared<Vec<String>>,
    #[serde(default)]
    pub(crate) recipe_book: PublicationShared<crate::ClientRecipeBookState>,
    #[serde(default)]
    pub(crate) recipes: PublicationShared<ClientRecipesState>,
    #[serde(default)]
    pub(crate) advancements: PublicationShared<ClientAdvancementsState>,
    #[serde(default)]
    pub(crate) client_toasts: ClientToastState,
    #[serde(default)]
    pub(crate) client_tutorial: ClientTutorialState,
    #[serde(default)]
    pub(crate) maps: PublicationShared<BTreeMap<i32, MapItemState>>,
    #[serde(skip)]
    pub(crate) map_lru: crate::PublicationOmitted<crate::maps::MapLruTracker>,
    #[serde(default)]
    pub(crate) last_map_color_patch: Option<LastMapColorPatchState>,
    #[serde(default)]
    pub(crate) local_player: LocalPlayerState,
    /// Startup/login identity of the native local client. The play protocol
    /// does not repeat this UUID, but `ClientLevel.getEntity(UUID)` includes
    /// the LocalPlayer for waypoint following and self-marker suppression.
    /// It is process configuration rather than server-owned snapshot state.
    #[serde(skip)]
    pub(crate) local_player_profile_id: Option<uuid::Uuid>,
    /// Previous local-player pose endpoint used by render interpolation. It is
    /// runtime client state rather than serialized canonical wire state.
    #[serde(skip)]
    pub(crate) local_player_old_pose: Option<crate::LocalPlayerPoseState>,
    #[serde(default)]
    pub(crate) local_player_id: Option<i32>,
    #[serde(default)]
    pub(crate) local_player_vehicle_id: Option<i32>,
    #[serde(default)]
    pub(crate) removed_player_vehicle_id: Option<i32>,
    #[serde(default)]
    pub(crate) last_projectile_power: Option<ProjectilePowerUpdateState>,
    #[serde(default)]
    pub(crate) creative_inventory: PublicationShared<crate::CreativeInventoryState>,
    pub(crate) inventory: PublicationShared<InventoryState>,
    /// Vanilla item/block default profiles synced from the asset registry at
    /// startup; flattened for serialized-snapshot compatibility.
    #[serde(flatten, default)]
    pub(crate) items: ItemProfiles,
    #[serde(default)]
    pub(crate) apply_diagnostics: WorldApplyDiagnosticsState,
    pub(crate) counters: WorldCounters,
    #[serde(skip)]
    pub(crate) content_registry: crate::registries::state::SessionContentRegistry,
    #[serde(skip)]
    pub(crate) mod_custom_payloads: crate::PublicationOmitted<VecDeque<(String, Vec<u8>)>>,
}

impl WorldStore {
    pub fn new() -> Self {
        Self {
            registries: RegistrySet::vanilla_26_1().into(),
            ..Self::default()
        }
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.content_registry.0
    }

    pub fn drain_mod_custom_payloads(&mut self) -> impl Iterator<Item = (String, Vec<u8>)> + '_ {
        self.mod_custom_payloads.drain(..)
    }

    pub(crate) fn set_content_registry(&mut self, registry: Arc<ContentRegistry>) {
        let wrapped = crate::registries::state::SessionContentRegistry(Arc::clone(&registry));
        self.content_registry = wrapped.clone();
        self.registries.content_registry = wrapped;
        self.items.set_content_registry(Arc::clone(&registry));
        self.entities.set_content_registry(registry);
    }

    pub(crate) fn advance_scoreboard_display_revision(&mut self) {
        self.scoreboard_display_revision = self.scoreboard_display_revision.saturating_add(1);
        self.advance_player_list_display_revision();
    }

    pub(crate) fn advance_player_list_display_revision(&mut self) {
        self.player_list_display_revision = self.player_list_display_revision.saturating_add(1);
    }

    pub(crate) fn advance_block_entity_render_revision(&mut self) {
        self.block_entity_render_revision = self.block_entity_render_revision.saturating_add(1);
    }

    pub fn item_profiles(&self) -> &ItemProfiles {
        &self.items
    }

    pub(crate) fn push_pending_client_particle_spawn(
        &mut self,
        spawn: crate::particles::ParticleSpawn,
    ) {
        const MAX_PENDING_CLIENT_PARTICLE_SPAWNS: usize = 4_096;
        if self.pending_client_particle_spawns.len() < MAX_PENDING_CLIENT_PARTICLE_SPAWNS {
            self.pending_client_particle_spawns.push(spawn);
        }
    }

    pub fn take_pending_client_particle_spawns(&mut self) -> Vec<crate::particles::ParticleSpawn> {
        std::mem::take(&mut self.pending_client_particle_spawns)
    }

    pub(crate) fn push_pending_local_level_event(
        &mut self,
        event: bbb_protocol::packets::LevelEvent,
    ) {
        const MAX_PENDING_LOCAL_LEVEL_EVENTS: usize = 256;
        if self.pending_local_level_events.len() < MAX_PENDING_LOCAL_LEVEL_EVENTS {
            self.pending_local_level_events.push(event);
        }
    }

    pub fn take_pending_local_level_events(&mut self) -> Vec<bbb_protocol::packets::LevelEvent> {
        std::mem::take(&mut self.pending_local_level_events)
    }

    pub fn stable_block_entity_render_revision(&self) -> StableBlockEntityRenderRevision {
        StableBlockEntityRenderRevision {
            client_level_generation: self.client_level_generation,
            terrain: self.terrain_revision,
            block_entity_data: self.block_entity_render_revision,
        }
    }

    /// Reconcile each ticker family once per loaded-state change, including
    /// zero-tick frames while animations are frozen.
    pub(crate) fn reconcile_client_ticking_block_entities(
        &mut self,
        family: crate::chunks::BlockFamily,
    ) -> bool {
        let revision = self.stable_block_entity_render_revision();
        let reconciled = &mut self.ticking_block_entity_reconciliation;
        if reconciled.revision != Some(revision) {
            reconciled.revision = Some(revision);
            reconciled.families = 0;
        }
        let pending = reconciled.families & family.bit() == 0;
        reconciled.families |= family.bit();
        pending
    }

    pub(crate) fn client_ticking_block_entity_presence(&self) -> ClientTickingBlockEntityPresence {
        use crate::chunks::BlockFamily;
        let families = self.chunks.families();
        let has = |family: BlockFamily| families & family.bit() != 0;
        ClientTickingBlockEntityPresence {
            beacon: has(BlockFamily::Beacon),
            enchanting_table: has(BlockFamily::EnchantingTable),
            conduit: has(BlockFamily::Conduit),
            end_gateway: has(BlockFamily::EndGateway),
            skull: has(BlockFamily::Skull),
            spawner: has(BlockFamily::Spawner),
            trial_spawner: has(BlockFamily::TrialSpawner),
            vault: has(BlockFamily::Vault),
        }
    }

    /// Conservative candidates for ordinary renderers in the inclusive column range.
    pub fn block_entity_render_presence(
        &self,
        columns: Option<(ChunkPos, ChunkPos)>,
    ) -> BlockEntityRenderPresence {
        use crate::chunks::BlockFamily;
        let all = self.chunks.families();
        let mut chest_columns = Vec::new();
        let mut spawner_columns = Vec::new();
        let (near, has_records) = match columns {
            Some((min, max)) => {
                self.chunks
                    .families_and_records_in_columns(min, max, |pos, mask| {
                        if mask & BlockFamily::Chest.bit() != 0 {
                            chest_columns.push(pos);
                        }
                        if mask & BlockFamily::Spawner.bit() != 0 {
                            spawner_columns.push(pos);
                        }
                    })
            }
            None => {
                chest_columns.extend(self.chunks.with_family(BlockFamily::Chest).map(|c| c.pos));
                spawner_columns
                    .extend(self.chunks.with_family(BlockFamily::Spawner).map(|c| c.pos));
                (all, true)
            }
        };
        let has = |family: BlockFamily| near & family.bit() != 0;
        BlockEntityRenderPresence {
            item_displays: has_records
                && (has(BlockFamily::Campfire)
                    || has(BlockFamily::Shelf)
                    || has(BlockFamily::Brushable)
                    || has(BlockFamily::Vault))
                && self.has_block_entity_item_display_sources(),
            chest_columns,
            spawner_columns,
            bell: has(BlockFamily::Bell),
            shulker_box: has(BlockFamily::ShulkerBox),
            decorated_pot: has(BlockFamily::DecoratedPot),
            banner: has(BlockFamily::Banner),
            enchanting_table: has(BlockFamily::EnchantingTable),
            conduit: has(BlockFamily::Conduit),
            copper_golem_statue: has_records && has(BlockFamily::CopperGolemStatue),
            // This family includes end gateways, which have a longer view distance.
            end_portal: all & BlockFamily::EndPortal.bit() != 0,
            skull: has(BlockFamily::Skull),
            // Trial spawners also draw from their retained client tick state.
            spawner: has(BlockFamily::Spawner)
                || has(BlockFamily::TrialSpawner)
                || self.trial_spawner_blocks.iter().any(|state| {
                    let pos = state.pos.chunk();
                    columns.is_none_or(|(min, max)| {
                        (min.x..=max.x).contains(&pos.x) && (min.z..=max.z).contains(&pos.z)
                    })
                }),
        }
    }

    /// `Level.isRaining()` (`canHaveWeather && getRainLevel(1.0F) > 0.2`) plus
    /// the level sea level, resolved once per entity block-effect pass.
    pub(crate) fn entity_block_effect_weather(&self) -> crate::client::EntityBlockEffectWeather {
        crate::client::EntityBlockEffectWeather {
            raining: self
                .level
                .as_ref()
                .is_some_and(crate::WorldLevelInfo::can_have_weather)
                && self.weather.rain_level > 0.2,
            sea_level: self.level.as_ref().map_or(63, |level| level.sea_level),
        }
    }

    /// Vanilla `Level.isThundering`: the interpolated thunder level is
    /// multiplied by the interpolated rain level before the `0.9` threshold.
    pub fn is_thundering(&self) -> bool {
        self.level
            .as_ref()
            .is_some_and(crate::WorldLevelInfo::can_have_weather)
            && self.weather.thunder_level * self.weather.rain_level > 0.9
    }

    /// Vanilla `Level.isRainingAt`, using synced rain, sky light,
    /// `MOTION_BLOCKING`, and biome precipitation facts.
    pub fn is_raining_at(&self, pos: crate::BlockPos) -> bool {
        crate::client::EntityBlockEffectWorld::new(
            self.dimension,
            &self.registries,
            &self.chunks,
            self.entity_block_effect_weather(),
        )
        .is_raining_at(pos)
    }
}
