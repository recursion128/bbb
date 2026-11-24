use std::collections::BTreeSet;

use bbb_protocol::game_event_types::{
    VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID, VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
    VANILLA_CLIENT_GAME_EVENT_IMMEDIATE_RESPAWN_ID,
    VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
    VANILLA_CLIENT_GAME_EVENT_LIMITED_CRAFTING_ID,
    VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID,
    VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID, VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID,
    VANILLA_CLIENT_GAME_EVENT_STOP_RAINING_ID, VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
    VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
};
use bbb_protocol::game_types::{
    GameType, VANILLA_GAME_TYPE_ADVENTURE_ID, VANILLA_GAME_TYPE_CREATIVE_ID,
    VANILLA_GAME_TYPE_NOT_SET_ID, VANILLA_GAME_TYPE_SPECTATOR_ID, VANILLA_GAME_TYPE_SURVIVAL_ID,
};
pub use bbb_protocol::packets::{ClockUpdate as ClockUpdateState, GlobalPos as WorldGlobalPos};
use bbb_protocol::packets::{
    CommonPlayerSpawnInfo as ProtocolSpawnInfo, GameEvent as ProtocolGameEvent,
    PlayLogin as ProtocolPlayLogin, PlayTime as ProtocolPlayTime, Respawn as ProtocolRespawn,
    TickingState as ProtocolTickingState, TickingStep as ProtocolTickingStep,
};
use bbb_registry_codec::registry_codec::{DimensionCardinalLight, DimensionSkybox};
use serde::{Deserialize, Serialize};

use crate::{
    registries::{ClientEnvironmentAttributes, WorldIntProvider, DIMENSION_TYPE_REGISTRY},
    ChatNarrationKind, ClientGameEventScreenState, WorldStore,
};

mod dimension_defaults;
pub(crate) use dimension_defaults::*;

const RESPAWN_KEEP_ATTRIBUTE_MODIFIERS: i8 = 1;
const RESPAWN_KEEP_ENTITY_DATA: i8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDimension {
    pub min_y: i32,
    pub height: i32,
}

impl Default for WorldDimension {
    fn default() -> Self {
        Self {
            min_y: -64,
            height: 384,
        }
    }
}

impl WorldDimension {
    pub fn min_section_y(self) -> i32 {
        self.min_y.div_euclid(16)
    }

    pub fn section_count(self) -> usize {
        usize::try_from(self.height.max(0))
            .unwrap_or(0)
            .div_ceil(16)
    }

    pub fn contains_section_y(self, section_y: i32) -> bool {
        let Some(section_offset) = section_y.checked_sub(self.min_section_y()) else {
            return false;
        };
        let Ok(offset) = usize::try_from(section_offset) else {
            return false;
        };
        offset < self.section_count()
    }

    pub fn contains_y(self, y: i32) -> bool {
        y >= self.min_y && y < self.min_y + self.height
    }
}

/// Client-owned scalar projection of vanilla `DimensionType.NETWORK_CODEC`.
/// Fields without a current simulation consumer remain available through the
/// level snapshot instead of being discarded during registry publication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldDimensionTypeProperties {
    #[serde(default)]
    pub has_fixed_time: bool,
    #[serde(default)]
    pub has_ender_dragon_fight: bool,
    #[serde(default = "world_dimension_coordinate_scale_default")]
    pub coordinate_scale: f64,
    #[serde(default)]
    pub logical_height: i32,
    #[serde(default)]
    pub infiniburn_tag: Option<String>,
    #[serde(default)]
    pub monster_spawn_light_level: Option<WorldIntProvider>,
    #[serde(default)]
    pub monster_spawn_block_light_limit: Option<i32>,
}

impl Default for WorldDimensionTypeProperties {
    fn default() -> Self {
        Self {
            has_fixed_time: false,
            has_ender_dragon_fight: false,
            coordinate_scale: world_dimension_coordinate_scale_default(),
            logical_height: 0,
            infiniburn_tag: None,
            monster_spawn_light_level: None,
            monster_spawn_block_light_limit: None,
        }
    }
}

const fn world_dimension_coordinate_scale_default() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldLevelInfo {
    pub dimension: String,
    pub dimension_type_id: i32,
    pub dimension_type_name: Option<String>,
    #[serde(default)]
    pub dimension_type_properties: WorldDimensionTypeProperties,
    /// Vanilla passes the CommonPlayerSpawnInfo seed only to ClientLevel's
    /// BiomeManager. It is not client authority over server world generation.
    #[serde(default)]
    pub biome_zoom_seed: i64,
    #[serde(default)]
    pub default_clock_resource_id: Option<String>,
    #[serde(default)]
    pub timeline_resource_ids: Vec<String>,
    #[serde(default = "world_level_has_sky_light_default")]
    pub has_sky_light: bool,
    #[serde(default)]
    pub has_ceiling: bool,
    #[serde(default)]
    pub cardinal_lighting: WorldCardinalLighting,
    #[serde(default)]
    pub skybox: WorldSkybox,
    #[serde(default)]
    pub ambient_light: f32,
    #[serde(default)]
    pub environment_attributes: ClientEnvironmentAttributes,
    #[serde(default)]
    pub last_death_location: Option<WorldGlobalPos>,
    pub sea_level: i32,
    pub is_debug: bool,
    pub is_flat: bool,
}

/// Vanilla `net.minecraft.world.level.CardinalLighting.Type`, selected per
/// dimension by `DimensionType.cardinalLightType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorldCardinalLighting {
    #[default]
    Default,
    Nether,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WorldSkybox {
    None,
    #[default]
    Overworld,
    End,
}

