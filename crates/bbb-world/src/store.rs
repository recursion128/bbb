use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::item_profiles::ItemProfiles;
use crate::{
    BlockChangedAckState, BlockDestructionProgress, BlockEventRecord, ChunkColumn, ChunkPos,
    ChunkViewState, ClientAdvancementsState, ClientAudioState, ClientChatState, ClientCombatState,
    ClientDebugGameState, ClientDebugQueryState, ClientEffectsState, ClientFeatureState,
    ClientHudState, ClientRecipesState, ClientStatsState, ClientUiState, ClientWaypointsState,
    CommandSuggestionsState, CommandTreeState, InventoryState, ItemCooldownState,
    LastMapColorPatchState, LevelEventRecord, LocalBlockPredictionState, LocalPlayerState,
    MapItemState, PlayerInfoState, ProjectilePowerUpdateState, RegistrySet, ScoreboardState,
    ServerPresentationState, WorldApplyDiagnosticsState, WorldBorderState, WorldCounters,
    WorldDimension, WorldGameplayState, WorldLevelInfo,
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
    pub(crate) registries: RegistrySet,
    pub(crate) chunks: Vec<ChunkColumn>,
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
    pub(crate) chest_lids: Vec<crate::ChestLidState>,
    #[serde(default)]
    pub(crate) bell_shakes: Vec<crate::BellShakeState>,
    #[serde(default)]
    pub(crate) shulker_box_lids: Vec<crate::ShulkerBoxLidState>,
    #[serde(default)]
    pub(crate) decorated_pot_wobbles: Vec<crate::DecoratedPotWobbleState>,
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
    #[serde(default)]
    pub(crate) player_info: PlayerInfoState,
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
    #[serde(default)]
    pub(crate) local_player_id: Option<i32>,
    #[serde(default)]
    pub(crate) local_player_vehicle_id: Option<i32>,
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
            registries: RegistrySet::vanilla_26_1(),
            ..Self::default()
        }
    }
}
