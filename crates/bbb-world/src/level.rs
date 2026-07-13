use std::collections::BTreeSet;

use bbb_protocol::game_event_types::{
    VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID, VANILLA_CLIENT_GAME_EVENT_IMMEDIATE_RESPAWN_ID,
    VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
    VANILLA_CLIENT_GAME_EVENT_LIMITED_CRAFTING_ID,
    VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID,
    VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID, VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID,
    VANILLA_CLIENT_GAME_EVENT_STOP_RAINING_ID, VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
};
use bbb_protocol::game_types::{
    GameType, VANILLA_GAME_TYPE_NOT_SET_ID, VANILLA_GAME_TYPE_SPECTATOR_ID,
    VANILLA_GAME_TYPE_SURVIVAL_ID,
};
use bbb_protocol::packets::{
    ClockUpdate as ProtocolClockUpdate, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
    GameEvent as ProtocolGameEvent, PlayLogin as ProtocolPlayLogin, PlayTime as ProtocolPlayTime,
    Respawn as ProtocolRespawn, TickingState as ProtocolTickingState,
    TickingStep as ProtocolTickingStep,
};
use serde::{Deserialize, Serialize};

use crate::{
    chunks::{decode_nbt_root, find_nbt_entry, NbtValue},
    registries::{
        decode_client_environment_attributes, ClientEnvironmentAttributes,
        EnvironmentAttributeProjectionError,
    },
    BlockPos, WorldStore,
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

    pub fn contains_y(self, y: i32) -> bool {
        y >= self.min_y && y < self.min_y + self.height
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldLevelInfo {
    pub dimension: String,
    pub dimension_type_id: i32,
    pub dimension_type_name: Option<String>,
    /// Vanilla passes the CommonPlayerSpawnInfo seed only to ClientLevel's
    /// BiomeManager. It is not client authority over server world generation.
    #[serde(default)]
    pub biome_zoom_seed: i64,
    #[serde(default)]
    pub default_clock_resource_id: Option<String>,
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
        self.environment_attributes
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
    /// `ClientboundLoginPacket.reducedDebugInfo`.
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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLevelLoadState {
    pub phase: WorldLevelLoadPhase,
    pub starts: usize,
    pub loading_packets_received: usize,
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
            registries: crate::RegistrySet::vanilla_26_1(),
            ..Self::default()
        }
    }

    pub fn apply_login(&mut self, login: &ProtocolPlayLogin) {
        self.counters.play_logins_received += 1;
        self.advance_client_level_generation();
        self.available_levels = login.levels.iter().cloned().collect();
        self.server_enforces_secure_chat = Some(login.enforces_secure_chat);
        // Vanilla constructs the new ClientChunkCache and ClientLevel from these
        // login fields before later SetChunkCacheRadius/SetSimulationDistance
        // packets may replace them.
        self.chunk_view = crate::ChunkViewState {
            center: None,
            radius: Some(login.chunk_radius),
        };
        self.local_player.simulation_distance = Some(login.simulation_distance);
        self.removed_player_vehicle_id = None;
        if let Some(local_player_id) = self.local_player_id {
            self.clear_local_player_mount(local_player_id);
        } else {
            self.local_player_vehicle_id = None;
        }
        self.local_player_id = Some(login.player_id);
        self.reset_local_player_permission_level();
        self.gameplay.show_death_screen = login.show_death_screen;
        self.gameplay.do_limited_crafting = login.do_limited_crafting;
        self.gameplay.reduced_debug_info = login.reduced_debug_info;
        self.gameplay.hardcore = login.hardcore;
        self.apply_spawn_info(&login.common_spawn_info);
        self.start_client_level_load();
    }

    pub fn apply_respawn(&mut self, respawn: &ProtocolRespawn) -> Option<i32> {
        self.counters.respawns_received += 1;
        self.advance_client_level_generation();
        let closed_server_container_id = self.close_container_for_respawn();
        self.reset_local_player_for_respawn(respawn.data_to_keep);
        self.apply_spawn_info(&respawn.common_spawn_info);
        self.start_client_level_load();
        closed_server_container_id
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
            VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID => {
                if self.level_load.phase != WorldLevelLoadPhase::Idle {
                    self.level_load.loading_packets_received =
                        self.level_load.loading_packets_received.saturating_add(1);
                    if self.level_load.phase == WorldLevelLoadPhase::WaitingForServer {
                        self.level_load.phase = WorldLevelLoadPhase::WaitingForPlayerChunk;
                    }
                }
            }
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

        if self.local_player.portal_cooldown > 0 {
            let ticks = i32::try_from(ticks).unwrap_or(i32::MAX);
            self.local_player.portal_cooldown = self
                .local_player
                .portal_cooldown
                .saturating_sub(ticks)
                .max(0);
        }
        self.advance_recipe_book_tab_animation(ticks);

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
        self.local_player_id = None;
        self.local_player_vehicle_id = None;
        self.removed_player_vehicle_id = None;
        self.last_projectile_power = None;
        self.clear_level_bound_state();
    }

    pub fn level_load(&self) -> &WorldLevelLoadState {
        &self.level_load
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

    fn start_client_level_load(&mut self) {
        self.level_load.phase = WorldLevelLoadPhase::WaitingForServer;
        self.level_load.starts = self.level_load.starts.saturating_add(1);
        self.level_load.loading_packets_received = 0;
    }

    fn advance_client_level_generation(&mut self) {
        self.client_level_generation = self.client_level_generation.saturating_add(1);
        self.debug_query = crate::ClientDebugQueryState::default();
    }

    fn apply_spawn_info(&mut self, spawn_info: &ProtocolSpawnInfo) {
        let profile = dimension_profile(spawn_info.dimension_type_id, &spawn_info.dimension);
        let dimension_type_entry = self
            .registry_entry_by_holder_id("minecraft:dimension_type", spawn_info.dimension_type_id);
        let dimension_type = dimension_type_client_projection_from_registry(dimension_type_entry);
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
        let environment_attributes = dimension_type
            .as_ref()
            .map(|projection| projection.environment_attributes)
            .unwrap_or_default();
        if let Some(err) = dimension_type
            .as_ref()
            .and_then(|projection| projection.environment_error)
        {
            self.record_apply_error("dimension_type_environment_attributes", err);
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
            biome_zoom_seed: spawn_info.seed,
            default_clock_resource_id,
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

    fn reset_local_player_for_respawn(&mut self, data_to_keep: i8) {
        let keep_entity_data = data_to_keep & RESPAWN_KEEP_ENTITY_DATA != 0;
        let keep_attribute_modifiers = data_to_keep & RESPAWN_KEEP_ATTRIBUTE_MODIFIERS != 0;
        let old_pose = self.local_player.pose;

        self.local_player.health = None;
        self.local_player.experience = None;
        self.local_player.camera = crate::CameraState::default();
        self.local_player.last_look_at = None;
        self.local_player.interaction = crate::LocalPlayerInteractionState::default();
        self.local_player.pose = keep_entity_data.then_some(old_pose).flatten().map(|pose| {
            crate::LocalPlayerPoseState {
                on_ground: false,
                horizontal_collision: false,
                fall_distance: 0.0,
                ..pose
            }
        });
        self.removed_player_vehicle_id = None;

        let Some(local_player_id) = self.local_player_id else {
            self.local_player_vehicle_id = None;
            return;
        };
        self.clear_local_player_mount(local_player_id);
        self.entities
            .with_mob_effects_mut(local_player_id, |effects| {
                effects.effects.clear();
            });
        self.entities
            .with_transient_events_mut(local_player_id, |events| {
                events.last_animation_action = None;
                events.last_event_id = None;
                events.last_hurt_yaw = None;
            });
        self.entities.with_damage_mut(local_player_id, |damage| {
            damage.last_damage = None;
        });
        if !keep_entity_data {
            self.entities
                .with_metadata_mut(local_player_id, |metadata| {
                    metadata.data_values.clear();
                });
        }
        if !keep_attribute_modifiers {
            self.entities
                .with_attributes_mut(local_player_id, |attributes| {
                    for attribute in &mut attributes.attributes {
                        attribute.modifiers.clear();
                    }
                });
        }
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
    }

    fn clear_level_bound_state(&mut self) {
        self.chunks.clear();
        self.first_chunk = None;
        self.block_destructions.clear();
        self.block_events.clear();
        self.chest_lids.clear();
        self.bell_shakes.clear();
        self.shulker_box_lids.clear();
        self.decorated_pot_wobbles.clear();
        self.enchanting_table_books.clear();
        self.enchanting_book_random = crate::EnchantingBookRandom::default();
        self.level_events.clear();
        self.client_audio.playing_jukebox_songs.clear();
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

    pub fn local_player_is_spectator(&self) -> bool {
        self.gameplay.game_type == VANILLA_GAME_TYPE_SPECTATOR_ID
    }

    pub fn local_player_has_reduced_debug_info(&self) -> bool {
        self.local_player_id.is_some() && self.gameplay.reduced_debug_info
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

#[derive(Debug, Clone, PartialEq)]
struct DimensionTypeClientProjection {
    dimension: Option<WorldDimension>,
    has_sky_light: Option<bool>,
    has_ceiling: Option<bool>,
    cardinal_lighting: Option<WorldCardinalLighting>,
    skybox: Option<WorldSkybox>,
    ambient_light: Option<f32>,
    default_clock_resource_id: Option<String>,
    environment_attributes: ClientEnvironmentAttributes,
    environment_error: Option<EnvironmentAttributeProjectionError>,
}

fn dimension_type_client_projection_from_registry(
    entry: Option<&crate::RegistryPacketEntry>,
) -> Option<DimensionTypeClientProjection> {
    let root = decode_nbt_root(entry?.raw_data()?).ok()??;
    let NbtValue::Compound(fields) = root else {
        return None;
    };
    let dimension = dimension_type_shape_from_fields(&fields);
    let has_sky_light = match find_nbt_entry(&fields, "has_skylight") {
        Some(NbtValue::Byte(value)) => Some(*value != 0),
        _ => None,
    };
    let has_ceiling = match find_nbt_entry(&fields, "has_ceiling") {
        Some(NbtValue::Byte(value)) => Some(*value != 0),
        _ => None,
    };
    let cardinal_lighting = match find_nbt_entry(&fields, "cardinal_light") {
        Some(NbtValue::String(name)) => cardinal_lighting_type_from_name(name),
        None => Some(WorldCardinalLighting::Default),
        _ => None,
    };
    let skybox = match find_nbt_entry(&fields, "skybox") {
        Some(NbtValue::String(name)) => skybox_from_name(name),
        None => Some(WorldSkybox::Overworld),
        _ => None,
    };
    let ambient_light = match find_nbt_entry(&fields, "ambient_light") {
        Some(NbtValue::Float(value)) => Some(*value),
        _ => None,
    };
    let default_clock_resource_id = match find_nbt_entry(&fields, "default_clock") {
        Some(NbtValue::String(resource_id)) => Some(resource_id.clone()),
        _ => None,
    };
    let (environment_attributes, environment_error) = match find_nbt_entry(&fields, "attributes") {
        Some(NbtValue::Compound(entries)) => match decode_client_environment_attributes(entries) {
            Ok(attributes) => (attributes, None),
            Err(err) => (ClientEnvironmentAttributes::default(), Some(err)),
        },
        Some(_) => (
            ClientEnvironmentAttributes::default(),
            Some(EnvironmentAttributeProjectionError {
                field: "attributes",
                reason: "expected compound",
            }),
        ),
        None => (ClientEnvironmentAttributes::default(), None),
    };
    Some(DimensionTypeClientProjection {
        dimension,
        has_sky_light,
        has_ceiling,
        cardinal_lighting,
        skybox,
        ambient_light,
        default_clock_resource_id,
        environment_attributes,
        environment_error,
    })
}

fn dimension_type_shape_from_fields(fields: &[(String, NbtValue)]) -> Option<WorldDimension> {
    // DimensionType.MIN_Y/MAX_Y/Y_SIZE with BlockPos.PACKED_Y_LENGTH = 12.
    const MIN_Y: i32 = -2032;
    const MAX_Y: i32 = 2031;
    const MAX_HEIGHT: i32 = 4064;

    let NbtValue::Int(min_y) = find_nbt_entry(fields, "min_y")? else {
        return None;
    };
    let NbtValue::Int(height) = find_nbt_entry(fields, "height")? else {
        return None;
    };
    let NbtValue::Int(logical_height) = find_nbt_entry(fields, "logical_height")? else {
        return None;
    };
    if !(MIN_Y..=MAX_Y).contains(min_y)
        || !(16..=MAX_HEIGHT).contains(height)
        || !(0..=MAX_HEIGHT).contains(logical_height)
        || logical_height > height
        || min_y.rem_euclid(16) != 0
        || height % 16 != 0
        || min_y.checked_add(*height)? > MAX_Y + 1
    {
        return None;
    }
    Some(WorldDimension {
        min_y: *min_y,
        height: *height,
    })
}

fn cardinal_lighting_type_from_name(name: &str) -> Option<WorldCardinalLighting> {
    match name {
        "default" => Some(WorldCardinalLighting::Default),
        "nether" => Some(WorldCardinalLighting::Nether),
        _ => None,
    }
}

fn skybox_from_name(name: &str) -> Option<WorldSkybox> {
    match name {
        "none" => Some(WorldSkybox::None),
        "overworld" => Some(WorldSkybox::Overworld),
        "end" => Some(WorldSkybox::End),
        _ => None,
    }
}

fn rounded_game_event_param(param: f32) -> i32 {
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
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
        AttributeSnapshot as ProtocolAttributeSnapshot,
        BlockDestruction as ProtocolBlockDestruction, BlockEvent as ProtocolBlockEvent,
        BlockPos as ProtocolBlockPos, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind, GlobalPos as ProtocolGlobalPos, LevelEvent as ProtocolLevelEvent,
        MapItemData as ProtocolMapItemData, MobEffectFlags, OpenScreen as ProtocolOpenScreen,
        PlayerExperience as ProtocolPlayerExperience, PlayerHealth as ProtocolPlayerHealth,
        RegistryData, RegistryDataEntry, SetCamera as ProtocolSetCamera,
        SetEntityData as ProtocolSetEntityData, UpdateAttributes as ProtocolUpdateAttributes,
        UpdateMobEffect as ProtocolUpdateMobEffect, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    use crate::entities::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use crate::{
        BlockPos, CameraState, ChunkColumn, ChunkPos, ChunkState, LightData, LocalPlayerPoseState,
    };

    #[test]
    fn play_login_updates_world_dimension_and_level_info() {
        let mut store = WorldStore::new();
        store.chunks.push(stale_chunk());

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
            store.chunks.push(chunk);
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
    fn respawn_updates_dimension_and_clears_old_chunks() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        store
            .available_levels
            .insert("minecraft:overworld".to_string());
        store.server_enforces_secure_chat = Some(true);
        store.chunks.push(stale_chunk());
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
            menu_type_id: 2,
            title: "Chest".to_string(),
            title_styled: Vec::new(),
        });

        assert_eq!(
            store.apply_respawn(&ProtocolRespawn {
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
            }),
            Some(7)
        );
        assert_eq!(store.open_container_id(), None);
        assert_eq!(store.chunk_count(), 1);
        assert_eq!(store.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
        assert_eq!(store.entity_count(), 1);
        assert_eq!(store.level_info().unwrap().biome_zoom_seed, 12345);
        assert_eq!(store.local_player().portal_cooldown, 6);
        assert_eq!(store.client_level_generation(), 1);
        let available_levels = store.available_levels().clone();
        let server_enforces_secure_chat = store.server_enforces_secure_chat();

        assert_eq!(
            store.apply_respawn(&ProtocolRespawn {
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
            }),
            None
        );

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
        let map = store.map_item(42).expect("map survives dimension respawn");
        assert_eq!(map.scale, 2);
        assert!(map.locked);
        assert_eq!(store.counters().maps_tracked, 1);
        assert_eq!(store.counters().entities_tracked, 0);
        assert_eq!(store.counters().respawns_received, 2);
        let level = store.level_info().unwrap();
        assert_eq!(level.biome_zoom_seed, 98765);
        assert_eq!(store.local_player().portal_cooldown, 2);
        assert_eq!(store.client_level_generation(), 2);
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
    fn respawn_without_keep_data_resets_local_player_runtime_state() {
        let mut store = respawn_state_with_local_player_data();

        store.apply_respawn(&respawn_packet(0));

        assert!(store.local_player().health.is_none());
        assert!(store.local_player().experience.is_none());
        assert_eq!(store.local_player_pose(), None);
        assert_eq!(store.local_player().camera, CameraState::default());
        assert_eq!(
            store.local_player().interaction,
            crate::LocalPlayerInteractionState::default()
        );

        let entity = store.probe_entity(123).unwrap();
        assert!(entity.data_values.is_empty());
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

        store.apply_respawn(&respawn_packet(3));

        assert!(store.local_player().health.is_none());
        assert!(store.local_player().experience.is_none());
        assert_eq!(
            store.local_player_pose(),
            Some(LocalPlayerPoseState {
                on_ground: false,
                horizontal_collision: false,
                fall_distance: 0.0,
                ..old_pose
            })
        );
        assert_eq!(store.local_player().camera, CameraState::default());
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
                operation_id: 1,
            }]
        );
    }

    #[test]
    fn clear_client_level_removes_level_bound_state_without_resetting_packet_counters() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        store.level = Some(WorldLevelInfo {
            dimension: "minecraft:the_nether".to_string(),
            dimension_type_id: 1,
            dimension_type_name: Some("minecraft:the_nether".to_string()),
            biome_zoom_seed: 0,
            default_clock_resource_id: None,
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
        store.chunks.push(stale_chunk());
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
            data: 27,
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
        store.apply_game_event(ProtocolGameEvent {
            event_id: 3,
            param: 3.0,
        });
        assert!(store.local_player_is_spectator());
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

        store.apply_respawn(&respawn_packet(0));
        assert_eq!(
            store.level_load().phase,
            WorldLevelLoadPhase::WaitingForServer
        );
        assert_eq!(store.level_load().starts, 2);
        assert_eq!(store.level_load().loading_packets_received, 0);
    }

    fn respawn_state_with_local_player_data() -> WorldStore {
        let mut store = WorldStore::new();
        store.local_player_id = Some(123);
        store.apply_add_entity(local_player_entity(123));
        store.apply_add_entity(protocol_add_entity(456));
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
            fall_distance: 8.0,
            sneaking: true,
            swimming: true,
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
                attribute_id: 21,
                base: 0.1,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:test_speed".to_string(),
                    amount: 0.25,
                    operation_id: 1,
                }],
            }],
        }));
        assert!(store.apply_update_mob_effect(ProtocolUpdateMobEffect {
            entity_id: 123,
            effect_id: 3,
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
            serializer_id: 0,
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
            entity_type_id: 7,
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
    fn dimension_type_registry_projects_direct_environment_attributes_and_diagnoses_modifiers() {
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
                "minecraft:visual/fog_color: attribute modifiers are not yet supported",
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

    fn dimension_type_nbt_with_default_clock(default_clock: &str) -> Vec<u8> {
        let mut payload = dimension_type_nbt(None, None, None, None, None, None);
        assert_eq!(payload.pop(), Some(0));
        payload.push(8);
        write_nbt_string(&mut payload, "default_clock");
        write_nbt_string(&mut payload, default_clock);
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