impl WorldLevelInfo {
    pub fn has_sky_light(&self) -> bool {
        self.has_sky_light
    }

    pub fn has_ceiling(&self) -> bool {
        self.has_ceiling
    }

    pub fn can_have_weather(&self) -> bool {
        self.has_sky_light && !self.has_ceiling && self.dimension != "minecraft:the_end"
    }

    pub fn cardinal_lighting(&self) -> WorldCardinalLighting {
        self.cardinal_lighting
    }

    pub fn skybox(&self) -> WorldSkybox {
        self.skybox
    }

    pub fn ambient_light(&self) -> f32 {
        self.ambient_light
    }

    pub fn environment_attributes(&self) -> ClientEnvironmentAttributes {
        self.environment_attributes.clone()
    }

    pub fn timeline_resource_ids(&self) -> &[String] {
        &self.timeline_resource_ids
    }
}

const fn world_level_has_sky_light_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldGameplayState {
    pub game_type: i32,
    pub game_type_name: String,
    pub previous_game_type: Option<i32>,
    pub previous_game_type_name: Option<String>,
    pub show_death_screen: bool,
    pub do_limited_crafting: bool,
    /// Vanilla `LocalPlayer.isReducedDebugInfo`, initialized from
    /// `ClientboundLoginPacket.reducedDebugInfo` and updated by local-player
    /// `Player.handleEntityEvent(22/23)` dispatch.
    #[serde(default)]
    pub reduced_debug_info: bool,
    /// Vanilla `ClientLevel.Data.isHardcore` from the `ClientboundLoginPacket`
    /// `hardcore` flag (read by `Gui.extractHearts` for the hardcore heart
    /// sprite variants, Gui.java:834).
    #[serde(default)]
    pub hardcore: bool,
}

