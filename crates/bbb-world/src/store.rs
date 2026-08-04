use std::collections::{BTreeMap, BTreeSet};

use bbb_pack::KnownPackDataSource;
use serde::{Deserialize, Serialize};

use crate::chunks::RuntimeLightUpdateQueue;
use crate::item_profiles::ItemProfiles;
use crate::terrain::TerrainChunkRenderRevisions;
use crate::{
    BlockChangedAckState, BlockDestructionProgress, BlockEventRecord, ChunkColumn, ChunkPos,
    ChunkViewState, ClientAdvancementsState, ClientAudioState, ClientChatState, ClientCombatState,
    ClientDebugGameState, ClientDebugQueryState, ClientEffectsState, ClientFeatureState,
    ClientHudState, ClientRecipesState, ClientStatsState, ClientToastState, ClientTutorialState,
    ClientUiState, ClientWaypointsState, CommandSuggestionsState, CommandTreeState, InventoryState,
    ItemCooldownState, LastMapColorPatchState, LevelEventRecord, LocalBlockPredictionState,
    LocalInteractionParticleAction, LocalPlayerState, MapItemState, PlayerInfoState,
    PlayerReportingState, PlayerSocialState, ProjectilePowerUpdateState, PublicationShared,
    RegistrySet, ScoreboardState, ServerPresentationState, WorldApplyDiagnosticsState,
    WorldBorderState, WorldCounters, WorldDimension, WorldGameplayState, WorldLevelInfo,
    WorldLevelLoadState,
};

/// Exact runtime-only invalidation key for the block-entity projections whose
/// complete inputs are stable between render frames.
///
/// Terrain covers loaded block states and light. `block_entity_data` covers
/// accepted incremental NBT updates that do not otherwise dirty terrain.
/// Client-level identity prevents a numerically equal fresh level from
/// selecting an owner retained for the previous level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StableBlockEntityRenderRevision {
    pub client_level_generation: u64,
    pub terrain: u64,
    pub block_entity_data: u64,
}

/// Runtime-only palette presence used by the client block-entity tickers.
/// Vanilla keeps a loaded block-entity collection and therefore does O(1)
/// work when a ticker family is absent. bbb derives those owners from chunk
/// block states, so cache the equivalent presence behind the exact terrain
/// revision instead of rescanning every loaded section once per family/tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ClientTickingBlockEntityPresence {
    pub(crate) enchanting_table: bool,
    pub(crate) conduit: bool,
    pub(crate) end_gateway: bool,
    pub(crate) skull: bool,
    pub(crate) spawner: bool,
    pub(crate) trial_spawner: bool,
    pub(crate) vault: bool,
}

impl ClientTickingBlockEntityPresence {
    fn is_complete(self) -> bool {
        self.enchanting_table
            && self.conduit
            && self.end_gateway
            && self.skull
            && self.spawner
            && self.trial_spawner
            && self.vault
    }

