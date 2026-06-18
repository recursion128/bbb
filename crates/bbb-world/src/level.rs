use bbb_protocol::packets::{
    ClockUpdate as ProtocolClockUpdate, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
    GameEvent as ProtocolGameEvent, PlayLogin as ProtocolPlayLogin, PlayTime as ProtocolPlayTime,
    Respawn as ProtocolRespawn, TickingState as ProtocolTickingState,
    TickingStep as ProtocolTickingStep,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const VANILLA_SPECTATOR_GAME_TYPE_ID: i32 = 3;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLevelInfo {
    pub dimension: String,
    pub dimension_type_id: i32,
    pub dimension_type_name: Option<String>,
    pub sea_level: i32,
    pub is_debug: bool,
    pub is_flat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldGameplayState {
    pub game_type: i32,
    pub game_type_name: String,
    pub previous_game_type: Option<i32>,
    pub previous_game_type_name: Option<String>,
    pub show_death_screen: bool,
    pub do_limited_crafting: bool,
}

impl Default for WorldGameplayState {
    fn default() -> Self {
        Self {
            game_type: 0,
            game_type_name: game_type_name(0).to_string(),
            previous_game_type: None,
            previous_game_type_name: None,
            show_death_screen: true,
            do_limited_crafting: false,
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
        if let Some(local_player_id) = self.local_player_id {
            self.clear_local_player_mount(local_player_id);
        } else {
            self.local_player_vehicle_id = None;
        }
        self.local_player_id = Some(login.player_id);
        self.gameplay.show_death_screen = login.show_death_screen;
        self.gameplay.do_limited_crafting = login.do_limited_crafting;
        self.apply_spawn_info(&login.common_spawn_info);
    }

    pub fn apply_respawn(&mut self, respawn: &ProtocolRespawn) {
        self.counters.respawns_received += 1;
        self.apply_spawn_info(&respawn.common_spawn_info);
    }

    pub fn apply_world_time(&mut self, time: ProtocolPlayTime) -> &WorldTimeState {
        self.counters.world_time_packets += 1;
        let clock_updates: Vec<_> = time
            .clock_updates
            .into_iter()
            .map(ClockUpdateState::from)
            .collect();
        let day_time = clock_updates
            .first()
            .map(|clock| clock.total_ticks)
            .unwrap_or(time.game_time);
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
            1 => {
                self.weather.rain_level = 0.0;
                self.weather.raining = false;
            }
            2 => {
                self.weather.rain_level = 1.0;
                self.weather.raining = true;
            }
            7 => {
                self.weather.rain_level = event.param.clamp(0.0, 1.0);
                self.weather.raining = self.weather.rain_level > 0.0;
            }
            8 => {
                self.weather.thunder_level = event.param.clamp(0.0, 1.0);
            }
            3 => {
                self.set_game_type_from_game_event_param(event.param);
            }
            11 => {
                self.gameplay.show_death_screen = event.param == 0.0;
            }
            12 => {
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

    pub fn clear_client_level(&mut self) {
        self.dimension = WorldDimension::default();
        self.level = None;
        self.gameplay = WorldGameplayState::default();
        self.world_border = crate::WorldBorderState::default();
        self.world_time = None;
        self.weather = crate::WorldWeatherState::default();
        self.ticking = crate::WorldTickingState::default();
        self.chunk_view = crate::ChunkViewState::default();
        self.last_block_changed_ack = None;
        self.local_player = crate::LocalPlayerState::default();
        self.local_player_id = None;
        self.local_player_vehicle_id = None;
        self.last_projectile_power = None;
        self.clear_level_bound_state();
    }

    fn apply_spawn_info(&mut self, spawn_info: &ProtocolSpawnInfo) {
        let profile = dimension_profile(spawn_info.dimension_type_id, &spawn_info.dimension);
        let dimension_key_changed = self
            .level
            .as_ref()
            .is_some_and(|level| level.dimension != spawn_info.dimension);
        if self.dimension != profile.dimension || dimension_key_changed {
            self.clear_level_bound_state();
        }
        self.dimension = profile.dimension;
        self.level = Some(WorldLevelInfo {
            dimension: spawn_info.dimension.clone(),
            dimension_type_id: spawn_info.dimension_type_id,
            dimension_type_name: profile.name.map(str::to_string),
            sea_level: spawn_info.sea_level,
            is_debug: spawn_info.is_debug,
            is_flat: spawn_info.is_flat,
        });
        self.set_game_type_from_spawn_info(
            i32::from(spawn_info.game_type),
            i32::from(spawn_info.previous_game_type),
        );
    }

    fn set_game_type_from_spawn_info(&mut self, game_type: i32, previous_game_type: i32) {
        let game_type = canonical_game_type_id(game_type);
        self.gameplay.game_type = game_type;
        self.gameplay.game_type_name = game_type_name(game_type).to_string();
        if previous_game_type == -1 {
            self.gameplay.previous_game_type = None;
            self.gameplay.previous_game_type_name = None;
        } else {
            let previous_game_type = canonical_game_type_id(previous_game_type);
            self.gameplay.previous_game_type = Some(previous_game_type);
            self.gameplay.previous_game_type_name =
                Some(game_type_name(previous_game_type).to_string());
        }
    }

    fn set_game_type_from_game_event_param(&mut self, param: f32) {
        let game_type = canonical_game_type_id(rounded_game_event_param(param));
        if self.gameplay.game_type != game_type {
            self.gameplay.previous_game_type = Some(self.gameplay.game_type);
            self.gameplay.previous_game_type_name = Some(self.gameplay.game_type_name.clone());
        }
        self.gameplay.game_type = game_type;
        self.gameplay.game_type_name = game_type_name(game_type).to_string();
    }

    fn clear_level_bound_state(&mut self) {
        self.chunks.clear();
        self.first_chunk = None;
        self.block_destructions.clear();
        self.block_events.clear();
        self.level_events.clear();
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

    pub fn local_player_is_spectator(&self) -> bool {
        self.gameplay.game_type == VANILLA_SPECTATOR_GAME_TYPE_ID
    }

    pub fn world_time(&self) -> Option<&WorldTimeState> {
        self.world_time.as_ref()
    }

    pub fn weather(&self) -> WorldWeatherState {
        self.weather
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

fn rounded_game_event_param(param: f32) -> i32 {
    if param.is_finite() {
        (param + 0.5).floor() as i32
    } else {
        0
    }
}

fn canonical_game_type_id(id: i32) -> i32 {
    if (0..=3).contains(&id) {
        id
    } else {
        0
    }
}

fn positive_tick_steps(tick_steps: i32) -> u32 {
    u32::try_from(tick_steps).unwrap_or(0)
}

fn game_type_name(id: i32) -> &'static str {
    match canonical_game_type_id(id) {
        1 => "creative",
        2 => "adventure",
        VANILLA_SPECTATOR_GAME_TYPE_ID => "spectator",
        _ => "survival",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, BlockDestruction as ProtocolBlockDestruction,
        BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos,
        LevelEvent as ProtocolLevelEvent, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    use crate::{ChunkColumn, ChunkPos, ChunkState, LightData};

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
            ],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
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
                last_death_location: None,
                portal_cooldown: 0,
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
        assert_eq!(store.counters().play_logins_received, 1);
        let level = store.level_info().unwrap();
        assert_eq!(level.dimension, "minecraft:the_nether");
        assert_eq!(level.dimension_type_id, 1);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_nether")
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
            }
        );
    }

    #[test]
    fn respawn_updates_dimension_and_clears_old_chunks() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        store.chunks.push(stale_chunk());
        store.first_chunk = Some(ChunkPos { x: 1, z: -2 });
        store.apply_add_entity(protocol_add_entity(123));

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
                portal_cooldown: 0,
                sea_level: 32,
            },
            data_to_keep: 3,
        });
        assert_eq!(store.chunk_count(), 1);
        assert_eq!(store.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
        assert_eq!(store.entity_count(), 1);

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
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 3,
        });

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
        assert_eq!(store.counters().entities_tracked, 0);
        assert_eq!(store.counters().respawns_received, 2);
        let level = store.level_info().unwrap();
        assert_eq!(level.dimension, "minecraft:the_end");
        assert_eq!(level.dimension_type_id, 2);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_end")
        );
        assert_eq!(store.gameplay().game_type, 1);
        assert_eq!(store.gameplay().game_type_name, "creative");
        assert_eq!(store.gameplay().previous_game_type, Some(1));
        assert_eq!(
            store.gameplay().previous_game_type_name.as_deref(),
            Some("creative")
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
            sea_level: 32,
            is_debug: false,
            is_flat: false,
        });
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
        assert!(store.level_info().is_none());
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.first_chunk(), None);
        assert_eq!(store.entity_count(), 0);
        assert!(store.block_destructions.is_empty());
        assert!(store.block_events.is_empty());
        assert!(store.level_events.is_empty());
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
            }
        );
        assert_eq!(store.counters().game_event_packets, 3);
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
}
