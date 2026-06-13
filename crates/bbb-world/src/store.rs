use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    BlockDestructionProgress, BlockEventRecord, ChunkColumn, ChunkViewState,
    ClientAdvancementsState, ClientChatState, ClientDebugQueryState, ClientFeatureState,
    ClientHudState, ClientRecipesState, ClientUiState, ClientWaypointsState,
    CommandSuggestionsState, CommandTreeState, InventoryState, ItemCooldownState, LevelEventRecord,
    LocalPlayerState, MapItemState, PlayerInfoState, RegistrySet, ScoreboardState,
    ServerPresentationState, WorldBorderState, WorldCounters, WorldDimension, WorldLevelInfo,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStore {
    pub(crate) dimension: WorldDimension,
    pub(crate) level: Option<WorldLevelInfo>,
    #[serde(default)]
    pub(crate) world_border: WorldBorderState,
    #[serde(default)]
    pub(crate) world_time: Option<crate::WorldTimeState>,
    #[serde(default)]
    pub(crate) weather: crate::WorldWeatherState,
    #[serde(default)]
    pub(crate) ticking: crate::WorldTickingState,
    pub(crate) registries: RegistrySet,
    pub(crate) chunks: Vec<ChunkColumn>,
    #[serde(default)]
    pub(crate) chunk_view: ChunkViewState,
    #[serde(default)]
    pub(crate) block_destructions: Vec<BlockDestructionProgress>,
    #[serde(default)]
    pub(crate) block_events: Vec<BlockEventRecord>,
    #[serde(default)]
    pub(crate) level_events: Vec<LevelEventRecord>,
    pub(crate) entities: Vec<crate::EntityState>,
    #[serde(default)]
    pub(crate) scoreboard: ScoreboardState,
    #[serde(default)]
    pub(crate) client_hud: ClientHudState,
    #[serde(default)]
    pub(crate) client_ui: ClientUiState,
    #[serde(default)]
    pub(crate) waypoints: ClientWaypointsState,
    #[serde(default)]
    pub(crate) client_chat: ClientChatState,
    #[serde(default)]
    pub(crate) debug_query: ClientDebugQueryState,
    #[serde(default)]
    pub(crate) features: ClientFeatureState,
    #[serde(default)]
    pub(crate) player_info: PlayerInfoState,
    #[serde(default)]
    pub(crate) presentation: ServerPresentationState,
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
    pub(crate) local_player: LocalPlayerState,
    #[serde(default)]
    pub(crate) local_player_id: Option<i32>,
    #[serde(default)]
    pub(crate) local_player_vehicle_id: Option<i32>,
    pub(crate) inventory: InventoryState,
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