    fn record_block_name(&mut self, name: &str) {
        match name {
            "minecraft:enchanting_table" => self.enchanting_table = true,
            "minecraft:conduit" => self.conduit = true,
            "minecraft:end_gateway" => self.end_gateway = true,
            "minecraft:spawner" => self.spawner = true,
            "minecraft:trial_spawner" => self.trial_spawner = true,
            "minecraft:vault" => self.vault = true,
            name if crate::skull_blocks::is_skull_block_name(name) => self.skull = true,
            _ => {}
        }
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
    pub(crate) available_levels: BTreeSet<String>,
    /// Raw server declaration from ClientboundLoginPacket. Effective secure
    /// chat additionally depends on profile-key validation support.
    #[serde(default)]
    pub(crate) server_enforces_secure_chat: Option<bool>,
    /// X.509 DER-encoded RSA keys that authenticate signed profile texture
    /// properties. These survive level replacement and reconfiguration like
    /// Vanilla's process-owned `Services` instance.
    #[serde(default)]
    pub(crate) services_profile_property_keys: Vec<Vec<u8>>,
    /// X.509 DER-encoded RSA keys that authenticate player certificates.
    #[serde(default)]
    pub(crate) services_player_certificate_keys: Vec<Vec<u8>>,
    /// Vanilla carries this once-per-connection-chain warning guard through
    /// reconfiguration and TransferState.
    #[serde(default)]
    pub(crate) seen_insecure_chat_warning: bool,
    /// Monotonic presentation edge for the native toast owner.
    #[serde(default)]
    pub(crate) insecure_chat_warning_count: usize,
    pub(crate) registries: PublicationShared<RegistrySet>,
    pub(crate) chunks: Vec<PublicationShared<ChunkColumn>>,
    /// Vanilla's ClientLevel light-update queue. Pending work is runtime-only:
    /// canonical snapshots publish light only after the bounded frame poll.
    #[serde(skip)]
    pub(crate) queued_light_updates: RuntimeLightUpdateQueue,
    /// Runtime extraction revisions for the chunk terrain cache. These are
    /// deterministic derivatives of canonical chunk mutations, but are not
    /// part of serialized world snapshots: a restored runtime starts with an
    /// empty render cache and rebuilds every resident chunk once.
    #[serde(skip)]
    pub(crate) terrain_revision: u64,
    #[serde(skip)]
    pub(crate) terrain_chunk_content_revisions: BTreeMap<ChunkPos, u64>,
    #[serde(skip)]
    pub(crate) terrain_chunk_mesh_revisions:
        BTreeMap<ChunkPos, PublicationShared<TerrainChunkRenderRevisions>>,
    /// Runtime-only edge for accepted incremental block-entity data updates.
    /// Full chunk/block/light changes are already represented by
    /// `terrain_revision`.
    #[serde(skip)]
    pub(crate) block_entity_render_revision: u64,
    #[serde(skip)]
    pub(crate) client_ticking_block_entity_presence: Option<(
        StableBlockEntityRenderRevision,
        ClientTickingBlockEntityPresence,
    )>,
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
    pub(crate) moving_pistons: Vec<crate::MovingPistonState>,
    #[serde(default)]
    pub(crate) chest_lids: Vec<crate::ChestLidState>,
    #[serde(default)]
    pub(crate) bell_shakes: Vec<crate::BellShakeState>,
    #[serde(default)]
    pub(crate) shulker_box_lids: Vec<crate::ShulkerBoxLidState>,
    #[serde(default)]
    pub(crate) decorated_pot_wobbles: Vec<crate::DecoratedPotWobbleState>,
    #[serde(default)]
    pub(crate) skull_blocks: Vec<crate::SkullBlockState>,
    #[serde(default)]
    pub(crate) conduit_blocks: Vec<crate::ConduitBlockState>,
    #[serde(default)]
    pub(crate) end_gateways: Vec<crate::EndGatewayBlockState>,
    #[serde(default)]
    pub(crate) spawner_blocks: Vec<crate::SpawnerBlockState>,
    #[serde(default)]
    pub(crate) trial_spawner_blocks: Vec<crate::TrialSpawnerBlockRenderState>,
    #[serde(default)]
    pub(crate) vault_blocks: Vec<crate::VaultBlockState>,
    #[serde(default)]
    pub(crate) enchanting_table_books: Vec<crate::EnchantingTableBookState>,
    #[serde(default)]
    pub(crate) enchanting_book_random: crate::EnchantingBookRandom,
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
    #[serde(default)]
    pub(crate) scoreboard: ScoreboardState,
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
    #[serde(default)]
    pub(crate) client_ui: ClientUiState,
    #[serde(default)]
    pub(crate) waypoints: ClientWaypointsState,
    #[serde(default)]
    pub(crate) client_chat: ClientChatState,
    #[serde(default)]
    pub(crate) client_combat: ClientCombatState,
    #[serde(default)]
    pub(crate) client_debug_game: ClientDebugGameState,
    #[serde(default)]
    pub(crate) debug_query: ClientDebugQueryState,
    #[serde(default)]
    pub(crate) features: ClientFeatureState,
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
    pub(crate) player_info: PlayerInfoState,
    #[serde(default)]
    pub(crate) player_social: PlayerSocialState,
    #[serde(default)]
    pub(crate) player_reporting: PlayerReportingState,
    #[serde(default)]
    pub(crate) presentation: ServerPresentationState,
    #[serde(default)]
    pub(crate) client_stats: ClientStatsState,
    #[serde(default)]
    pub(crate) cooldowns: BTreeMap<String, ItemCooldownState>,
    #[serde(default)]
    pub(crate) command_suggestions: CommandSuggestionsState,
    #[serde(default)]
    pub(crate) commands: CommandTreeState,
    /// Resource-pack sound event ids exposed by the native sound manager to
    /// the `minecraft:available_sounds` command suggestion provider. The
    /// derived resource view is publication-shared and intentionally omitted
    /// from serialized canonical snapshots.
    #[serde(skip)]
    pub(crate) available_command_sounds: PublicationShared<Vec<String>>,
    #[serde(default)]
    pub(crate) recipe_book: crate::ClientRecipeBookState,
    #[serde(default)]
    pub(crate) recipes: ClientRecipesState,
    #[serde(default)]
    pub(crate) advancements: ClientAdvancementsState,
    #[serde(default)]
    pub(crate) client_toasts: ClientToastState,
    #[serde(default)]
    pub(crate) client_tutorial: ClientTutorialState,
    #[serde(default)]
    pub(crate) maps: BTreeMap<i32, MapItemState>,
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
    pub(crate) creative_inventory: crate::CreativeInventoryState,
    pub(crate) inventory: InventoryState,
    /// Vanilla item/block default profiles synced from the asset registry at
    /// startup; flattened for serialized-snapshot compatibility.
    #[serde(flatten, default)]
    pub(crate) items: ItemProfiles,
    #[serde(default)]
    pub(crate) apply_diagnostics: WorldApplyDiagnosticsState,
    pub(crate) counters: WorldCounters,
}

impl WorldStore {
    pub fn new() -> Self {
        Self {
            registries: RegistrySet::vanilla_26_1().into(),
            ..Self::default()
        }
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

    pub fn stable_block_entity_render_revision(&self) -> StableBlockEntityRenderRevision {
        StableBlockEntityRenderRevision {
            client_level_generation: self.client_level_generation,
            terrain: self.terrain_revision,
            block_entity_data: self.block_entity_render_revision,
        }
    }

    pub(crate) fn client_ticking_block_entity_presence(
        &mut self,
    ) -> ClientTickingBlockEntityPresence {
        let revision = self.stable_block_entity_render_revision();
        if let Some((cached_revision, presence)) = self.client_ticking_block_entity_presence {
            if cached_revision == revision {
                return presence;
            }
        }

        let mut presence = ClientTickingBlockEntityPresence::default();
        'chunks: for chunk in &self.chunks {
            for section in &chunk.sections {
                let block_states = &section.block_states;
                match block_states.palette_kind {
                    crate::PaletteKind::SingleValue | crate::PaletteKind::Local => {
                        for global_id in &block_states.palette_global_ids {
                            if let Some(state) = self.registries.block_state(*global_id) {
                                presence.record_block_name(&state.name);
                            }
                            if presence.is_complete() {
                                break 'chunks;
                            }
                        }
                    }
                    crate::PaletteKind::Global => {
                        for index in 0..block_states.entry_count {
                            let Some(value) = block_states.value_at(index) else {
                                continue;
                            };
                            if let Some(state) = self.registries.block_state(value.global_id) {
                                presence.record_block_name(&state.name);
                            }
                            if presence.is_complete() {
                                break 'chunks;
                            }
                        }
                    }
                }
            }
        }
        self.client_ticking_block_entity_presence = Some((revision, presence));
        presence
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
