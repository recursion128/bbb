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
    ClientHudState, ClientRecipesState, ClientStatsState, ClientUiState, ClientWaypointsState,
    CommandSuggestionsState, CommandTreeState, InventoryState, ItemCooldownState,
    LastMapColorPatchState, LevelEventRecord, LocalBlockPredictionState,
    LocalInteractionParticleAction, LocalPlayerState, MapItemState, PlayerInfoState,
    PlayerSocialState, ProjectilePowerUpdateState, PublicationShared, RegistrySet, ScoreboardState,
    ServerPresentationState, WorldApplyDiagnosticsState, WorldBorderState, WorldCounters,
    WorldDimension, WorldGameplayState, WorldLevelInfo, WorldLevelLoadState,
};

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
    /// responses after Login, Respawn, or disconnect transitions.
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
    /// X.509 DER-encoded RSA public keys from the Minecraft Services key set.
    /// These survive level replacement and connection reconfiguration like
    /// Vanilla's process-owned `Services` instance.
    #[serde(default)]
    pub(crate) services_profile_keys: Vec<Vec<u8>>,
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
    pub(crate) enchanting_table_books: Vec<crate::EnchantingTableBookState>,
    #[serde(default)]
    pub(crate) enchanting_book_random: crate::EnchantingBookRandom,
    #[serde(default)]
    pub(crate) level_events: Vec<LevelEventRecord>,
    #[serde(default)]
    pub(crate) last_block_changed_ack: Option<BlockChangedAckState>,
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
    #[serde(default)]
    pub(crate) player_info: PlayerInfoState,
    #[serde(default)]
    pub(crate) player_social: PlayerSocialState,
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
}
