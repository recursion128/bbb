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
use bbb_protocol::packets::{
    ClockUpdate as ProtocolClockUpdate, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
    GameEvent as ProtocolGameEvent, PlayLogin as ProtocolPlayLogin, PlayTime as ProtocolPlayTime,
    Respawn as ProtocolRespawn, TickingState as ProtocolTickingState,
    TickingStep as ProtocolTickingStep,
};
use serde::{Deserialize, Serialize};

use crate::{
    registries::{ClientEnvironmentAttributes, WorldIntProvider, DIMENSION_TYPE_REGISTRY},
    BlockPos, ChatNarrationKind, ClientGameEventScreenState, WorldStore,
};

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
pub struct WorldGlobalPos {
    pub dimension: String,
    pub pos: BlockPos,
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
pub struct ClockUpdateState {
    pub clock_id: i32,
    pub total_ticks: i64,
    pub partial_tick: f32,
    pub rate: f32,
}

impl From<ProtocolClockUpdate> for ClockUpdateState {
    fn from(update: ProtocolClockUpdate) -> Self {
        Self {
            clock_id: update.clock_id,
            total_ticks: update.total_ticks,
            partial_tick: update.partial_tick,
            rate: update.rate,
        }
    }
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

fn world_global_pos_from_protocol(pos: &bbb_protocol::packets::GlobalPos) -> WorldGlobalPos {
    WorldGlobalPos {
        dimension: pos.dimension.clone(),
        pos: BlockPos {
            x: pos.pos.x,
            y: pos.pos.y,
            z: pos.pos.z,
        },
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
        self.clear_client_level();
        self.reset_chat_session_state();
        self.available_levels = login.levels.iter().cloned().collect();
        self.server_enforces_secure_chat = Some(login.enforces_secure_chat);
        if !self.seen_insecure_chat_warning && !self.effectively_enforces_secure_chat() {
            self.seen_insecure_chat_warning = true;
            self.insecure_chat_warning_count = self.insecure_chat_warning_count.saturating_add(1);
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
        self.start_client_level_load(WorldLevelLoadReason::Other);
    }

    pub fn apply_respawn(&mut self, respawn: &ProtocolRespawn) -> RespawnApplyResult {
        self.counters.respawns_received += 1;
        self.clear_death_screen();
        self.close_game_rules_screen();
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
        self.reset_client_toasts_for_new_level();
        self.requeue_active_tutorial_toast();
        let closed_server_container_id = self.close_container_for_respawn();
        self.reset_creative_inventory_for_new_level();
        self.reset_local_player_for_respawn(
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
        self.start_client_level_load(level_load_reason);
        RespawnApplyResult {
            closed_server_container_id,
            dimension_changed,
        }
    }

    pub fn apply_world_time(&mut self, time: ProtocolPlayTime) -> &WorldTimeState {
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
        for update in time.clock_updates {
            let update = ClockUpdateState::from(update);
            match clock_updates
                .iter_mut()
                .find(|clock| clock.clock_id == update.clock_id)
            {
                Some(clock) => *clock = update,
                None => clock_updates.push(update),
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
        self.reset_creative_inventory_for_new_level();
        self.clear_active_player_info();
        // A configuration re-entry constructs a new ClientPacketListener. Its
        // Scoreboard and ClientWaypointManager are fresh final fields and are
        // not carried by CommonListenerCookie.
        self.scoreboard = crate::ScoreboardState::default();
        self.waypoints = crate::ClientWaypointsState::default();
        self.counters.waypoints_tracked = 0;
        // Vanilla login constructs a new ClientLevel with an empty map table;
        // only the dimension-change respawn path carries maps over via
        // getAllMapData→addMapData, which never reaches this reset.
        self.maps.clear();
        self.last_map_color_patch = None;
        self.counters.maps_tracked = 0;
        self.counters.map_decorations_tracked = 0;
        self.reset_command_tree_for_new_listener();
        self.reset_command_suggestions_for_new_listener();
        self.reset_recipes_for_new_listener();
        self.reset_advancements_for_new_listener();
        self.reset_client_toasts_for_new_level();
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
        self.services_profile_property_keys = profile_property_keys;
        self.services_player_certificate_keys = player_certificate_keys;
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
        let dimension_type_entry_name = dimension_type_entry.map(|entry| entry.id.clone());
        let dimension_type_name = dimension_type_entry_name
            .clone()
            .or_else(|| profile.name.map(str::to_string));
        let dimension = dimension_type
            .as_ref()
            .and_then(|projection| projection.dimension)
            .unwrap_or(profile.dimension);
        let has_sky_light = dimension_type
            .as_ref()
            .and_then(|projection| projection.has_sky_light)
            .unwrap_or_else(|| {
                built_in_dimension_has_sky_light(
                    spawn_info.dimension_type_id,
                    &spawn_info.dimension,
                )
            });
        let has_ceiling = dimension_type
            .as_ref()
            .and_then(|projection| projection.has_ceiling)
            .unwrap_or_else(|| {
                built_in_dimension_has_ceiling(spawn_info.dimension_type_id, &spawn_info.dimension)
            });
        let cardinal_lighting = dimension_type
            .as_ref()
            .and_then(|projection| projection.cardinal_lighting)
            .unwrap_or_else(|| {
                built_in_dimension_cardinal_lighting(
                    spawn_info.dimension_type_id,
                    &spawn_info.dimension,
                )
            });
        let skybox = dimension_type
            .as_ref()
            .and_then(|projection| projection.skybox)
            .unwrap_or_else(|| {
                built_in_dimension_skybox(spawn_info.dimension_type_id, &spawn_info.dimension)
            });
        let ambient_light = dimension_type
            .as_ref()
            .and_then(|projection| projection.ambient_light)
            .unwrap_or_else(|| {
                built_in_dimension_ambient_light(
                    spawn_info.dimension_type_id,
                    &spawn_info.dimension,
                )
            });
        let dimension_type_properties = WorldDimensionTypeProperties {
            has_fixed_time: dimension_type
                .as_ref()
                .and_then(|projection| projection.has_fixed_time)
                .unwrap_or(false),
            has_ender_dragon_fight: dimension_type
                .as_ref()
                .and_then(|projection| projection.has_ender_dragon_fight)
                .unwrap_or(false),
            coordinate_scale: dimension_type
                .as_ref()
                .and_then(|projection| projection.coordinate_scale)
                .unwrap_or(1.0),
            logical_height: dimension_type
                .as_ref()
                .and_then(|projection| projection.logical_height)
                .unwrap_or(dimension.height),
            infiniburn_tag: dimension_type
                .as_ref()
                .and_then(|projection| projection.infiniburn_tag.clone()),
            monster_spawn_light_level: dimension_type
                .as_ref()
                .and_then(|projection| projection.monster_spawn_light_level.clone()),
            monster_spawn_block_light_limit: dimension_type
                .as_ref()
                .and_then(|projection| projection.monster_spawn_block_light_limit),
        };
        let environment_attributes = dimension_type
            .as_ref()
            .map(|projection| projection.environment_attributes.clone())
            .unwrap_or_default();
        if let Some(err) = dimension_type
            .as_ref()
            .and_then(|projection| projection.environment_error)
        {
            self.record_apply_error("dimension_type_environment_attributes", err);
        }
        let timeline_resource_ids = dimension_type
            .as_ref()
            .map(|projection| projection.timeline_resource_ids.clone())
            .unwrap_or_default();
        if let Some(err) = dimension_type
            .as_ref()
            .and_then(|projection| projection.timeline_error.as_deref())
        {
            self.record_apply_error("dimension_type_timelines", err);
        }
        let default_clock_resource_id = dimension_type
            .as_ref()
            .and_then(|projection| projection.default_clock_resource_id.clone())
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
            last_death_location: spawn_info
                .last_death_location
                .as_ref()
                .map(world_global_pos_from_protocol),
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
    ) {
        self.reset_local_player_ambient_audio();
        let keep_entity_data = data_to_keep & RESPAWN_KEEP_ENTITY_DATA != 0;
        let keep_attribute_modifiers = data_to_keep & RESPAWN_KEEP_ATTRIBUTE_MODIFIERS != 0;
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
            return;
        };
        self.clear_local_player_mount(local_player_id);
        self.entities.rebuild_local_player_entity_for_respawn(
            local_player_id,
            keep_entity_data,
            keep_attribute_modifiers,
        );
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
        self.clear_death_screen();
        self.clear_game_event_screen();
        self.clear_social_interactions_screen();
        self.close_game_rules_screen();
        self.close_command_block_editor();
        self.close_minecart_command_block_editor();
        self.close_structure_block_editor();
        self.close_jigsaw_block_editor();
        self.close_test_block_editor();
        self.close_test_instance_block_editor();
        self.clear_debug_subscriber_values();
        self.clear_game_test_highlight_markers();
        self.clear_queued_light_updates();
        self.chunks.clear();
        self.clear_terrain_revisions();
        self.first_chunk = None;
        self.block_destructions.clear();
        self.block_events.clear();
        self.moving_pistons.clear();
        self.chest_lids.clear();
        self.bell_shakes.clear();
        self.shulker_box_lids.clear();
        self.decorated_pot_wobbles.clear();
        self.enchanting_table_books.clear();
        self.trial_spawner_blocks.clear();
        self.vault_blocks.clear();
        self.enchanting_book_random = crate::EnchantingBookRandom::default();
        self.level_events.clear();
        self.client_audio.playing_jukebox_songs.clear();
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
            pose.position.y + pose.eye_height() - self.client_horizon_height() < 0.0
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

struct DimensionProfile {
    dimension: WorldDimension,
    name: Option<&'static str>,
}

/// Vanilla `ClientPacketListener.determineLevelLoadingReason`
/// (ClientPacketListener.java:1341-1353): a death respawn always stays
/// `OTHER`, otherwise either side of the dimension change picks the portal
/// background.
fn level_loading_reason(
    player_died: bool,
    dimension: &str,
    previous_dimension: Option<&str>,
) -> WorldLevelLoadReason {
    if player_died {
        return WorldLevelLoadReason::Other;
    }
    let touches = |name: &str| dimension == name || previous_dimension == Some(name);
    if touches("minecraft:the_nether") {
        WorldLevelLoadReason::NetherPortal
    } else if touches("minecraft:the_end") {
        WorldLevelLoadReason::EndPortal
    } else {
        WorldLevelLoadReason::Other
    }
}

fn dimension_profile(dimension_type_id: i32, dimension: &str) -> DimensionProfile {
    match (dimension_type_id, dimension) {
        (0, _) | (_, "minecraft:overworld") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld"),
        },
        (1, _) | (_, "minecraft:the_nether") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_nether"),
        },
        (2, _) | (_, "minecraft:the_end") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_end"),
        },
        (3, _) | (_, "minecraft:overworld_caves") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld_caves"),
        },
        _ => DimensionProfile {
            dimension: WorldDimension::default(),
            name: None,
        },
    }
}

fn built_in_dimension_cardinal_lighting(
    dimension_type_id: i32,
    dimension: &str,
) -> WorldCardinalLighting {
    if dimension_profile(dimension_type_id, dimension).name == Some("minecraft:the_nether") {
        WorldCardinalLighting::Nether
    } else {
        WorldCardinalLighting::Default
    }
}

fn built_in_dimension_has_sky_light(dimension_type_id: i32, dimension: &str) -> bool {
    !matches!(
        dimension_profile(dimension_type_id, dimension).name,
        Some("minecraft:the_nether" | "minecraft:the_end")
    )
}

fn built_in_dimension_has_ceiling(dimension_type_id: i32, dimension: &str) -> bool {
    dimension_profile(dimension_type_id, dimension).name == Some("minecraft:the_nether")
}

fn built_in_dimension_skybox(dimension_type_id: i32, dimension: &str) -> WorldSkybox {
    match dimension_profile(dimension_type_id, dimension).name {
        Some("minecraft:the_nether") => WorldSkybox::None,
        Some("minecraft:the_end") => WorldSkybox::End,
        _ => WorldSkybox::Overworld,
    }
}

fn built_in_dimension_ambient_light(dimension_type_id: i32, dimension: &str) -> f32 {
    match dimension_profile(dimension_type_id, dimension).name {
        Some("minecraft:the_nether") => 0.1,
        Some("minecraft:the_end") => 0.25,
        _ => 0.0,
    }
}

fn built_in_dimension_default_clock_resource_id(
    dimension_type_id: i32,
    dimension: &str,
) -> Option<&'static str> {
    match dimension_profile(dimension_type_id, dimension).name {
        Some("minecraft:overworld" | "minecraft:overworld_caves") => Some("minecraft:overworld"),
        Some("minecraft:the_end") => Some("minecraft:the_end"),
        _ => None,
    }
}

pub(crate) fn rounded_game_event_param(param: f32) -> i32 {
    // ClientPacketListener.handleGameEvent uses Mth.floor(paramFloat + 0.5F)
    // once for every integer-valued game-event consumer.
    if param.is_finite() {
        (param + 0.5).floor() as i32
    } else {
        VANILLA_GAME_TYPE_SURVIVAL_ID
    }
}

fn advance_client_clock(clock: &mut ClockUpdateState, game_time_delta: i64) {
    if game_time_delta == 0 {
        return;
    }

    let new_partial_ticks =
        f64::from(clock.partial_tick) + (game_time_delta as f64) * f64::from(clock.rate);
    let full_ticks = new_partial_ticks.floor() as i64;
    clock.partial_tick = (new_partial_ticks - full_ticks as f64) as f32;
    clock.total_ticks = clock.total_ticks.saturating_add(full_ticks);
}

fn default_clock_total_ticks(
    clock_updates: &[ClockUpdateState],
    default_clock_id: Option<i32>,
) -> i64 {
    default_clock_id
        .and_then(|clock_id| {
            clock_updates
                .iter()
                .find(|clock| clock.clock_id == clock_id)
        })
        .map(|clock| clock.total_ticks)
        .unwrap_or(0)
}

fn positive_tick_steps(tick_steps: i32) -> u32 {
    u32::try_from(tick_steps).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
    use bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID;
    use bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_MINING_FATIGUE_ID;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AdvancementCriterionProgressSummary,
        AdvancementDisplaySummary, AdvancementFrameType, AdvancementIconSummary,
        AdvancementProgressSummary, AdvancementSummary,
        AttributeModifier as ProtocolAttributeModifier,
        AttributeSnapshot as ProtocolAttributeSnapshot,
        BlockDestruction as ProtocolBlockDestruction, BlockEvent as ProtocolBlockEvent,
        BlockPos as ProtocolBlockPos, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind, EntityEvent as ProtocolEntityEvent, EquipmentSlot,
        EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate, GlobalPos as ProtocolGlobalPos,
        ItemStackSummary as ProtocolItemStackSummary, LevelEvent as ProtocolLevelEvent,
        MapItemData as ProtocolMapItemData, MobEffectFlags, OpenScreen as ProtocolOpenScreen,
        PlayerExperience as ProtocolPlayerExperience, PlayerHealth as ProtocolPlayerHealth,
        RegistryData, RegistryDataEntry, RegistryTags, SetCamera as ProtocolSetCamera,
        SetEntityData as ProtocolSetEntityData, TagNetworkPayload, Transfer as ProtocolTransfer,
        UpdateAdvancements, UpdateAttributes as ProtocolUpdateAttributes,
        UpdateMobEffect as ProtocolUpdateMobEffect, UpdateTags, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    use crate::entities::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use crate::{
        BlockPos, CameraState, ChunkColumn, ChunkPos, ChunkState, LightData, LocalPlayerPoseState,
    };

    #[test]
    fn play_login_updates_world_dimension_and_level_info() {
        let mut store = WorldStore::new();
        store.chunks.push(stale_chunk().into());

        store.apply_login(&ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec![
                "minecraft:overworld".to_string(),
                "minecraft:the_nether".to_string(),
                "minecraft:the_end".to_string(),
                "minecraft:overworld".to_string(),
            ],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: true,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: Some(ProtocolGlobalPos {
                    dimension: "minecraft:overworld".to_string(),
                    pos: ProtocolBlockPos { x: 8, y: 64, z: -3 },
                }),
                portal_cooldown: 7,
                sea_level: 32,
            },
            enforces_secure_chat: true,
        });

        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: 0,
                height: 256,
            }
        );
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.chunk_cache_center(), None);
        assert_eq!(store.chunk_cache_radius(), Some(8));
        assert_eq!(store.local_player().simulation_distance, Some(6));
        assert_eq!(store.counters().chunk_cache_radius_updates_received, 0);
        assert_eq!(store.counters().simulation_distance_packets, 0);
        assert_eq!(store.counters().play_logins_received, 1);
        assert_eq!(store.client_level_generation(), 1);
        assert_eq!(
            store.available_levels(),
            &BTreeSet::from([
                "minecraft:overworld".to_string(),
                "minecraft:the_nether".to_string(),
                "minecraft:the_end".to_string(),
            ])
        );
        assert_eq!(store.server_enforces_secure_chat(), Some(true));
        assert_eq!(store.local_player().portal_cooldown, 7);
        for x in [11, 12] {
            let mut chunk = stale_chunk();
            chunk.pos = ChunkPos { x, z: 0 };
            store.chunks.push(chunk.into());
        }
        store.apply_set_chunk_cache_center(bbb_protocol::packets::SetChunkCacheCenter {
            chunk_x: 0,
            chunk_z: 0,
        });
        assert_eq!(store.chunk_positions(), vec![ChunkPos { x: 11, z: 0 }]);
        let level = store.level_info().unwrap();
        assert_eq!(level.biome_zoom_seed, 12345);
        assert_eq!(level.dimension, "minecraft:the_nether");
        assert_eq!(level.dimension_type_id, 1);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_nether")
        );
        assert_eq!(level.cardinal_lighting(), WorldCardinalLighting::Nether);
        assert_eq!(
            level.last_death_location,
            Some(WorldGlobalPos {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos { x: 8, y: 64, z: -3 },
            })
        );
        assert_eq!(level.sea_level, 32);
        assert_eq!(
            store.gameplay(),
            &WorldGameplayState {
                game_type: 1,
                game_type_name: "creative".to_string(),
                previous_game_type: None,
                previous_game_type_name: None,
                show_death_screen: true,
                do_limited_crafting: false,
                reduced_debug_info: true,
                hardcore: false,
            }
        );
        assert!(store.local_player_has_reduced_debug_info());
        store.advance_client_time(3);
        assert_eq!(store.local_player().portal_cooldown, 4);
    }

    #[test]
    fn respawn_clears_advancement_toast_handoff_without_dropping_tree() {
        let mut store = WorldStore::new();
        store.apply_update_advancements(UpdateAdvancements {
            reset: true,
            added: vec![AdvancementSummary {
                id: "test:root".to_string(),
                parent: None,
                display: Some(AdvancementDisplaySummary {
                    title: bbb_protocol::RichTextComponent::plain("Root"),
                    description: bbb_protocol::RichTextComponent::plain(""),
                    icon: AdvancementIconSummary {
                        item_id: 1,
                        count: 1,
                        component_patch: Default::default(),
                    },
                    frame_type: AdvancementFrameType::Task,
                    show_toast: true,
                    hidden: false,
                    background: None,
                    x: 0.0,
                    y: 0.0,
                }),
                requirements: vec![vec!["done".to_string()]],
                sends_telemetry_event: false,
            }],
            removed: Vec::new(),
            progress: Vec::new(),
            show_advancements: false,
        });
        store.apply_update_advancements(UpdateAdvancements {
            reset: false,
            added: Vec::new(),
            removed: Vec::new(),
            progress: vec![AdvancementProgressSummary {
                id: "test:root".to_string(),
                criteria: vec![AdvancementCriterionProgressSummary {
                    name: "done".to_string(),
                    obtained_epoch_millis: Some(1),
                }],
            }],
            show_advancements: true,
        });
        assert_eq!(store.client_toast_events().len(), 1);

        store.apply_respawn(&respawn_packet(0));

        assert!(store.client_toast_events().is_empty());
        assert_eq!(store.client_toasts().next_sequence, 0);
        assert!(store
            .client_advancements()
            .advancements
            .contains_key("test:root"));
    }

    #[test]
    fn login_rebuilds_local_player_camera_and_game_mode_abilities() {
        let mut store = WorldStore::new();
        store.local_player_id = Some(7);
        store.chunks.push(stale_chunk().into());
        store.apply_add_entity(protocol_add_entity(99));
        store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: false,
            flying: true,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.2,
            walking_speed: 0.3,
        });
        store.apply_player_health(ProtocolPlayerHealth {
            health: 4.0,
            food: 7,
            saturation: 0.5,
        });
        store.apply_player_experience(ProtocolPlayerExperience {
            progress: 0.25,
            level: 3,
            total: 40,
        });
        assert!(store.set_local_selected_hotbar_slot(8));
        store.local_player.camera = CameraState {
            entity_id: Some(99),
            follows_player: false,
            entity_known: true,
        };
        store.local_player.pose = Some(LocalPlayerPoseState::default());

        let mut login = login_packet(0, "minecraft:overworld");
        login.common_spawn_info.game_type = 3;
        login.common_spawn_info.portal_cooldown = 4;
        store.apply_login(&login);

        assert_eq!(store.local_player_id(), Some(42));
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.entity_count(), 0);
        assert_eq!(
            store.local_player().abilities,
            Some(crate::LocalPlayerAbilitiesState {
                invulnerable: true,
                flying: true,
                can_fly: true,
                instabuild: false,
                flying_speed: 0.05,
                walking_speed: 0.1,
            })
        );
        assert!(store.local_player().health.is_none());
        assert!(store.local_player().experience.is_none());
        assert_eq!(store.local_player().selected_hotbar_slot, 0);
        assert_eq!(store.local_player().simulation_distance, Some(6));
        assert_eq!(store.local_player().portal_cooldown, 4);
        assert!(store.local_player().pose.is_none());
        assert_eq!(
            store.local_player().camera,
            CameraState {
                entity_id: Some(42),
                follows_player: true,
                entity_known: true,
            }
        );
        assert_eq!(store.counters().player_abilities_packets, 1);
        assert_eq!(store.counters().player_health_packets, 1);
        assert_eq!(store.counters().player_experience_packets, 1);
    }

    #[test]
    fn login_warns_once_when_profile_keys_cannot_be_validated() {
        let mut store = WorldStore::new();

        store.apply_login(&login_packet(0, "minecraft:overworld"));

        assert_eq!(store.server_enforces_secure_chat(), Some(true));
        assert!(!store.profile_key_validation_available());
        assert!(!store.effectively_enforces_secure_chat());
        assert!(store.seen_insecure_chat_warning());
        assert_eq!(store.insecure_chat_warning_count(), 1);
    }

    #[test]
    fn insecure_chat_warning_guard_survives_transfer_level_reset() {
        let mut store = WorldStore::new();
        store.replace_services_public_keys(Vec::new(), vec![vec![1]]);
        store.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(store.effectively_enforces_secure_chat());
        assert!(!store.seen_insecure_chat_warning());
        assert_eq!(store.insecure_chat_warning_count(), 0);

        let mut insecure_login = login_packet(0, "minecraft:overworld");
        insecure_login.enforces_secure_chat = false;
        store.apply_login(&insecure_login);
        assert!(!store.effectively_enforces_secure_chat());
        assert!(store.seen_insecure_chat_warning());
        assert_eq!(store.insecure_chat_warning_count(), 1);

        store.apply_transfer(ProtocolTransfer {
            host: "next.example.com".to_string(),
            port: 25566,
        });
        store.clear_client_level();
        assert!(store.profile_key_validation_available());
        assert!(store.seen_insecure_chat_warning());
        assert_eq!(store.insecure_chat_warning_count(), 1);

        store.apply_login(&insecure_login);
        assert_eq!(store.insecure_chat_warning_count(), 1);
    }

    #[test]
    fn respawn_updates_dimension_and_clears_old_chunks() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        store
            .available_levels
            .insert("minecraft:overworld".to_string());
        store.server_enforces_secure_chat = Some(true);
        store.chunks.push(stale_chunk().into());
        store.first_chunk = Some(ChunkPos { x: 1, z: -2 });
        store.apply_add_entity(protocol_add_entity(123));
        assert!(store.apply_map_item_data(ProtocolMapItemData {
            map_id: 42,
            scale: 2,
            locked: true,
            decorations: None,
            color_patch: None,
        }));
        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
            title: bbb_protocol::RichTextComponent::plain("Chest"),
        });
        store.client_debug_game.last_debug_block_value = Some(crate::DebugBlockValueState {
            pos: BlockPos { x: 1, y: 2, z: 3 },
            subscription_id: 6,
            value_present: true,
        });
        store.client_debug_game.last_debug_chunk_value = Some(crate::DebugChunkValueState {
            pos: ChunkPos { x: 1, z: -2 },
            subscription_id: 11,
            value_present: false,
        });
        store.client_debug_game.last_debug_entity_value = Some(crate::DebugEntityValueState {
            entity_id: 123,
            subscription_id: 3,
            value_present: false,
        });
        store.client_debug_game.last_debug_event = Some(crate::DebugEventState {
            subscription_id: 14,
        });
        store.client_debug_game.last_debug_sample = Some(crate::DebugSampleState {
            sample_len: 2,
            sample_type: "tick_time".to_string(),
        });
        store.local_player.default_spawn = Some(crate::DefaultSpawnState {
            dimension: "minecraft:the_nether".to_string(),
            pos: BlockPos { x: 8, y: 64, z: 8 },
            yaw: 0.0,
            pitch: 0.0,
        });
        store.set_local_prediction_sequence(12);
        store.mark_local_prediction_teleport();
        store.record_local_block_prediction(12, BlockPos { x: 3, y: 4, z: 5 }, 1, 2, None);

        let same_dimension = store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 6,
                sea_level: 32,
            },
            data_to_keep: 3,
        });
        assert_eq!(same_dimension.closed_server_container_id, Some(7));
        assert!(!same_dimension.dimension_changed);
        assert_eq!(store.open_container_id(), None);
        assert_eq!(store.chunk_count(), 1);
        assert_eq!(store.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
        assert_eq!(store.entity_count(), 1);
        assert_eq!(store.level_info().unwrap().biome_zoom_seed, 12345);
        assert_eq!(store.local_player().portal_cooldown, 6);
        assert!(store.last_debug_block_value().is_some());
        assert!(store.local_player().default_spawn.is_some());
        assert_eq!(store.client_level_generation(), 0);
        assert_eq!(store.local_prediction_sequence(), 12);
        assert_eq!(store.local_prediction_last_teleport_sequence(), Some(12));
        assert_eq!(store.local_block_predictions().len(), 1);
        let available_levels = store.available_levels().clone();
        let server_enforces_secure_chat = store.server_enforces_secure_chat();

        let changed_dimension = store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 2,
                dimension: "minecraft:the_end".to_string(),
                seed: 98765,
                game_type: 1,
                previous_game_type: 1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 2,
                sea_level: 63,
            },
            data_to_keep: 3,
        });
        assert_eq!(changed_dimension.closed_server_container_id, None);
        assert!(changed_dimension.dimension_changed);

        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: 0,
                height: 256,
            }
        );
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.first_chunk(), None);
        assert_eq!(store.entity_count(), 0);
        assert_eq!(store.local_prediction_sequence(), 0);
        assert_eq!(store.local_prediction_last_teleport_sequence(), None);
        assert!(store.local_block_predictions().is_empty());
        assert!(store.last_debug_block_value().is_none());
        assert!(store.last_debug_chunk_value().is_none());
        assert!(store.last_debug_entity_value().is_none());
        assert!(store.last_debug_event().is_none());
        assert_eq!(
            store.last_debug_sample(),
            Some(&crate::DebugSampleState {
                sample_len: 2,
                sample_type: "tick_time".to_string(),
            })
        );
        assert!(store.local_player().default_spawn.is_none());
        let map = store.map_item(42).expect("map survives dimension respawn");
        assert_eq!(map.scale, 2);
        assert!(map.locked);
        assert_eq!(store.counters().maps_tracked, 1);
        assert_eq!(store.counters().entities_tracked, 0);
        assert_eq!(store.counters().respawns_received, 2);
        let level = store.level_info().unwrap();
        assert_eq!(level.biome_zoom_seed, 98765);
        assert_eq!(store.local_player().portal_cooldown, 2);
        assert_eq!(store.client_level_generation(), 1);
        assert_eq!(store.available_levels(), &available_levels);
        assert_eq!(
            store.server_enforces_secure_chat(),
            server_enforces_secure_chat
        );
        assert_eq!(level.dimension, "minecraft:the_end");
        assert_eq!(level.dimension_type_id, 2);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_end")
        );
        assert_eq!(level.cardinal_lighting(), WorldCardinalLighting::Default);
        assert_eq!(store.gameplay().game_type, 1);
        assert_eq!(store.gameplay().game_type_name, "creative");
        assert_eq!(store.gameplay().previous_game_type, Some(1));
        assert_eq!(
            store.gameplay().previous_game_type_name.as_deref(),
            Some("creative")
        );
    }

    #[test]
    fn client_level_generation_follows_construct_replace_and_clear_identity_only() {
        let mut store = WorldStore::new();
        store.apply_login(&login_packet(0, "minecraft:overworld"));
        let login_generation = store.client_level_generation();
        assert_eq!(login_generation, 1);
        let retained_login = store.publication_snapshot();

        store.apply_respawn(&respawn_packet(0));
        assert_eq!(store.client_level_generation(), login_generation);
        assert_eq!(retained_login.client_level_generation(), login_generation);
        assert_eq!(
            retained_login
                .level_info()
                .expect("retained login level")
                .dimension
                .as_str(),
            "minecraft:overworld"
        );

        let mut changed_dimension = respawn_packet(0);
        changed_dimension.common_spawn_info.dimension_type_id = 1;
        changed_dimension.common_spawn_info.dimension = "minecraft:the_nether".to_string();
        store.apply_respawn(&changed_dimension);
        assert_eq!(
            store.client_level_generation(),
            login_generation.saturating_add(1)
        );
        assert_eq!(
            retained_login.client_level_generation(),
            login_generation,
            "retained publications must not observe replacement"
        );

        store.clear_client_level();
        assert_eq!(
            store.client_level_generation(),
            login_generation.saturating_add(2)
        );
        assert!(store.level_info().is_none());
    }

    #[test]
    fn client_level_generation_production_source_has_one_mutator_and_exact_edges() {
        let source = include_str!("level.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("level production source");
        assert_eq!(
            source
                .matches("self.advance_client_level_generation();")
                .count(),
            2,
            "only dimension-changing respawn and clear may advance level identity"
        );
        assert_eq!(
            source
                .matches("client_level_generation.saturating_add(1)")
                .count(),
            1,
            "level generation must have one canonical mutation owner"
        );
        let respawn = source
            .split_once("pub fn apply_respawn(")
            .expect("respawn owner")
            .1
            .split_once("pub fn apply_world_time(")
            .expect("respawn owner end")
            .0;
        let dimension_check = respawn
            .find("let dimension_changed =")
            .expect("dimension identity decision");
        let conditional = respawn
            .find("if dimension_changed {")
            .expect("conditional level replacement");
        let advance = respawn
            .find("self.advance_client_level_generation();")
            .expect("level replacement generation edge");
        assert!(dimension_check < conditional && conditional < advance);
    }

    #[test]
    fn local_player_auto_jump_option_survives_level_replacement_and_timer_resets() {
        let mut store = respawn_state_with_local_player_data();
        store.set_local_player_auto_jump_enabled(false);
        store.arm_local_player_auto_jump();

        store.apply_respawn(&respawn_packet(0));

        assert!(!store.local_player_auto_jump_enabled());
        assert!(!store.local_player_auto_jump_is_armed());
        store.arm_local_player_auto_jump();
        store.clear_client_level();
        assert!(!store.local_player_auto_jump_enabled());
        assert!(!store.local_player_auto_jump_is_armed());

        store.arm_local_player_auto_jump();
        store.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(!store.local_player_auto_jump_enabled());
        assert!(!store.local_player_auto_jump_is_armed());
    }

    #[test]
    fn respawn_without_keep_data_resets_local_player_runtime_state() {
        let mut store = respawn_state_with_local_player_data();

        store.apply_respawn(&respawn_packet(0));

        assert_eq!(
            store.local_player().abilities,
            Some(crate::LocalPlayerAbilitiesState {
                invulnerable: false,
                flying: false,
                can_fly: false,
                instabuild: false,
                flying_speed: 0.05,
                walking_speed: 0.1,
            })
        );
        assert!(store.local_player().health.is_none());
        assert!(store.local_player().experience.is_none());
        assert_eq!(store.local_player().selected_hotbar_slot, 0);
        assert_eq!(
            store.local_player().permission_level,
            crate::LocalPlayerPermissionLevel::NoPermissions
        );
        assert!(store.local_player().default_spawn.is_some());
        assert_eq!(store.local_player().simulation_distance, Some(12));
        assert_eq!(store.local_player_pose(), None);
        assert!(!store.local_player_is_sprinting());
        assert_eq!(store.local_player().camera, CameraState::default());
        assert_eq!(
            store.local_player().interaction,
            crate::LocalPlayerInteractionState::default()
        );

        let entity = store.probe_entity(123).unwrap();
        assert_eq!(entity.data, 0);
        assert_eq!(entity.position, Default::default());
        assert_eq!(entity.delta_movement, Default::default());
        assert_eq!(entity.y_rot, -180.0);
        assert_eq!(entity.x_rot, 0.0);
        assert!(entity.data_values.is_empty());
        assert!(entity.equipment.is_empty());
        assert_eq!(entity.leash_holder_id, None);
        assert_eq!(entity.last_event_id, None);
        assert_eq!(entity.main_supporting_block_pos, None);
        assert!(!entity.on_ground_no_blocks);
        assert!(entity.mob_effects.is_empty());
        assert_eq!(store.counters().active_mob_effects_tracked, 0);
        assert_eq!(entity.attributes.len(), 1);
        assert_eq!(entity.attributes[0].base, 0.1);
        assert!(entity.attributes[0].modifiers.is_empty());
    }

    #[test]
    fn respawn_keep_all_data_preserves_entity_data_pose_and_attribute_modifiers() {
        let mut store = respawn_state_with_local_player_data();
        let old_pose = store.local_player_pose().unwrap();
        store.local_player_old_pose = Some(old_pose);

        store.apply_respawn(&respawn_packet(3));

        assert!(store.local_player().health.is_none());
        assert!(store.local_player().experience.is_none());
        assert_eq!(
            store.local_player_pose(),
            Some(LocalPlayerPoseState {
                position: Default::default(),
                delta_movement: old_pose.delta_movement,
                on_ground: false,
                horizontal_collision: false,
                minor_horizontal_collision: false,
                fall_distance: 0.0,
                sneaking: old_pose.sneaking,
                swimming: old_pose.swimming,
                no_jump_delay_ticks: 0,
                y_rot: old_pose.y_rot,
                x_rot: old_pose.x_rot,
                last_teleport_id: 0,
            })
        );
        assert_eq!(store.local_player().camera, CameraState::default());
        assert_eq!(store.local_player_old_pose, None);
        assert_eq!(
            store.local_player().interaction,
            crate::LocalPlayerInteractionState::default()
        );

        let entity = store.probe_entity(123).unwrap();
        assert_eq!(entity.data_values, vec![entity_byte_data(0, 0x02)]);
        assert!(entity.mob_effects.is_empty());
        assert_eq!(store.counters().active_mob_effects_tracked, 0);
        assert_eq!(entity.attributes.len(), 1);
        assert_eq!(entity.attributes[0].base, 0.1);
        assert_eq!(
            entity.attributes[0].modifiers,
            vec![ProtocolAttributeModifier {
                id: "minecraft:test_speed".to_string(),
                amount: 0.25,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            }]
        );
    }

    #[test]
    fn respawn_keep_bits_preserve_entity_data_and_attribute_modifiers_independently() {
        let mut attributes_only = respawn_state_with_local_player_data();
        attributes_only.apply_respawn(&respawn_packet(RESPAWN_KEEP_ATTRIBUTE_MODIFIERS));
        assert!(attributes_only.local_player_pose().is_none());
        assert!(!attributes_only.local_player_is_sprinting());
        let entity = attributes_only.probe_entity(123).unwrap();
        assert!(entity.data_values.is_empty());
        assert_eq!(entity.attributes[0].modifiers.len(), 1);

        let mut entity_data_only = respawn_state_with_local_player_data();
        let old_pose = entity_data_only.local_player_pose().unwrap();
        entity_data_only.apply_respawn(&respawn_packet(RESPAWN_KEEP_ENTITY_DATA));
        assert!(entity_data_only.local_player_is_sprinting());
        assert_eq!(
            entity_data_only.local_player_pose(),
            Some(LocalPlayerPoseState {
                position: Default::default(),
                delta_movement: old_pose.delta_movement,
                on_ground: false,
                horizontal_collision: false,
                minor_horizontal_collision: false,
                fall_distance: 0.0,
                sneaking: old_pose.sneaking,
                swimming: old_pose.swimming,
                no_jump_delay_ticks: 0,
                y_rot: old_pose.y_rot,
                x_rot: old_pose.x_rot,
                last_teleport_id: 0,
            })
        );
        let entity = entity_data_only.probe_entity(123).unwrap();
        assert_eq!(entity.data_values, vec![entity_byte_data(0, 0x02)]);
        assert!(entity.attributes[0].modifiers.is_empty());
    }

    #[test]
    fn clear_client_level_removes_level_bound_state_without_resetting_packet_counters() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        store.local_player_old_pose = Some(LocalPlayerPoseState::default());
        store.level = Some(WorldLevelInfo {
            dimension: "minecraft:the_nether".to_string(),
            dimension_type_id: 1,
            dimension_type_name: Some("minecraft:the_nether".to_string()),
            dimension_type_properties: WorldDimensionTypeProperties::default(),
            biome_zoom_seed: 0,
            default_clock_resource_id: None,
            timeline_resource_ids: Vec::new(),
            has_sky_light: false,
            has_ceiling: true,
            cardinal_lighting: WorldCardinalLighting::Nether,
            skybox: WorldSkybox::None,
            ambient_light: 0.1,
            environment_attributes: ClientEnvironmentAttributes::default(),
            last_death_location: None,
            sea_level: 32,
            is_debug: false,
            is_flat: false,
        });
        store
            .available_levels
            .insert("minecraft:the_nether".to_string());
        store.server_enforces_secure_chat = Some(true);
        store.chunks.push(stale_chunk().into());
        store.first_chunk = Some(ChunkPos { x: 1, z: -2 });
        store.apply_add_entity(protocol_add_entity(123));
        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 4,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 6,
        }));
        store.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: 12,
                y: 65,
                z: -5,
            },
            b0: 2,
            b1: 9,
            block_id: 54,
        });
        store.apply_level_event(ProtocolLevelEvent {
            event_type: 1001,
            pos: ProtocolBlockPos { x: 3, y: 4, z: 5 },
            data: 42,
            global: true,
        });
        store.apply_level_event(ProtocolLevelEvent {
            event_type: 1010,
            pos: ProtocolBlockPos { x: 6, y: 7, z: 8 },
            data: 12,
            global: false,
        });
        assert_eq!(store.playing_jukebox_songs().len(), 1);
        store.apply_world_time(ProtocolPlayTime {
            game_time: 123,
            clock_updates: Vec::new(),
        });
        store.apply_game_event(ProtocolGameEvent {
            event_id: 7,
            param: 0.5,
        });
        store.apply_ticking_state(ProtocolTickingState {
            tick_rate: 0.25,
            frozen: true,
        });
        store.apply_ticking_step(ProtocolTickingStep { tick_steps: 7 });
        store.local_player_id = Some(123);
        store.local_player_vehicle_id = Some(456);
        store.set_local_using_item(true);
        store.counters.play_logins_received = 1;

        store.clear_client_level();

        assert_eq!(store.dimension(), WorldDimension::default());
        assert_eq!(store.client_level_generation(), 1);
        assert!(store.level_info().is_none());
        assert!(store.available_levels().is_empty());
        assert_eq!(store.server_enforces_secure_chat(), None);
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.first_chunk(), None);
        assert_eq!(store.entity_count(), 0);
        assert!(store.block_destructions.is_empty());
        assert!(store.block_events.is_empty());
        assert!(store.level_events.is_empty());
        assert!(store.playing_jukebox_songs().is_empty());
        assert!(store.world_time().is_none());
        assert_eq!(store.weather(), WorldWeatherState::default());
        assert_eq!(store.ticking(), WorldTickingState::default());
        assert_eq!(store.gameplay(), &WorldGameplayState::default());
        assert_eq!(store.local_player_id(), None);
        assert_eq!(store.local_player_vehicle_id(), None);
        assert_eq!(store.local_player(), &crate::LocalPlayerState::default());
        assert_eq!(store.local_player_old_pose, None);

        let counters = store.counters();
        assert_eq!(counters.play_logins_received, 1);
        assert_eq!(counters.entities_tracked, 0);
        assert_eq!(counters.block_destructions_tracked, 0);
        assert_eq!(counters.block_events_tracked, 0);
        assert_eq!(counters.level_events_tracked, 0);
    }

    #[test]
    fn world_time_weather_and_ticking_are_canonical_state() {
        let mut store = WorldStore::new();

        store.apply_world_time(ProtocolPlayTime {
            game_time: 123,
            clock_updates: vec![ProtocolClockUpdate {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.25,
                rate: 1.0,
            }],
        });
        store.apply_game_event(ProtocolGameEvent {
            event_id: 7,
            param: 0.5,
        });
        store.apply_game_event(ProtocolGameEvent {
            event_id: 8,
            param: 0.75,
        });
        store.apply_ticking_state(ProtocolTickingState {
            tick_rate: 0.25,
            frozen: true,
        });
        store.apply_ticking_step(ProtocolTickingStep { tick_steps: 7 });

        let time = store.world_time().unwrap();
        assert_eq!(time.game_time, 123);
        assert_eq!(time.day_time, 6000);
        assert_eq!(
            time.clock_updates,
            vec![ClockUpdateState {
                clock_id: 0,
                total_ticks: 6000,
                partial_tick: 0.25,
                rate: 1.0,
            }]
        );

        assert_eq!(
            store.weather(),
            WorldWeatherState {
                raining: true,
                rain_level: 0.5,
                thunder_level: 0.75,
                last_game_event_id: Some(8),
                last_game_event_param: 0.75,
            }
        );
        assert_eq!(
            store.ticking(),
            WorldTickingState {
                tick_rate: 1.0,
                frozen: true,
                frozen_ticks_to_run: 7,
            }
        );

        let counters = store.counters();
        assert_eq!(counters.world_time_packets, 1);
        assert_eq!(counters.game_event_packets, 2);
        assert_eq!(counters.ticking_state_packets, 1);
        assert_eq!(counters.ticking_step_packets, 1);
    }

    #[test]
    fn world_time_selects_dimension_default_clock_by_session_holder_identity() {
        let mut store = WorldStore::new();
        store.record_registry_data(RegistryData {
            registry: "minecraft:world_clock".to_string(),
            raw_payload_len: 0,
            entries: vec![
                registry_stub("minecraft:the_end"),
                registry_stub("minecraft:overworld"),
            ],
        });
        record_custom_dimension_type_registry(
            &mut store,
            dimension_type_nbt_with_default_clock("minecraft:overworld"),
        );
        store.apply_login(&login_packet(4, "example:custom"));

        store.apply_world_time(ProtocolPlayTime {
            game_time: 100,
            clock_updates: vec![
                ProtocolClockUpdate {
                    clock_id: 0,
                    total_ticks: 9_000,
                    partial_tick: 0.0,
                    rate: 1.0,
                },
                ProtocolClockUpdate {
                    clock_id: 1,
                    total_ticks: 6_000,
                    partial_tick: 0.0,
                    rate: 1.0,
                },
            ],
        });

        assert_eq!(
            store
                .level_info()
                .unwrap()
                .default_clock_resource_id
                .as_deref(),
            Some("minecraft:overworld")
        );
        assert_eq!(store.world_time().unwrap().day_time, 6_000);

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: login_packet(2, "minecraft:the_end").common_spawn_info,
            data_to_keep: 3,
        });
        assert_eq!(store.world_time().unwrap().day_time, 9_000);

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: login_packet(1, "minecraft:the_nether").common_spawn_info,
            data_to_keep: 3,
        });
        assert_eq!(store.level_info().unwrap().default_clock_resource_id, None);
        assert_eq!(store.world_time().unwrap().day_time, 0);
    }

    #[test]
    fn dimension_timeline_layers_follow_session_tag_order() {
        let mut store = WorldStore::new();
        store.record_registry_data(RegistryData {
            registry: crate::TIMELINE_REGISTRY.to_string(),
            raw_payload_len: 0,
            entries: vec![
                registry_stub("example:custom"),
                registry_stub("minecraft:day"),
            ],
        });
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: crate::TIMELINE_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:dimension_layers".to_string(),
                    entries: vec![1, 0],
                }],
            }],
        });
        record_custom_dimension_type_registry(
            &mut store,
            dimension_type_nbt_with_timelines("#example:dimension_layers"),
        );

        store.apply_login(&login_packet(4, "example:custom"));

        assert_eq!(
            store.level_info().unwrap().timeline_resource_ids(),
            ["minecraft:day", "example:custom"]
        );
        assert!(store.apply_diagnostics().apply_errors.is_empty());
    }

    #[test]
    fn world_time_merges_partial_clock_updates_like_client_clock_manager() {
        let mut store = WorldStore::new();
        store.apply_world_time(ProtocolPlayTime {
            game_time: 100,
            clock_updates: vec![
                ProtocolClockUpdate {
                    clock_id: 0,
                    total_ticks: 10,
                    partial_tick: 0.0,
                    rate: 1.0,
                },
                ProtocolClockUpdate {
                    clock_id: 1,
                    total_ticks: 20,
                    partial_tick: 0.0,
                    rate: 1.0,
                },
            ],
        });
        store.apply_world_time(ProtocolPlayTime {
            game_time: 102,
            clock_updates: vec![ProtocolClockUpdate {
                clock_id: 1,
                total_ticks: 50,
                partial_tick: 0.5,
                rate: 0.0,
            }],
        });

        let time = store.world_time().unwrap();
        assert_eq!(time.day_time, 12);
        assert_eq!(time.clock_updates.len(), 2);
        assert_eq!(time.clock_updates[0].total_ticks, 12);
        assert_eq!(time.clock_updates[1].total_ticks, 50);
    }

    #[test]
    fn advance_client_time_ticks_game_time_and_clock_instances_like_vanilla() {
        let mut store = WorldStore::new();
        store.apply_world_time(ProtocolPlayTime {
            game_time: 100,
            clock_updates: vec![
                ProtocolClockUpdate {
                    clock_id: 0,
                    total_ticks: 1_485,
                    partial_tick: 0.75,
                    rate: 0.5,
                },
                ProtocolClockUpdate {
                    clock_id: 2,
                    total_ticks: 10,
                    partial_tick: 0.25,
                    rate: 2.25,
                },
            ],
        });

        store.advance_client_time(1);

        let time = store.world_time().unwrap();
        assert_eq!(time.game_time, 101);
        assert_eq!(time.day_time, 1_486);
        assert_eq!(
            time.clock_updates,
            vec![
                ClockUpdateState {
                    clock_id: 0,
                    total_ticks: 1_486,
                    partial_tick: 0.25,
                    rate: 0.5,
                },
                ClockUpdateState {
                    clock_id: 2,
                    total_ticks: 12,
                    partial_tick: 0.5,
                    rate: 2.25,
                },
            ]
        );
    }

    #[test]
    fn advance_client_time_preserves_paused_clock_updates() {
        let mut store = WorldStore::new();
        store.apply_world_time(ProtocolPlayTime {
            game_time: 100,
            clock_updates: vec![ProtocolClockUpdate {
                clock_id: 0,
                total_ticks: 1_485,
                partial_tick: 0.75,
                rate: 0.0,
            }],
        });

        store.advance_client_time(4);

        let time = store.world_time().unwrap();
        assert_eq!(time.game_time, 104);
        assert_eq!(time.day_time, 1_485);
        assert_eq!(
            time.clock_updates,
            vec![ClockUpdateState {
                clock_id: 0,
                total_ticks: 1_485,
                partial_tick: 0.75,
                rate: 0.0,
            }]
        );
    }

    #[test]
    fn sky_flash_time_decrements_on_client_ticks_and_clears_with_level() {
        let mut store = WorldStore::new();

        store.set_sky_flash_time(2);
        assert_eq!(store.sky_flash_time(), 2);

        store.advance_sky_flash_time(1);
        assert_eq!(store.sky_flash_time(), 1);

        store.advance_sky_flash_time(10);
        assert_eq!(store.sky_flash_time(), 0);

        store.set_sky_flash_time(-1);
        assert_eq!(store.sky_flash_time(), 0);

        store.trigger_sky_flash();
        assert_eq!(store.sky_flash_time(), 2);
        store.clear_client_level();
        assert_eq!(store.sky_flash_time(), 0);
    }

    #[test]
    fn frozen_ticking_gates_production_time_advance_but_not_set_time_resync() {
        let mut store = WorldStore::new();
        store.apply_world_time(ProtocolPlayTime {
            game_time: 100,
            clock_updates: vec![ProtocolClockUpdate {
                clock_id: 0,
                total_ticks: 6_000,
                partial_tick: 0.0,
                rate: 1.0,
            }],
        });
        store.apply_ticking_state(ProtocolTickingState {
            tick_rate: 20.0,
            frozen: true,
        });

        // The production entry consumes zero running ticks while frozen, so
        // neither gameTime nor any clock instance advances (vanilla
        // tickRateManager.runsNormally gating ClientLevel.tickTime).
        let (_, running_ticks) = store.advance_client_level_entity_ticks_with_observation(3);
        assert_eq!(running_ticks, 0);
        let time = store.world_time().expect("world time");
        assert_eq!(time.game_time, 100);
        assert_eq!(time.clock_updates[0].total_ticks, 6_000);

        // Sky flash decay uses raw client ticks, outside the freeze gate.
        store.trigger_sky_flash();
        store.advance_sky_flash_time(2);
        assert_eq!(store.sky_flash_time(), 0);

        // Vanilla handleSetTime resyncs unconditionally while frozen.
        store.apply_world_time(ProtocolPlayTime {
            game_time: 220,
            clock_updates: Vec::new(),
        });
        assert_eq!(store.world_time().expect("world time").game_time, 220);
    }

    #[test]
    fn consume_running_render_ticks_respects_frozen_tick_steps() {
        let mut store = WorldStore::new();
        assert_eq!(store.consume_running_render_ticks(3), 3);

        store.apply_ticking_state(ProtocolTickingState {
            tick_rate: 20.0,
            frozen: true,
        });
        assert_eq!(store.consume_running_render_ticks(3), 0);

        store.apply_ticking_step(ProtocolTickingStep { tick_steps: 2 });
        assert_eq!(store.consume_running_render_ticks(3), 2);
        assert_eq!(
            store.ticking(),
            WorldTickingState {
                tick_rate: 20.0,
                frozen: true,
                frozen_ticks_to_run: 0,
            }
        );

        store.apply_ticking_state(ProtocolTickingState {
            tick_rate: 20.0,
            frozen: false,
        });
        store.apply_ticking_step(ProtocolTickingStep { tick_steps: 2 });
        assert_eq!(store.consume_running_render_ticks(3), 3);
        assert_eq!(store.ticking().frozen_ticks_to_run, 0);
    }

    #[test]
    fn game_events_update_local_gameplay_state() {
        let mut store = WorldStore::new();

        store.apply_login(&ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 12345,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });

        assert!(!store.local_player_is_spectator());
        store.set_local_player_pose(crate::LocalPlayerPoseState {
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: -0.5,
                z: 0.2,
            },
            ..crate::LocalPlayerPoseState::default()
        });
        store.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 3.0,
        });
        assert!(store.local_player_is_spectator());
        let pose = store.local_player().pose.expect("pose survives the switch");
        assert_eq!(
            pose.delta_movement.y, 0.0,
            "entering spectator zeroes vertical motion like onGameModeChanged"
        );
        assert_eq!(pose.delta_movement.x, 0.1);
        store.apply_game_event(ProtocolGameEvent {
            event_id: 11,
            param: 1.0,
        });
        store.apply_game_event(ProtocolGameEvent {
            event_id: 12,
            param: 1.0,
        });

        assert_eq!(
            store.gameplay(),
            &WorldGameplayState {
                game_type: 3,
                game_type_name: "spectator".to_string(),
                previous_game_type: Some(0),
                previous_game_type_name: Some("survival".to_string()),
                show_death_screen: false,
                do_limited_crafting: true,
                reduced_debug_info: false,
                hardcore: false,
            }
        );
        assert_eq!(store.counters().game_event_packets, 3);
    }

    #[test]
    fn no_respawn_block_game_event_records_translatable_client_system_message() {
        let mut store = WorldStore::new();

        store.apply_game_event(ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID,
            param: 0.0,
        });

        let message = store.client_chat().messages.last().unwrap();
        assert_eq!(message.kind, crate::ChatMessageKind::ClientSystem);
        assert_eq!(message.content, "block.minecraft.spawn.not_valid");
        assert_eq!(
            message.content_translation_key.as_deref(),
            Some("block.minecraft.spawn.not_valid")
        );
        assert!(message.styled_content.is_empty());
        assert!(message.content_component.is_some());
        let narration = store
            .client_chat()
            .pending_narrations
            .last()
            .expect("system-chat narration");
        assert_eq!(narration.kind, ChatNarrationKind::SystemChat);
        assert!(narration.content_component.is_some());
    }

    #[test]
    fn win_and_demo_intro_game_events_replace_supported_screens_by_generation() {
        let mut store = WorldStore::new();
        assert!(store.open_advancements_screen());

        store.apply_game_event(ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
            param: 1.0,
        });
        let win = store.game_event_screen().expect("win screen");
        assert_eq!(win, ClientGameEventScreenState::Win { generation: 1 });
        assert!(!store.advancements_screen_is_open());

        store.apply_game_event(ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
            param: 0.0,
        });
        let demo = store.game_event_screen().expect("demo help");
        assert_eq!(demo, ClientGameEventScreenState::DemoHelp { generation: 2 });
        assert!(!store.close_game_event_screen(win.generation()));
        assert!(store.close_game_event_screen(demo.generation()));
        assert_eq!(store.game_event_screen(), None);
    }

    #[test]
    fn demo_hint_game_events_record_rich_translations_and_system_narration() {
        let mut store = WorldStore::new();
        let hints = [
            (101.0, "demo.help.movement", &["W", "A", "S", "D"][..]),
            (102.0, "demo.help.jump", &["Space"][..]),
            (103.0, "demo.help.inventory", &["E"][..]),
            (104.0, "demo.day.6", &["F2"][..]),
        ];

        for (param, key, arguments) in hints {
            store.apply_game_event(ProtocolGameEvent {
                event_id: VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
                param,
            });
            let message = store.client_chat().messages.last().unwrap();
            assert_eq!(message.content_translation_key.as_deref(), Some(key));
            let component = message.content_component.as_ref().unwrap();
            let bbb_protocol::NbtSummaryValue::Compound(entries) = component else {
                panic!("translated hint must be a component compound");
            };
            let with = entries
                .iter()
                .find(|entry| entry.name == "with")
                .map(|entry| &entry.value);
            assert_eq!(
                with,
                Some(&bbb_protocol::NbtSummaryValue::List(
                    arguments
                        .iter()
                        .map(|argument| {
                            bbb_protocol::NbtSummaryValue::String((*argument).to_string())
                        })
                        .collect()
                ))
            );
            let narration = store.client_chat().pending_narrations.last().unwrap();
            assert_eq!(narration.kind, ChatNarrationKind::System);
            assert_eq!(narration.content_component.as_ref(), Some(component));
        }

        let message_count = store.client_chat().messages.len();
        store.apply_game_event(ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
            param: 100.0,
        });
        assert_eq!(store.client_chat().messages.len(), message_count);
    }

    #[test]
    fn clearing_client_level_closes_game_event_screen() {
        let mut store = WorldStore::new();
        store.apply_game_event(ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
            param: 0.0,
        });
        assert!(store.game_event_screen().is_some());

        store.clear_client_level();

        assert_eq!(store.game_event_screen(), None);
    }

    #[test]
    fn respawn_and_level_clear_close_death_screen() {
        let mut store = WorldStore::new();
        store.apply_login(&login_packet(0, "minecraft:overworld"));
        store.apply_player_combat_kill(bbb_protocol::packets::PlayerCombatKill {
            player_id: 42,
            message: bbb_protocol::RichTextComponent::plain("First death"),
        });
        assert!(store.death_screen().is_some());

        store.apply_respawn(&respawn_packet(0));
        assert_eq!(store.death_screen(), None);

        store.apply_player_combat_kill(bbb_protocol::packets::PlayerCombatKill {
            player_id: 42,
            message: bbb_protocol::RichTextComponent::plain("Second death"),
        });
        assert!(store.death_screen().is_some());
        store.clear_client_level();
        assert_eq!(store.death_screen(), None);
    }

    #[test]
    fn level_chunks_load_start_advances_active_level_load_tracker_once() {
        let mut store = WorldStore::new();
        let event = ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
            param: 0.0,
        };
        store.apply_game_event(event);
        assert_eq!(store.level_load(), &WorldLevelLoadState::default());

        store.apply_login(&login_packet(0, "minecraft:overworld"));
        assert_eq!(
            store.level_load().phase,
            WorldLevelLoadPhase::WaitingForServer
        );
        assert_eq!(store.level_load().starts, 1);

        store.apply_game_event(event);
        store.apply_game_event(event);

        assert_eq!(
            store.level_load().phase,
            WorldLevelLoadPhase::WaitingForPlayerChunk
        );
        assert_eq!(store.level_load().loading_packets_received, 2);

        assert!(store.complete_client_level_load(false));
        assert_eq!(store.level_load().phase, WorldLevelLoadPhase::ClientLoaded);
        assert_eq!(store.level_load().loads_completed, 1);
        assert_eq!(store.level_load().timeouts, 0);
        assert!(!store.complete_client_level_load(true));
        store.apply_game_event(event);
        assert_eq!(store.level_load().loading_packets_received, 2);

        store.apply_respawn(&respawn_packet(0));
        assert_eq!(
            store.level_load().phase,
            WorldLevelLoadPhase::WaitingForServer
        );
        assert_eq!(store.level_load().starts, 2);
        assert_eq!(store.level_load().loading_packets_received, 0);
        assert!(!store.complete_client_level_load(true));

        store.apply_game_event(event);
        assert!(store.complete_client_level_load(true));
        assert_eq!(store.level_load().loads_completed, 2);
        assert_eq!(store.level_load().timeouts, 1);
    }

    #[test]
    fn the_level_loading_screen_window_and_reason_follow_the_dimension_change() {
        let mut store = WorldStore::new();
        assert!(!store.level_load().screen_is_up());

        store.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(store.level_load().screen_is_up());
        assert_eq!(store.level_load().reason, WorldLevelLoadReason::Other);

        let event = ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
            param: 0.0,
        };
        store.apply_game_event(event);
        assert!(store.level_load().screen_is_up());
        assert!(store.complete_client_level_load(false));
        assert!(!store.level_load().screen_is_up());

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension: "minecraft:the_nether".to_string(),
                dimension_type_id: 1,
                ..respawn_packet(0).common_spawn_info
            },
            data_to_keep: 0,
        });
        assert!(store.level_load().screen_is_up());
        assert_eq!(
            store.level_load().reason,
            WorldLevelLoadReason::NetherPortal
        );

        store.apply_game_event(event);
        assert!(store.complete_client_level_load(false));
        store.apply_respawn(&respawn_packet(0));
        assert_eq!(
            store.level_load().reason,
            WorldLevelLoadReason::NetherPortal
        );

        store.apply_game_event(event);
        assert!(store.complete_client_level_load(false));
        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension: "minecraft:the_end".to_string(),
                dimension_type_id: 2,
                ..respawn_packet(0).common_spawn_info
            },
            data_to_keep: 0,
        });
        assert_eq!(store.level_load().reason, WorldLevelLoadReason::EndPortal);
    }

    #[test]
    fn a_death_respawn_keeps_the_other_level_loading_reason() {
        assert_eq!(
            level_loading_reason(true, "minecraft:the_nether", Some("minecraft:overworld")),
            WorldLevelLoadReason::Other
        );
        assert_eq!(
            level_loading_reason(false, "minecraft:overworld", Some("minecraft:the_nether")),
            WorldLevelLoadReason::NetherPortal
        );
        assert_eq!(
            level_loading_reason(false, "minecraft:overworld", Some("minecraft:the_end")),
            WorldLevelLoadReason::EndPortal
        );
        assert_eq!(
            level_loading_reason(false, "minecraft:overworld", None),
            WorldLevelLoadReason::Other
        );
    }

    fn respawn_state_with_local_player_data() -> WorldStore {
        let mut store = WorldStore::new();
        store.local_player_id = Some(123);
        store.apply_add_entity(local_player_entity(123));
        store.apply_add_entity(protocol_add_entity(456));
        store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.2,
            walking_speed: 0.3,
        });
        assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 456 }));
        store.apply_player_health(ProtocolPlayerHealth {
            health: 4.0,
            food: 7,
            saturation: 0.5,
        });
        store.apply_player_experience(ProtocolPlayerExperience {
            progress: 0.25,
            level: 3,
            total: 40,
        });
        assert!(store.set_local_selected_hotbar_slot(8));
        store.local_player.default_spawn = Some(crate::DefaultSpawnState {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos { x: 8, y: 64, z: 8 },
            yaw: 0.0,
            pitch: 0.0,
        });
        store.local_player.simulation_distance = Some(12);
        store.local_player.sprinting = true;
        store
            .entities
            .with_equipment_mut(123, |equipment| {
                equipment.equipment.push(ProtocolEquipmentSlotUpdate {
                    slot: EquipmentSlot::Head,
                    item: ProtocolItemStackSummary {
                        item_id: Some(1),
                        count: 1,
                        component_patch: Default::default(),
                    },
                });
            })
            .unwrap();
        store
            .entities
            .with_leash_mut(123, |leash| leash.holder_id = Some(456))
            .unwrap();
        store
            .entities
            .with_supporting_block_mut(123, |support| {
                support.main = Some(BlockPos { x: 4, y: 63, z: -2 });
                support.on_ground_no_blocks = true;
            })
            .unwrap();
        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 123,
            event_id: 28,
        }));
        store.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 10.0,
                y: 65.0,
                z: -4.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: -0.2,
                z: 0.3,
            },
            on_ground: true,
            horizontal_collision: true,
            minor_horizontal_collision: false,
            fall_distance: 8.0,
            sneaking: true,
            swimming: true,
            no_jump_delay_ticks: 0,
            y_rot: 90.0,
            x_rot: 20.0,
            last_teleport_id: 77,
        });
        store.set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
        store.set_local_using_item(true);
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 123,
            values: vec![entity_byte_data(0, 0x02)],
        }));
        assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: 0.1,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:test_speed".to_string(),
                    amount: 0.25,
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                }],
            }],
        }));
        assert!(store.apply_update_mob_effect(ProtocolUpdateMobEffect {
            entity_id: 123,
            effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
            amplifier: 1,
            duration_ticks: 200,
            flags: MobEffectFlags::default(),
        }));
        assert_eq!(store.counters().active_mob_effects_tracked, 1);
        store
    }

    fn respawn_packet(data_to_keep: i8) -> ProtocolRespawn {
        ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 12345,
                game_type: 0,
                previous_game_type: 0,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep,
        }
    }

    fn entity_byte_data(data_id: u8, value: i8) -> ProtocolEntityDataValue {
        ProtocolEntityDataValue {
            data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(value),
        }
    }

    fn local_player_entity(id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            ..protocol_add_entity(id)
        }
    }

    fn stale_chunk() -> ChunkColumn {
        ChunkColumn {
            pos: ChunkPos { x: 1, z: -2 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: Vec::new(),
            light: LightData::default(),
        }
    }

    fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -10.0,
            y_rot: 20.0,
            y_head_rot: 30.0,
            data: 99,
        }
    }

    #[test]
    fn login_resets_map_store_while_dimension_respawn_carries_it_over() {
        let mut store = WorldStore::new();
        store.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(store.apply_map_item_data(ProtocolMapItemData {
            map_id: 9,
            scale: 1,
            locked: false,
            decorations: None,
            color_patch: None,
        }));
        assert_eq!(store.counters().maps_tracked, 1);

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 1,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            data_to_keep: 0,
        });
        assert!(
            store.map_item(9).is_some(),
            "dimension respawn carries the map table over"
        );

        store.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(
            store.map_item(9).is_none(),
            "login starts from an empty map table"
        );
        assert!(store.map_items().is_empty());
        assert!(store.last_map_color_patch().is_none());
        assert_eq!(store.counters().maps_tracked, 0);
        assert_eq!(store.counters().map_decorations_tracked, 0);
    }

    #[test]
    fn spawn_info_uses_builtin_cardinal_lighting_when_dimension_type_registry_is_missing() {
        let mut nether = WorldStore::new();
        nether.apply_login(&login_packet(1, "minecraft:custom_nether_key"));
        assert!(!nether.level_info().unwrap().has_sky_light());
        assert_eq!(nether.debug_sky_light_section_status([0, 0, 0]), "n/a");
        assert_eq!(
            nether.debug_sky_light_section_occupancy([0, 0, 0]),
            [false, false]
        );
        assert_eq!(
            nether.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Nether
        );

        let mut remapped_nether = WorldStore::new();
        remapped_nether.apply_login(&login_packet(7, "minecraft:the_nether"));
        assert!(!remapped_nether.level_info().unwrap().has_sky_light());
        assert_eq!(
            remapped_nether.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Nether
        );

        let mut end = WorldStore::new();
        end.apply_login(&login_packet(2, "minecraft:the_end"));
        assert!(!end.level_info().unwrap().has_sky_light());
        assert_eq!(
            end.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Default
        );

        let mut overworld = WorldStore::new();
        overworld.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(overworld.level_info().unwrap().has_sky_light());
    }

    #[test]
    fn dimension_type_registry_cardinal_light_overrides_builtin_fallback() {
        let mut custom_nether = WorldStore::new();
        record_custom_dimension_type_registry(
            &mut custom_nether,
            dimension_type_nbt(
                Some("nether"),
                Some(false),
                Some(true),
                None,
                Some("none"),
                Some(0.1),
            ),
        );
        custom_nether.apply_login(&login_packet(4, "example:custom"));
        assert!(!custom_nether.level_info().unwrap().has_sky_light());
        assert_eq!(
            custom_nether.debug_sky_light_section_status([0, 0, 0]),
            "n/a"
        );
        assert_eq!(
            custom_nether.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Nether
        );

        let mut omitted_field = WorldStore::new();
        record_custom_dimension_type_registry(
            &mut omitted_field,
            dimension_type_nbt(None, None, None, None, None, None),
        );
        omitted_field.apply_login(&login_packet(4, "minecraft:the_nether"));
        assert_eq!(
            omitted_field.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Default
        );
    }

    #[test]
    fn dimension_type_registry_projects_remaining_network_scalars_to_level_state() {
        let mut store = WorldStore::new();
        record_custom_dimension_type_registry(
            &mut store,
            dimension_type_nbt_with_network_scalars(),
        );

        store.apply_login(&login_packet(4, "example:custom"));

        assert_eq!(
            store.level_info().unwrap().dimension_type_properties,
            WorldDimensionTypeProperties {
                has_fixed_time: true,
                has_ender_dragon_fight: true,
                coordinate_scale: 8.0,
                logical_height: 256,
                infiniburn_tag: Some("example:forever".to_string()),
                monster_spawn_light_level: Some(WorldIntProvider::Constant(7)),
                monster_spawn_block_light_limit: Some(4),
            }
        );
    }

    #[test]
    fn dimension_type_registry_shape_overrides_builtin_and_rejects_invalid_shape() {
        let mut custom = WorldStore::new();
        record_custom_dimension_type_registry(
            &mut custom,
            dimension_type_nbt(
                None,
                Some(true),
                Some(true),
                Some((-32, 512, 384)),
                Some("end"),
                Some(0.35),
            ),
        );
        custom.apply_login(&login_packet(4, "example:custom"));
        assert_eq!(
            custom.level_info().unwrap().dimension_type_name.as_deref(),
            Some("example:custom")
        );
        assert_eq!(
            custom.dimension(),
            WorldDimension {
                min_y: -32,
                height: 512,
            }
        );
        assert_eq!(custom.level_info().unwrap().skybox(), WorldSkybox::End);
        assert_eq!(custom.level_info().unwrap().ambient_light(), 0.35);
        assert!(custom.level_info().unwrap().has_ceiling());
        assert!(!custom.level_info().unwrap().can_have_weather());

        for invalid_shape in [
            (-31, 512, 384),
            (-32, 512, 513),
            (0, 15, 15),
            (0, 4080, 384),
            (2016, 32, 32),
            (-2048, 16, 16),
        ] {
            let mut invalid = WorldStore::new();
            record_custom_dimension_type_registry(
                &mut invalid,
                dimension_type_nbt(None, Some(true), None, Some(invalid_shape), None, None),
            );
            invalid.apply_login(&login_packet(4, "example:custom"));
            assert_eq!(invalid.dimension(), WorldDimension::default());
        }
    }

    #[test]
    fn login_and_respawn_resolve_dimension_type_holders_in_server_order() {
        let mut store = WorldStore::new();
        store.record_registry_data(RegistryData {
            registry: "minecraft:dimension_type".to_string(),
            raw_payload_len: 0,
            entries: vec![
                RegistryDataEntry {
                    id: "example:compact".to_string(),
                    raw_data: Some(dimension_type_nbt(
                        Some("nether"),
                        Some(false),
                        Some(true),
                        Some((0, 128, 96)),
                        Some("none"),
                        Some(0.2),
                    )),
                },
                RegistryDataEntry {
                    id: "example:tall".to_string(),
                    raw_data: Some(dimension_type_nbt(
                        Some("default"),
                        Some(true),
                        Some(false),
                        Some((-64, 384, 320)),
                        Some("end"),
                        Some(0.4),
                    )),
                },
            ],
        });

        store.apply_login(&login_packet(1, "example:first_level"));
        let login_level = store.level_info().unwrap();
        assert_eq!(login_level.dimension_type_id, 1);
        assert_eq!(
            login_level.dimension_type_name.as_deref(),
            Some("example:tall")
        );
        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: -64,
                height: 384,
            }
        );
        assert!(login_level.has_sky_light());
        assert_eq!(login_level.skybox(), WorldSkybox::End);
        assert_eq!(login_level.ambient_light(), 0.4);

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 0,
                dimension: "example:second_level".to_string(),
                seed: 67890,
                game_type: 0,
                previous_game_type: 0,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 3,
        });
        let respawn_level = store.level_info().unwrap();
        assert_eq!(respawn_level.dimension_type_id, 0);
        assert_eq!(
            respawn_level.dimension_type_name.as_deref(),
            Some("example:compact")
        );
        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: 0,
                height: 128,
            }
        );
        assert!(!respawn_level.has_sky_light());
        assert!(respawn_level.has_ceiling());
        assert_eq!(
            respawn_level.cardinal_lighting(),
            WorldCardinalLighting::Nether
        );
        assert_eq!(respawn_level.skybox(), WorldSkybox::None);
        assert_eq!(respawn_level.ambient_light(), 0.2);
    }

    #[test]
    fn dimension_type_registry_projects_environment_attributes_and_diagnoses_invalid_modifiers() {
        let mut custom = WorldStore::new();
        record_custom_dimension_type_registry(
            &mut custom,
            dimension_type_nbt_with_environment_attributes(false),
        );
        custom.apply_login(&login_packet(4, "example:custom"));

        let attributes = custom.level_info().unwrap().environment_attributes();
        assert_eq!(attributes.fog_color, Some([0x12, 0x34, 0x56]));
        assert_eq!(attributes.fog_start_distance, Some(12.0));
        assert_eq!(attributes.fog_end_distance, Some(144.0));
        assert_eq!(attributes.water_fog_start_distance, Some(-4.0));
        assert_eq!(attributes.water_fog_end_distance, Some(48.0));
        assert_eq!(attributes.block_light_tint, Some([0x20, 0x40, 0x60]));
        assert_eq!(attributes.sky_light_factor, Some(0.25));
        assert_eq!(attributes.ambient_light_color, Some([0x30, 0x50, 0x70]));
        assert!(custom.apply_diagnostics().apply_errors.is_empty());

        let mut unsupported = WorldStore::new();
        record_custom_dimension_type_registry(
            &mut unsupported,
            dimension_type_nbt_with_environment_attributes(true),
        );
        unsupported.apply_login(&login_packet(4, "example:custom"));
        assert_eq!(
            unsupported.level_info().unwrap().environment_attributes(),
            ClientEnvironmentAttributes::default()
        );
        assert_eq!(
            unsupported
                .apply_diagnostics()
                .last_apply_error
                .as_ref()
                .map(|error| (error.source.as_str(), error.message.as_str())),
            Some((
                "dimension_type_environment_attributes",
                "minecraft:visual/fog_color: unknown attribute modifier",
            ))
        );
    }

    #[test]
    fn apply_login_records_the_hardcore_flag() {
        let mut world = WorldStore::new();
        assert!(!world.is_hardcore());

        let mut login = login_packet(0, "minecraft:overworld");
        login.hardcore = true;
        world.apply_login(&login);
        assert!(world.is_hardcore());
        assert!(world.gameplay().hardcore);

        // A later non-hardcore login clears it.
        world.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(!world.is_hardcore());
    }

    #[test]
    fn connection_services_survive_respawn_and_reset_only_with_the_listener() {
        let mut world = WorldStore::new();
        world.apply_login(&login_packet(0, "minecraft:overworld"));
        assert!(
            world.apply_set_objective(bbb_protocol::packets::SetObjective {
                objective_name: "session".to_string(),
                method: bbb_protocol::packets::SetObjectiveMethod::Add.into(),
                parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                    display_name: bbb_protocol::RichTextComponent::plain("Session"),
                    render_type: bbb_protocol::objective_render_types::ObjectiveRenderType::Integer,
                    number_format: None,
                }),
            })
        );
        world.apply_update_recipes(bbb_protocol::packets::UpdateRecipes {
            property_sets: vec![bbb_protocol::packets::RecipePropertySetSummary {
                key: "minecraft:smithing_base".to_string(),
                item_ids: vec![7],
            }],
            stonecutter_recipes: Vec::new(),
        });
        world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
            features: vec!["minecraft:vanilla".to_string()],
        });
        world.set_brewing_potion_item_ids(BTreeSet::from([44]));
        world.set_furnace_fuel_item_ids(BTreeSet::from([43]));

        // Vanilla keeps the same ClientPacketListener across any respawn, so
        // every connection-scoped service survives a dimension change.
        world.apply_respawn(&respawn_packet(0));
        assert_eq!(world.scoreboard.objectives.len(), 1);
        assert_eq!(world.recipes().property_sets.len(), 1);
        assert!(world.is_feature_enabled("minecraft:vanilla"));
        assert_eq!(*world.items.brewing_potion_item_ids, BTreeSet::from([44]));
        assert_eq!(*world.furnace_fuel_item_ids(), BTreeSet::from([43]));

        // A fresh listener (login/configuration re-entry) owns fresh final
        // Scoreboard and pending-recipe state, while CommonListenerCookie
        // carries enabled features and the registry-derived brewing/fuel
        // values persist until the next registry rebuild overwrites them.
        world.clear_client_level();
        assert!(world.scoreboard.objectives.is_empty());
        assert!(world.recipes().property_sets.is_empty());
        assert!(world.is_feature_enabled("minecraft:vanilla"));
        assert_eq!(*world.items.brewing_potion_item_ids, BTreeSet::from([44]));
        assert_eq!(*world.furnace_fuel_item_ids(), BTreeSet::from([43]));
    }

    #[test]
    fn client_world_owns_no_scheduled_tick_simulation_or_socket() {
        // Vanilla `ClientLevel.getBlockTicks`/`getFluidTicks` return
        // `BlackholeTickAccess`: the client never simulates scheduled ticks,
        // every block/fluid change arrives as a server packet. The world
        // crate must not grow a scheduled-tick queue, and
        // `sendPacketToServer` delegation means it must not own a socket.
        let production = |source: &'static str| {
            source
                .split("#[cfg(test)]")
                .next()
                .expect("production source")
                .to_string()
        };
        let source = production(include_str!("store.rs"))
            + &production(include_str!("level.rs"))
            + &production(include_str!("chunks/store.rs"));
        assert_eq!(source.matches("scheduled_tick").count(), 0);
        assert_eq!(source.matches("TcpStream").count(), 0);
        assert_eq!(
            source.matches("std::net").count(),
            0,
            "outbound packets leave only through the net crate's send path"
        );
    }

    #[test]
    fn horizon_height_and_void_darkness_range_follow_the_flat_level_flag() {
        let mut world = WorldStore::new();
        // Without a level the non-flat defaults apply.
        assert_eq!(world.client_horizon_height(), 63.0);
        assert_eq!(world.client_void_darkness_onset_range(), 32.0);

        world.apply_login(&login_packet(0, "minecraft:overworld"));
        assert_eq!(world.client_horizon_height(), 63.0);
        assert_eq!(world.client_void_darkness_onset_range(), 32.0);

        let mut flat_login = login_packet(0, "minecraft:overworld");
        flat_login.common_spawn_info.is_flat = true;
        world.apply_login(&flat_login);
        assert_eq!(
            world.client_horizon_height(),
            f64::from(world.dimension().min_y)
        );
        assert_eq!(world.client_void_darkness_onset_range(), 1.0);
    }

    #[test]
    fn dark_disc_gate_compares_the_player_eye_against_the_horizon_height() {
        let mut world = WorldStore::new();
        world.apply_login(&login_packet(0, "minecraft:overworld"));
        // No player pose keeps the disc hidden.
        assert!(!world.local_player_eye_below_horizon());

        let mut pose = LocalPlayerPoseState::default();
        pose.position.y = 61.0;
        world.local_player.pose = Some(pose);
        // Standing eye height 1.62 puts the eye at 62.62, under the 63.0
        // overworld horizon.
        assert!(world.local_player_eye_below_horizon());

        pose.position.y = 61.5;
        world.local_player.pose = Some(pose);
        assert!(!world.local_player_eye_below_horizon());
    }

    fn login_packet(dimension_type_id: i32, dimension: &str) -> ProtocolPlayLogin {
        ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec![dimension.to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id,
                dimension: dimension.to_string(),
                seed: 12345,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        }
    }

    fn record_custom_dimension_type_registry(store: &mut WorldStore, raw_data: Vec<u8>) {
        store.record_registry_data(RegistryData {
            registry: "minecraft:dimension_type".to_string(),
            raw_payload_len: raw_data.len(),
            entries: vec![
                registry_stub("minecraft:overworld"),
                registry_stub("minecraft:the_nether"),
                registry_stub("minecraft:the_end"),
                registry_stub("minecraft:overworld_caves"),
                RegistryDataEntry {
                    id: "example:custom".to_string(),
                    raw_data: Some(raw_data),
                },
            ],
        });
    }

    fn registry_stub(id: &str) -> RegistryDataEntry {
        RegistryDataEntry {
            id: id.to_string(),
            raw_data: None,
        }
    }

    fn dimension_type_nbt(
        cardinal_light: Option<&str>,
        has_sky_light: Option<bool>,
        has_ceiling: Option<bool>,
        shape: Option<(i32, i32, i32)>,
        skybox: Option<&str>,
        ambient_light: Option<f32>,
    ) -> Vec<u8> {
        let mut payload = vec![10];
        if let Some((min_y, height, logical_height)) = shape {
            for (name, value) in [
                ("min_y", min_y),
                ("height", height),
                ("logical_height", logical_height),
            ] {
                payload.push(3);
                write_nbt_string(&mut payload, name);
                payload.extend_from_slice(&value.to_be_bytes());
            }
        }
        if let Some(has_sky_light) = has_sky_light {
            payload.push(1);
            write_nbt_string(&mut payload, "has_skylight");
            payload.push(u8::from(has_sky_light));
        }
        if let Some(has_ceiling) = has_ceiling {
            payload.push(1);
            write_nbt_string(&mut payload, "has_ceiling");
            payload.push(u8::from(has_ceiling));
        }
        if let Some(cardinal_light) = cardinal_light {
            payload.push(8);
            write_nbt_string(&mut payload, "cardinal_light");
            write_nbt_string(&mut payload, cardinal_light);
        }
        if let Some(skybox) = skybox {
            payload.push(8);
            write_nbt_string(&mut payload, "skybox");
            write_nbt_string(&mut payload, skybox);
        }
        if let Some(ambient_light) = ambient_light {
            payload.push(5);
            write_nbt_string(&mut payload, "ambient_light");
            payload.extend_from_slice(&ambient_light.to_bits().to_be_bytes());
        }
        payload.push(0);
        payload
    }

    fn dimension_type_nbt_with_environment_attributes(modified_fog_color: bool) -> Vec<u8> {
        let mut payload = dimension_type_nbt(
            None,
            Some(true),
            Some(false),
            Some((-64, 384, 384)),
            Some("overworld"),
            Some(0.0),
        );
        assert_eq!(payload.pop(), Some(0));
        payload.push(10);
        write_nbt_string(&mut payload, "attributes");
        if modified_fog_color {
            payload.push(10);
            write_nbt_string(&mut payload, "minecraft:visual/fog_color");
            payload.push(8);
            write_nbt_string(&mut payload, "modifier");
            write_nbt_string(&mut payload, "minecraft:override");
            payload.push(8);
            write_nbt_string(&mut payload, "argument");
            write_nbt_string(&mut payload, "#123456");
            payload.push(0);
        } else {
            write_nbt_named_string(&mut payload, "minecraft:visual/fog_color", "#123456");
        }
        write_nbt_named_string(&mut payload, "minecraft:visual/sky_color", "#345678");
        write_nbt_named_string(&mut payload, "minecraft:visual/water_fog_color", "#56789a");
        write_nbt_named_float(&mut payload, "minecraft:visual/fog_start_distance", 12.0);
        write_nbt_named_float(&mut payload, "minecraft:visual/fog_end_distance", 144.0);
        write_nbt_named_float(&mut payload, "minecraft:visual/sky_fog_end_distance", 96.0);
        write_nbt_named_float(
            &mut payload,
            "minecraft:visual/cloud_fog_end_distance",
            192.0,
        );
        write_nbt_named_float(
            &mut payload,
            "minecraft:visual/water_fog_start_distance",
            -4.0,
        );
        write_nbt_named_float(
            &mut payload,
            "minecraft:visual/water_fog_end_distance",
            48.0,
        );
        write_nbt_named_string(&mut payload, "minecraft:visual/block_light_tint", "#204060");
        write_nbt_named_string(&mut payload, "minecraft:visual/sky_light_color", "#406080");
        write_nbt_named_float(&mut payload, "minecraft:visual/sky_light_factor", 0.25);
        write_nbt_named_string(
            &mut payload,
            "minecraft:visual/night_vision_color",
            "#507090",
        );
        write_nbt_named_string(
            &mut payload,
            "minecraft:visual/ambient_light_color",
            "#305070",
        );
        payload.push(0);
        payload.push(0);
        payload
    }

    fn dimension_type_nbt_with_network_scalars() -> Vec<u8> {
        let mut payload = dimension_type_nbt(
            None,
            Some(false),
            Some(true),
            Some((0, 256, 256)),
            None,
            Some(0.1),
        );
        assert_eq!(payload.pop(), Some(0));
        payload.push(1);
        write_nbt_string(&mut payload, "has_fixed_time");
        payload.push(1);
        payload.push(1);
        write_nbt_string(&mut payload, "has_ender_dragon_fight");
        payload.push(1);
        payload.push(6);
        write_nbt_string(&mut payload, "coordinate_scale");
        payload.extend_from_slice(&8.0_f64.to_bits().to_be_bytes());
        write_nbt_named_string(&mut payload, "infiniburn", "#example:forever");
        payload.push(3);
        write_nbt_string(&mut payload, "monster_spawn_light_level");
        payload.extend_from_slice(&7_i32.to_be_bytes());
        payload.push(3);
        write_nbt_string(&mut payload, "monster_spawn_block_light_limit");
        payload.extend_from_slice(&4_i32.to_be_bytes());
        payload.push(0);
        payload
    }

    fn dimension_type_nbt_with_default_clock(default_clock: &str) -> Vec<u8> {
        let mut payload = dimension_type_nbt(None, None, None, None, None, None);
        assert_eq!(payload.pop(), Some(0));
        payload.push(8);
        write_nbt_string(&mut payload, "default_clock");
        write_nbt_string(&mut payload, default_clock);
        payload.push(0);
        payload
    }

    fn dimension_type_nbt_with_timelines(timelines: &str) -> Vec<u8> {
        let mut payload = dimension_type_nbt(None, None, None, None, None, None);
        assert_eq!(payload.pop(), Some(0));
        write_nbt_named_string(&mut payload, "timelines", timelines);
        payload.push(0);
        payload
    }

    fn write_nbt_named_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }

    fn write_nbt_named_float(out: &mut Vec<u8>, name: &str, value: f32) {
        out.push(5);
        write_nbt_string(out, name);
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