impl Default for WorldGameplayState {
    fn default() -> Self {
        Self {
            game_type: VANILLA_GAME_TYPE_SURVIVAL_ID,
            game_type_name: GameType::Survival.serialized_name().to_string(),
            previous_game_type: None,
            previous_game_type_name: None,
            show_death_screen: true,
            do_limited_crafting: false,
            reduced_debug_info: false,
            hardcore: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldTimeState {
    pub game_time: i64,
    pub day_time: i64,
    pub clock_updates: Vec<ClockUpdateState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldWeatherState {
    pub raining: bool,
    pub rain_level: f32,
    pub thunder_level: f32,
    pub last_game_event_id: Option<u8>,
    pub last_game_event_param: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldLevelLoadPhase {
    #[default]
    Idle,
    WaitingForServer,
    WaitingForPlayerChunk,
    ClientLoaded,
}

/// Vanilla `LevelLoadingScreen.Reason`, chosen by
/// `ClientPacketListener.determineLevelLoadingReason`
/// (ClientPacketListener.java:1341-1353).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldLevelLoadReason {
    #[default]
    Other,
    NetherPortal,
    EndPortal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLevelLoadState {
    pub phase: WorldLevelLoadPhase,
    pub starts: usize,
    pub loading_packets_received: usize,
    #[serde(default)]
    pub loads_completed: usize,
    #[serde(default)]
    pub timeouts: usize,
    #[serde(default)]
    pub reason: WorldLevelLoadReason,
}

impl WorldLevelLoadState {
    /// Vanilla shows `LevelLoadingScreen` for the whole
    /// `startWaitingForNewLevel` window and closes it once
    /// `LevelLoadTracker.isLevelReady()` holds.
    pub fn screen_is_up(self) -> bool {
        matches!(
            self.phase,
            WorldLevelLoadPhase::WaitingForServer | WorldLevelLoadPhase::WaitingForPlayerChunk
        )
    }
}

impl Default for WorldWeatherState {
    fn default() -> Self {
        Self {
            raining: false,
            rain_level: 0.0,
            thunder_level: 0.0,
            last_game_event_id: None,
            last_game_event_param: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldTickingState {
    pub tick_rate: f32,
    pub frozen: bool,
    pub frozen_ticks_to_run: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RespawnApplyResult {
    pub closed_server_container_id: Option<i32>,
    pub dimension_changed: bool,
}

impl Default for WorldTickingState {
    fn default() -> Self {
        Self {
            tick_rate: 20.0,
            frozen: false,
            frozen_ticks_to_run: 0,
        }
    }
}

impl WorldStore {
    pub fn with_dimension(dimension: WorldDimension) -> Self {
        Self {
            dimension,
            registries: crate::RegistrySet::vanilla_26_1().into(),
            ..Self::default()
        }
    }

    pub fn apply_login(&mut self, login: &ProtocolPlayLogin) {
        self.counters.play_logins_received += 1;
        // `ClientPacketListener.handleLogin` constructs the new ClientLevel and
        // calls `Minecraft.setLevel`; it does not call `Gui.onDisconnected`, so
        // a toast queued before the play login remains owned by ToastManager.
        // Configuration re-entry has already gone through `clearClientLevel`
        // and therefore clears the manager on that earlier transition.
        self.reset_client_level_state(false);
        self.reset_chat_session_state();
        self.available_levels = login
            .levels
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into();
        self.server_enforces_secure_chat = Some(login.enforces_secure_chat);
        if !self.seen_insecure_chat_warning && !self.effectively_enforces_secure_chat() {
            self.seen_insecure_chat_warning = true;
            self.insecure_chat_warning_count = self.insecure_chat_warning_count.saturating_add(1);
            self.push_system_toast_event(crate::ClientSystemToastEvent::InsecureServerWarning);
        }
        // Vanilla constructs the new ClientChunkCache and ClientLevel from these
        // login fields before later SetChunkCacheRadius/SetSimulationDistance
        // packets may replace them.
        self.chunk_view = crate::ChunkViewState {
            center: None,
            radius: Some(login.chunk_radius),
        };
        self.reset_local_player_for_login(
            login.player_id,
            i32::from(login.common_spawn_info.game_type),
        );
        self.local_player_id = Some(login.player_id);
        self.local_player.simulation_distance = Some(login.simulation_distance);
        self.gameplay.show_death_screen = login.show_death_screen;
        self.gameplay.do_limited_crafting = login.do_limited_crafting;
        self.gameplay.reduced_debug_info = login.reduced_debug_info;
        self.gameplay.hardcore = login.hardcore;
        self.apply_spawn_info(&login.common_spawn_info);
        self.add_local_player_entity(None, 0);
        self.start_client_level_load(WorldLevelLoadReason::Other);
    }

    pub fn apply_respawn(&mut self, respawn: &ProtocolRespawn) -> RespawnApplyResult {
        self.counters.respawns_received += 1;
        self.push_screen_request(crate::ScreenRequest::CloseDeathScreen);
        self.push_screen_request(crate::ScreenRequest::CloseGameRules);
        let level_load_reason = level_loading_reason(
            self.local_player_is_dead(),
            &respawn.common_spawn_info.dimension,
            self.level.as_ref().map(|level| level.dimension.as_str()),
        );
        let dimension_changed = self
            .level
            .as_ref()
            .is_some_and(|level| level.dimension != respawn.common_spawn_info.dimension);
        if dimension_changed {
            // Vanilla `ClientPacketListener.handleRespawn` constructs and
            // publishes a fresh `ClientLevel` only when the dimension key
            // changes. A same-dimension respawn replaces `LocalPlayer` while
            // retaining the exact level context (including CacheSlot values).
            self.advance_client_level_generation();
        }
        let closed_server_container_id = self.close_container_for_respawn();
        self.cooldowns.clear();
        self.reset_creative_inventory_for_new_level();
        let old_local_player = self.reset_local_player_for_respawn(
            respawn.data_to_keep,
            i32::from(respawn.common_spawn_info.game_type),
            dimension_changed,
        );
        if dimension_changed {
            // Vanilla ClientDebugSubscriber.dropLevel clears only values tied
            // to the replaced ClientLevel, then keeps the active subscription
            // set. DebugSample and the other listener-owned diagnostics survive.
            self.clear_debug_subscriber_values();
        }
        self.apply_spawn_info(&respawn.common_spawn_info);
        self.add_local_player_entity(old_local_player, respawn.data_to_keep);
        self.start_client_level_load(level_load_reason);
        RespawnApplyResult {
            closed_server_container_id,
            dimension_changed,
        }
    }

    pub fn apply_world_time(&mut self, time: ProtocolPlayTime) -> &WorldTimeState {
        const MAX_TRACKED_WORLD_CLOCKS: usize = 1_024;
        self.counters.world_time_packets += 1;
        // Vanilla ClientClockManager ticks every existing holder instance to the
        // packet game time, then overwrites only the holders present in this map.
        let mut clock_updates = self
            .world_time
            .take()
            .map(|mut previous| {
                let game_time_delta = time.game_time.saturating_sub(previous.game_time);
                for clock in &mut previous.clock_updates {
                    advance_client_clock(clock, game_time_delta);
                }
                previous.clock_updates
            })
            .unwrap_or_default();
        let known_clock_count = self
            .registries
            .contents
            .get("minecraft:world_clock")
            .map(|contents| contents.entries.len());
        let mut index_by_clock_id: rustc_hash::FxHashMap<i32, usize> = clock_updates
            .iter()
            .enumerate()
            .map(|(index, clock)| (clock.clock_id, index))
            .collect();
        for update in time.clock_updates {
            if update.clock_id < 0 {
                continue;
            }
            if let Some(count) = known_clock_count {
                if (update.clock_id as usize) >= count {
                    continue;
                }
            }
            if let Some(&index) = index_by_clock_id.get(&update.clock_id) {
                clock_updates[index] = update;
            } else if clock_updates.len() < MAX_TRACKED_WORLD_CLOCKS {
                let index = clock_updates.len();
                index_by_clock_id.insert(update.clock_id, index);
                clock_updates.push(update);
            }
        }
        let day_time = default_clock_total_ticks(&clock_updates, self.current_default_clock_id());
        self.world_time = Some(WorldTimeState {
            game_time: time.game_time,
            day_time,
            clock_updates,
        });
        self.world_time
            .as_ref()
            .expect("world time was just updated")
    }

    pub fn apply_game_event(&mut self, event: ProtocolGameEvent) -> WorldWeatherState {
        self.counters.game_event_packets += 1;
        self.weather.last_game_event_id = Some(event.event_id);
        self.weather.last_game_event_param = event.param;

        match event.event_id {
            VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID => {
                self.push_translatable_client_system_chat_message(
                    "block.minecraft.spawn.not_valid",
                );
            }
            VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID => match self.level_load.phase {
                WorldLevelLoadPhase::WaitingForServer => {
                    self.level_load.loading_packets_received =
                        self.level_load.loading_packets_received.saturating_add(1);
                    self.level_load.phase = WorldLevelLoadPhase::WaitingForPlayerChunk;
                }
                WorldLevelLoadPhase::WaitingForPlayerChunk => {
                    self.level_load.loading_packets_received =
                        self.level_load.loading_packets_received.saturating_add(1);
                }
                WorldLevelLoadPhase::Idle | WorldLevelLoadPhase::ClientLoaded => {}
            },
            VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID => {
                self.weather.rain_level = 0.0;
                self.weather.raining = false;
            }
            VANILLA_CLIENT_GAME_EVENT_STOP_RAINING_ID => {
                self.weather.rain_level = 1.0;
                self.weather.raining = true;
            }
            VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID => {
                self.weather.rain_level = event.param.clamp(0.0, 1.0);
                self.weather.raining = self.weather.rain_level > 0.0;
            }
            VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID => {
                self.weather.thunder_level = event.param.clamp(0.0, 1.0);
            }
            VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID => {
                self.set_game_type_from_game_event_param(event.param);
            }
            VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID => {
                self.open_game_event_screen(|generation| ClientGameEventScreenState::Win {
                    generation,
                });
            }
            VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID if event.param == 0.0 => {
                self.open_game_event_screen(|generation| ClientGameEventScreenState::DemoHelp {
                    generation,
                });
            }
            VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID => {
                let hint = match event.param {
                    101.0 => Some(("demo.help.movement", &["W", "A", "S", "D"][..])),
                    102.0 => Some(("demo.help.jump", &["Space"][..])),
                    103.0 => Some(("demo.help.inventory", &["E"][..])),
                    104.0 => Some(("demo.day.6", &["F2"][..])),
                    _ => None,
                };
                if let Some((translation_key, arguments)) = hint {
                    self.push_translatable_client_system_chat_message_with_arguments(
                        translation_key,
                        arguments,
                        ChatNarrationKind::System,
                    );
                }
            }
            VANILLA_CLIENT_GAME_EVENT_IMMEDIATE_RESPAWN_ID => {
                self.gameplay.show_death_screen = event.param == 0.0;
            }
            VANILLA_CLIENT_GAME_EVENT_LIMITED_CRAFTING_ID => {
                self.gameplay.do_limited_crafting = event.param == 1.0;
            }
            _ => {}
        }

        self.weather
    }

    pub fn apply_ticking_state(&mut self, ticking: ProtocolTickingState) -> WorldTickingState {
        self.counters.ticking_state_packets += 1;
        self.ticking.tick_rate = ticking.clamped_tick_rate();
        self.ticking.frozen = ticking.frozen;
        self.ticking
    }

    pub fn apply_ticking_step(&mut self, step: ProtocolTickingStep) -> WorldTickingState {
        self.counters.ticking_step_packets += 1;
        self.ticking.frozen_ticks_to_run = step.tick_steps;
        self.ticking
    }

    pub fn consume_running_render_ticks(&mut self, ticks: u32) -> u32 {
        if ticks == 0 {
            return 0;
        }

        let step_ticks = positive_tick_steps(self.ticking.frozen_ticks_to_run).min(ticks);
        self.ticking.frozen_ticks_to_run = self
            .ticking
            .frozen_ticks_to_run
            .saturating_sub(step_ticks as i32);
        if self.ticking.frozen {
            step_ticks
        } else {
            ticks
        }
    }

    pub fn advance_client_time(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }

        self.advance_hotbar_pop_times(ticks);

        if self.local_player.portal_cooldown > 0 {
            let ticks = i32::try_from(ticks).unwrap_or(i32::MAX);
            self.local_player.portal_cooldown = self
                .local_player
                .portal_cooldown
                .saturating_sub(ticks)
                .max(0);
        }

        let default_clock_id = self.current_default_clock_id();
        let Some(time) = self.world_time.as_mut() else {
            return;
        };

        let start_game_time = time.game_time;
        let game_time_delta = i64::from(ticks);
        time.game_time = time.game_time.saturating_add(game_time_delta);
        for clock in &mut time.clock_updates {
            advance_client_clock(clock, game_time_delta);
        }
        time.day_time = default_clock_total_ticks(&time.clock_updates, default_clock_id);

        // Vanilla `Minecraft.tick` runs entity ticks before `ClientLevel.tickTime`,
        // so entity tick side effects read the pre-increment gameTime for each tick.
        for offset in 1..=ticks {
            let game_time = start_game_time.saturating_add(i64::from(offset - 1));
            if game_time % 5 == 0 {
                self.entities.append_ominous_item_spawner_particle_states();
            }
        }
    }

    pub fn set_sky_flash_time(&mut self, ticks: i32) {
        self.sky_flash_time = ticks.max(0);
    }

    pub fn trigger_sky_flash(&mut self) {
        self.set_sky_flash_time(2);
    }

    pub fn advance_sky_flash_time(&mut self, ticks: u32) {
        let ticks = i32::try_from(ticks).unwrap_or(i32::MAX);
        self.sky_flash_time = self.sky_flash_time.saturating_sub(ticks).max(0);
    }

    pub fn clear_client_level(&mut self) {
        self.reset_client_level_state(true);
    }

    fn reset_client_level_state(&mut self, clear_toast_manager: bool) {
        self.advance_client_level_generation();
        let auto_jump_enabled = self.local_player_auto_jump_enabled();
        self.reset_local_player_ambient_audio();
        self.stop_tutorial_for_missing_level();
        // Login and configuration re-entry construct a fresh
        // ClientDebugSubscriber. Unlike a dimension-only dropLevel, the new
        // listener starts with an empty remote request set and resends the
        // current renderer-derived set on its next tick.
        self.clear_requested_debug_subscriptions();
        self.dimension = WorldDimension::default();
        self.level = None;
        self.gameplay = WorldGameplayState::default();
        self.world_border = crate::WorldBorderState::default();
        self.world_time = None;
        self.weather = crate::WorldWeatherState::default();
        self.sky_flash_time = 0;
        self.ticking = crate::WorldTickingState::default();
        self.level_load = WorldLevelLoadState::default();
        self.available_levels.clear();
        self.server_enforces_secure_chat = None;
        self.chunk_view = crate::ChunkViewState::default();
        self.last_block_changed_ack = None;
        self.local_player = crate::LocalPlayerState::default();
        self.set_local_player_auto_jump_enabled(auto_jump_enabled);
        self.local_player_old_pose = None;
        self.local_player_id = None;
        self.local_player_vehicle_id = None;
        self.removed_player_vehicle_id = None;
        self.last_projectile_power = None;
        self.inventory = Default::default();
        self.cooldowns.clear();
        self.pending_local_interaction_particle_actions.clear();
        self.pending_client_particle_spawns.clear();
        self.pending_local_level_events.clear();
        self.pending_portal_transition_trigger_sound = false;
        self.pending_portal_transition_container_close = false;
        self.client_stats = Default::default();
        self.client_combat = Default::default();
        self.client_effects = Default::default();
        self.recipe_book = Default::default();
        self.client_ui.last_mount_screen = None;
        self.client_ui.last_open_book = None;
        self.client_ui.last_ghost_recipe = None;
        self.reset_creative_inventory_for_new_level();
        self.clear_active_player_info();
        // A configuration re-entry constructs a new ClientPacketListener. Its
        // Scoreboard and ClientWaypointManager are fresh final fields and are
        // not carried by CommonListenerCookie.
        self.scoreboard = crate::ScoreboardState::default().into();
        self.waypoints = crate::ClientWaypointsState::default().into();
        self.counters.waypoints_tracked = 0;
        // Vanilla login constructs a new ClientLevel with an empty map table;
        // only the dimension-change respawn path carries maps over via
        // getAllMapData→addMapData, which never reaches this reset.
        self.maps.clear();
        self.map_lru.clear();
        // Vanilla clearLevel/disconnect run `GameRenderer.resetData`, which
        // clears the ScreenEffectRenderer item activation; the dimension
        // respawn path (`Minecraft.setLevel`) never reaches it.
        self.item_activation = None;
        self.last_map_color_patch = None;
        self.counters.maps_tracked = 0;
        self.counters.map_decorations_tracked = 0;
        self.reset_command_tree_for_new_listener();
        self.reset_command_suggestions_for_new_listener();
        self.reset_recipes_for_new_listener();
        self.reset_advancements_for_new_listener();
        if clear_toast_manager {
            self.clear_client_toasts_for_disconnect();
        }
        self.clear_hud_for_level_teardown();
        self.clear_level_bound_state();
    }

    pub fn level_load(&self) -> &WorldLevelLoadState {
        &self.level_load
    }

    pub fn complete_client_level_load(&mut self, timed_out: bool) -> bool {
        if self.level_load.phase != WorldLevelLoadPhase::WaitingForPlayerChunk {
            return false;
        }
        self.level_load.phase = WorldLevelLoadPhase::ClientLoaded;
        self.level_load.loads_completed = self.level_load.loads_completed.saturating_add(1);
        if timed_out {
            self.level_load.timeouts = self.level_load.timeouts.saturating_add(1);
        }
        true
    }

    pub fn client_level_generation(&self) -> u64 {
        self.client_level_generation
    }

    pub fn available_levels(&self) -> &BTreeSet<String> {
        &self.available_levels
    }

    pub fn server_enforces_secure_chat(&self) -> Option<bool> {
        self.server_enforces_secure_chat
    }

    /// Replaces both process-level Services validation families. The net owner
    /// has already checked that every entry is a bounded RSA-4096 X.509 key.
    pub fn replace_services_public_keys(
        &mut self,
        profile_property_keys: Vec<Vec<u8>>,
        player_certificate_keys: Vec<Vec<u8>>,
    ) {
        self.services_profile_property_keys = profile_property_keys.into();
        self.services_player_certificate_keys = player_certificate_keys.into();
    }

    pub fn services_profile_property_keys(&self) -> &[Vec<u8>] {
        &self.services_profile_property_keys
    }

    pub fn profile_key_validation_available(&self) -> bool {
        !self.services_player_certificate_keys.is_empty()
    }

    pub fn effectively_enforces_secure_chat(&self) -> bool {
        self.profile_key_validation_available() && self.server_enforces_secure_chat == Some(true)
    }

    pub fn seen_insecure_chat_warning(&self) -> bool {
        self.seen_insecure_chat_warning
    }

    pub fn insecure_chat_warning_count(&self) -> usize {
        self.insecure_chat_warning_count
    }

    fn start_client_level_load(&mut self, reason: WorldLevelLoadReason) {
        // ClientPacketListener.startWaitingForNewLevel replaces the current screen.
        self.push_screen_request(crate::ScreenRequest::CloseScreen);
        self.level_load.phase = WorldLevelLoadPhase::WaitingForServer;
        self.level_load.starts = self.level_load.starts.saturating_add(1);
        self.level_load.loading_packets_received = 0;
        self.level_load.reason = reason;
    }

    fn advance_client_level_generation(&mut self) {
        self.client_level_generation = self.client_level_generation.saturating_add(1);
        self.debug_query = crate::ClientDebugQueryState::default();
    }

    fn apply_spawn_info(&mut self, spawn_info: &ProtocolSpawnInfo) {
        let profile = dimension_profile(spawn_info.dimension_type_id, &spawn_info.dimension);
        let dimension_type_entry =
            self.registry_entry_by_holder_id(DIMENSION_TYPE_REGISTRY, spawn_info.dimension_type_id);
        let dimension_type = self
            .registries
            .dimension_type_client_projection(spawn_info.dimension_type_id);
        let facts = dimension_type.map(|projection| &projection.facts);
        let scalars = facts.map(|facts| &facts.scalars);
        let dimension_type_entry_name = dimension_type_entry.map(|entry| entry.id.clone());
        let dimension_type_name = dimension_type_entry_name
            .clone()
            .or_else(|| profile.name.map(str::to_string));
        let dimension = scalars
            .map(|scalars| WorldDimension {
                min_y: scalars.min_y,
                height: scalars.height,
            })
            .unwrap_or(profile.dimension);
        let has_sky_light = scalars
            .map(|scalars| scalars.has_skylight)
            .unwrap_or_else(|| {
                built_in_dimension_has_sky_light(
                    spawn_info.dimension_type_id,
                    &spawn_info.dimension,
                )
            });
        let has_ceiling = scalars
            .map(|scalars| scalars.has_ceiling)
            .unwrap_or_else(|| {
                built_in_dimension_has_ceiling(spawn_info.dimension_type_id, &spawn_info.dimension)
            });
        let cardinal_lighting = scalars
            .map(|scalars| match scalars.cardinal_light {
                DimensionCardinalLight::Default => WorldCardinalLighting::Default,
                DimensionCardinalLight::Nether => WorldCardinalLighting::Nether,
            })
            .unwrap_or_else(|| {
                built_in_dimension_cardinal_lighting(
                    spawn_info.dimension_type_id,
                    &spawn_info.dimension,
                )
            });
        let skybox = scalars
            .map(|scalars| match scalars.skybox {
                DimensionSkybox::None => WorldSkybox::None,
                DimensionSkybox::Overworld => WorldSkybox::Overworld,
                DimensionSkybox::End => WorldSkybox::End,
            })
            .unwrap_or_else(|| {
                built_in_dimension_skybox(spawn_info.dimension_type_id, &spawn_info.dimension)
            });
        let ambient_light = scalars
            .map(|scalars| scalars.ambient_light)
            .unwrap_or_else(|| {
                built_in_dimension_ambient_light(
                    spawn_info.dimension_type_id,
                    &spawn_info.dimension,
                )
            });
        let dimension_type_properties = WorldDimensionTypeProperties {
            has_fixed_time: scalars
                .map(|scalars| scalars.has_fixed_time)
                .unwrap_or(false),
            has_ender_dragon_fight: scalars
                .map(|scalars| scalars.has_ender_dragon_fight)
                .unwrap_or(false),
            coordinate_scale: scalars
                .map(|scalars| scalars.coordinate_scale)
                .unwrap_or(1.0),
            logical_height: scalars
                .map(|scalars| scalars.logical_height)
                .unwrap_or(dimension.height),
            infiniburn_tag: facts.map(|facts| facts.infiniburn_tag.clone()),
            monster_spawn_light_level: facts.map(|facts| facts.monster_spawn_light_level.clone()),
            monster_spawn_block_light_limit: scalars
                .map(|scalars| scalars.monster_spawn_block_light_limit),
        };
        let environment_attributes = dimension_type
            .map(|projection| projection.environment_attributes.clone())
            .unwrap_or_default();
        let timeline_resource_ids = dimension_type
            .map(|projection| projection.timeline_resource_ids.clone())
            .unwrap_or_default();
        let default_clock_resource_id = facts
            .and_then(|facts| facts.default_clock.clone())
            .or_else(|| {
                let (dimension_type_id, dimension_type_name) = dimension_type_entry_name
                    .as_deref()
                    .map(|name| (-1, name))
                    .unwrap_or((spawn_info.dimension_type_id, spawn_info.dimension.as_str()));
                built_in_dimension_default_clock_resource_id(dimension_type_id, dimension_type_name)
                    .map(str::to_string)
            });
        let dimension_key_changed = self
            .level
            .as_ref()
            .is_some_and(|level| level.dimension != spawn_info.dimension);
        if self.dimension != dimension || dimension_key_changed {
            self.clear_level_bound_state();
        }
        self.dimension = dimension;
        self.level = Some(WorldLevelInfo {
            dimension: spawn_info.dimension.clone(),
            dimension_type_id: spawn_info.dimension_type_id,
            dimension_type_name,
            dimension_type_properties,
            biome_zoom_seed: spawn_info.seed,
            default_clock_resource_id,
            timeline_resource_ids,
            has_sky_light,
            has_ceiling,
            cardinal_lighting,
            skybox,
            ambient_light,
            environment_attributes,
            last_death_location: spawn_info.last_death_location.clone(),
            sea_level: spawn_info.sea_level,
            is_debug: spawn_info.is_debug,
            is_flat: spawn_info.is_flat,
        });
        self.local_player.portal_cooldown = spawn_info.portal_cooldown;
        self.refresh_day_time_from_default_clock();
        self.set_game_type_from_spawn_info(
            i32::from(spawn_info.game_type),
            i32::from(spawn_info.previous_game_type),
        );
    }

    fn set_game_type_from_spawn_info(&mut self, game_type: i32, previous_game_type: i32) {
        let game_type = GameType::from_id(game_type);
        self.gameplay.game_type = game_type.id();
        self.gameplay.game_type_name = game_type.serialized_name().to_string();
        if previous_game_type == VANILLA_GAME_TYPE_NOT_SET_ID {
            self.gameplay.previous_game_type = None;
            self.gameplay.previous_game_type_name = None;
        } else {
            let previous_game_type = GameType::from_id(previous_game_type);
            self.gameplay.previous_game_type = Some(previous_game_type.id());
            self.gameplay.previous_game_type_name =
                Some(previous_game_type.serialized_name().to_string());
        }
    }

    fn reset_local_player_for_respawn(
        &mut self,
        data_to_keep: i8,
        game_type_id: i32,
        dimension_changed: bool,
    ) -> Option<crate::entities::EntityRecord> {
        self.reset_local_player_ambient_audio();
        let keep_entity_data = data_to_keep & RESPAWN_KEEP_ENTITY_DATA != 0;
        let auto_jump_enabled = self.local_player_auto_jump_enabled();
        let old_local_player = std::mem::take(&mut self.local_player);
        self.set_local_player_auto_jump_enabled(auto_jump_enabled);
        self.local_player_old_pose = None;
        let old_pose = old_local_player.pose;

        // handleRespawn constructs a fresh LocalPlayer. Only state explicitly
        // copied below survives; listener/level-owned projections are retained
        // separately where their Vanilla owner survives the replacement.
        self.local_player.abilities = Some(crate::client::game_type_player_abilities(
            GameType::from_id(game_type_id),
        ));
        self.local_player.default_spawn = (!dimension_changed)
            .then_some(old_local_player.default_spawn)
            .flatten();
        self.local_player.simulation_distance = old_local_player.simulation_distance;
        self.local_player.sprinting = keep_entity_data && old_local_player.sprinting;
        self.local_player.swimming = keep_entity_data && old_local_player.swimming;
        // Vanilla `ClientPacketListener.handleRespawn` (java:1304, 1332-1333):
        // bind the camera back to the new LocalPlayer and preserve the portal
        // transition intensities across the respawn boundary.
        self.local_player.camera = crate::CameraState {
            entity_id: self.local_player_id,
            follows_player: true,
            entity_known: self.local_player_id.is_some(),
        };
        self.local_player.portal_effect_intensity = old_local_player.portal_effect_intensity;
        self.local_player.previous_portal_effect_intensity =
            old_local_player.previous_portal_effect_intensity;
        self.local_player.pose = keep_entity_data.then_some(old_pose).flatten().map(|pose| {
            crate::LocalPlayerPoseState {
                position: Default::default(),
                delta_movement: pose.delta_movement,
                on_ground: false,
                horizontal_collision: false,
                minor_horizontal_collision: false,
                fall_distance: 0.0,
                sneaking: pose.sneaking,
                swimming: pose.swimming,
                no_jump_delay_ticks: 0,
                y_rot: pose.y_rot,
                x_rot: pose.x_rot,
                last_teleport_id: 0,
            }
        });
        self.removed_player_vehicle_id = None;

        let Some(local_player_id) = self.local_player_id else {
            self.local_player_vehicle_id = None;
            return None;
        };
        self.clear_local_player_mount(local_player_id);
        self.entities.take_record(local_player_id)
    }

    /// Adds the LocalPlayer that `handleLogin` / `handleRespawn` create to the
    /// level, after `apply_spawn_info` may have replaced that level.
    fn add_local_player_entity(
        &mut self,
        old: Option<crate::entities::EntityRecord>,
        data_to_keep: i8,
    ) {
        let Some(id) = self.local_player_id else {
            return;
        };
        match old {
            Some(old) => {
                let _ = self.entities.respawn_local_player(
                    id,
                    old,
                    data_to_keep & RESPAWN_KEEP_ENTITY_DATA != 0,
                    data_to_keep & RESPAWN_KEEP_ATTRIBUTE_MODIFIERS != 0,
                );
            }
            None => self
                .entities
                .spawn_local_player(id, self.local_player_profile_id.unwrap_or_default()),
        }
        self.update_entity_count();
        self.update_active_mob_effect_count();
    }

    fn set_game_type_from_game_event_param(&mut self, param: f32) {
        let game_type = GameType::from_id(rounded_game_event_param(param));
        if self.gameplay.game_type != game_type.id() {
            self.gameplay.previous_game_type = Some(self.gameplay.game_type);
            self.gameplay.previous_game_type_name = Some(self.gameplay.game_type_name.clone());
        }
        self.gameplay.game_type = game_type.id();
        self.gameplay.game_type_name = game_type.serialized_name().to_string();
        // Vanilla `LocalPlayer.onGameModeChanged`: entering spectator zeroes
        // the vertical motion so the camera does not keep falling.
        if game_type.id() == VANILLA_GAME_TYPE_SPECTATOR_ID {
            if let Some(pose) = self.local_player.pose.as_mut() {
                pose.delta_movement.y = 0.0;
            }
        }
    }

    fn clear_level_bound_state(&mut self) {
        self.push_screen_request(crate::ScreenRequest::CloseLevelScreens);
        self.clear_debug_subscriber_values();
        self.clear_game_test_highlight_markers();
        self.clear_queued_light_updates();
        self.chunks.clear();
        self.clear_terrain_revisions();
        self.first_chunk = None;
        self.chunk_view.center = None;
        self.weather = crate::WorldWeatherState::default();
        self.sky_flash_time = 0;
        self.world_border = crate::WorldBorderState::default();
        self.client_effects = crate::ClientEffectsState::default();
        self.pending_local_interaction_particle_actions.clear();
        self.pending_client_particle_spawns.clear();
        self.pending_local_level_events.clear();
        self.pending_portal_transition_trigger_sound = false;
        self.pending_portal_transition_container_close = false;
        self.last_block_changed_ack = None;
        self.block_destructions.clear();
        self.block_events.clear();
        self.moving_pistons.clear();
        self.moving_piston_index.clear();
        self.chest_lids.clear();
        self.chest_lid_index.clear();
        self.bell_shakes.clear();
        self.bell_shake_index.clear();
        self.bell_resonations.clear();
        self.shulker_box_lids.clear();
        self.shulker_box_lid_index.clear();
        self.decorated_pot_wobbles.clear();
        self.decorated_pot_wobble_index.clear();
        self.skull_blocks.clear();
        self.beacon_blocks.clear();
        self.conduit_blocks.clear();
        self.end_gateways.clear();
        self.spawner_blocks.clear();
        self.enchanting_table_books.clear();
        self.trial_spawner_blocks.clear();
        self.vault_blocks.clear();
        self.enchanting_book_random = crate::EnchantingBookRandom::default();
        self.level_events.clear();
        self.client_audio.playing_jukebox_songs.clear();
        self.client_audio.playing_jukebox_song_index.0.clear();
        self.clear_tickable_entity_sounds();
        self.local_prediction_sequence = 0;
        self.local_prediction_last_teleport_sequence = None;
        self.local_block_predictions.clear();
        self.entities.clear();
        self.counters.block_destructions_tracked = 0;
        self.counters.block_events_tracked = 0;
        self.counters.level_events_tracked = 0;
        self.counters.local_block_predictions_tracked = 0;
        self.update_active_mob_effect_count();
        self.update_entity_count();
    }

    pub fn dimension(&self) -> WorldDimension {
        self.dimension
    }

    pub fn level_info(&self) -> Option<&WorldLevelInfo> {
        self.level.as_ref()
    }

    pub fn gameplay(&self) -> &WorldGameplayState {
        &self.gameplay
    }

    /// Vanilla `player.level().getLevelData().isHardcore()` — the login
    /// `hardcore` flag `Gui.extractHearts` reads to pick the hardcore heart
    /// sprites (Gui.java:834).
    pub fn is_hardcore(&self) -> bool {
        self.gameplay.hardcore
    }

    /// Vanilla `ClientLevelData.getHorizonHeight`: flat levels drop the
    /// horizon to the level floor, every other level uses the fixed 63.0
    /// overworld horizon.
    pub fn client_horizon_height(&self) -> f64 {
        if self.level_info().is_some_and(|info| info.is_flat) {
            f64::from(self.dimension.min_y)
        } else {
            63.0
        }
    }

    /// Vanilla `ClientLevelData.voidDarknessOnsetRange`: flat levels compress
    /// the void-darkness fog onset to one block, every other level fades over
    /// 32 blocks below the level floor.
    pub fn client_void_darkness_onset_range(&self) -> f32 {
        if self.level_info().is_some_and(|info| info.is_flat) {
            1.0
        } else {
            32.0
        }
    }

    /// Vanilla `SkyRenderer.shouldRenderDarkDisc`: the below-horizon dark
    /// disc renders while the local player's eye sits under the level's
    /// horizon height.
    pub fn local_player_eye_below_horizon(&self) -> bool {
        self.local_player_pose().is_some_and(|pose| {
            pose.position.y + self.local_player_eye_height(pose) - self.client_horizon_height()
                < 0.0
        })
    }

    pub fn local_player_is_spectator(&self) -> bool {
        self.gameplay.game_type == VANILLA_GAME_TYPE_SPECTATOR_ID
    }

    pub fn local_player_is_creative(&self) -> bool {
        self.gameplay.game_type == VANILLA_GAME_TYPE_CREATIVE_ID
    }

    pub fn local_player_is_adventure(&self) -> bool {
        self.gameplay.game_type == VANILLA_GAME_TYPE_ADVENTURE_ID
    }

    pub fn local_player_is_survival(&self) -> bool {
        self.local_player_id.is_some() && self.gameplay.game_type == VANILLA_GAME_TYPE_SURVIVAL_ID
    }

    pub fn local_player_has_reduced_debug_info(&self) -> bool {
        self.local_player_id.is_some() && self.gameplay.reduced_debug_info
    }

    /// Updates the local player's reduced-debug gate from vanilla `Player.handleEntityEvent`.
    /// Remote players retain the field on their `Player` instance, but no vanilla client
    /// consumer reads it; all reduced-debug consumers query the local player.
    pub(crate) fn set_local_player_reduced_debug_info_from_entity_event(
        &mut self,
        entity_id: i32,
        reduced_debug_info: bool,
    ) -> bool {
        if self.local_player_id != Some(entity_id) {
            return false;
        }
        self.gameplay.reduced_debug_info = reduced_debug_info;
        true
    }

    pub fn world_time(&self) -> Option<&WorldTimeState> {
        self.world_time.as_ref()
    }

    fn current_default_clock_id(&self) -> Option<i32> {
        // Vanilla Level.getDefaultClockTime resolves DimensionType.defaultClock
        // through the active RegistryAccess. A pre-login store has the repo's
        // vanilla-overworld default; a loaded dimension with no clock stays None.
        let resource_id = match self.level.as_ref() {
            Some(level) => level.default_clock_resource_id.as_deref()?,
            None => "minecraft:overworld",
        };
        self.registry_holder_id_by_resource_id("minecraft:world_clock", resource_id)
    }

    fn refresh_day_time_from_default_clock(&mut self) {
        let default_clock_id = self.current_default_clock_id();
        if let Some(time) = self.world_time.as_mut() {
            time.day_time = default_clock_total_ticks(&time.clock_updates, default_clock_id);
        }
    }

    pub fn weather(&self) -> WorldWeatherState {
        self.weather
    }

    pub fn sky_flash_time(&self) -> i32 {
        self.sky_flash_time
    }

    pub fn ticking(&self) -> WorldTickingState {
        self.ticking
    }
}

fn built_in_dimension_ambient_light(dimension_type_id: i32, dimension: &str) -> f32 {
    match dimension_profile(dimension_type_id, dimension).name {
        Some("minecraft:the_nether") => 0.1,
        Some("minecraft:the_end") => 0.25,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests;
