use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    BlockChangedAckState, BlockDestructionProgress, BlockEventRecord, ChunkColumn, ChunkPos,
    ChunkViewState, ClientAdvancementsState, ClientAudioState, ClientChatState, ClientCombatState,
    ClientDebugGameState, ClientDebugQueryState, ClientEffectsState, ClientFeatureState,
    ClientHudState, ClientRecipesState, ClientStatsState, ClientUiState, ClientWaypointsState,
    CommandSuggestionsState, CommandTreeState, InventoryState, ItemAttackRange, ItemCooldownState,
    ItemEquipmentSlot, ItemUseEffects, LastMapColorPatchState, LevelEventRecord,
    LocalBlockPredictionState, LocalPlayerState, MapItemState, MountArmorSlotKind, PlayerInfoState,
    ProjectilePowerUpdateState, RegistrySet, ScoreboardState, ServerPresentationState,
    WorldApplyDiagnosticsState, WorldBlockDestroyProfile, WorldBlockSoundProfile, WorldBorderState,
    WorldCounters, WorldDimension, WorldGameplayState, WorldItemMiningProfile, WorldLevelInfo,
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
    #[serde(default)]
    pub(crate) default_item_max_stack_sizes: BTreeMap<i32, i32>,
    #[serde(default)]
    pub(crate) default_item_equipment_slots: BTreeMap<i32, ItemEquipmentSlot>,
    #[serde(default)]
    pub(crate) default_mount_body_armor_kinds: BTreeMap<i32, MountArmorSlotKind>,
    #[serde(default)]
    pub(crate) default_item_attack_ranges: BTreeMap<i32, ItemAttackRange>,
    #[serde(default)]
    pub(crate) default_item_use_effects: BTreeMap<i32, ItemUseEffects>,
    #[serde(default)]
    pub(crate) default_damageable_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) default_piercing_weapon_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) furnace_fuel_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) brewing_potion_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) brewing_ingredient_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) enchantment_lapis_lazuli_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) cartography_additional_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) freeze_immune_wearable_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) powder_snow_walkable_foot_item_ids: BTreeSet<i32>,
    #[serde(default)]
    pub(crate) default_item_mining_profiles: BTreeMap<i32, WorldItemMiningProfile>,
    #[serde(default)]
    pub(crate) default_block_destroy_profiles: BTreeMap<String, WorldBlockDestroyProfile>,
    #[serde(default)]
    pub(crate) default_block_sound_profiles: BTreeMap<String, WorldBlockSoundProfile>,
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
