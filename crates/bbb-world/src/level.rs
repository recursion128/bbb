use bbb_protocol::codec::Decoder;
use bbb_protocol::packets::{
    ClockUpdate as ProtocolClockUpdate, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
    GameEvent as ProtocolGameEvent, PlayLogin as ProtocolPlayLogin, PlayTime as ProtocolPlayTime,
    Respawn as ProtocolRespawn, TickingState as ProtocolTickingState,
    TickingStep as ProtocolTickingStep,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, WorldStore};

const VANILLA_SPECTATOR_GAME_TYPE_ID: i32 = 3;
const RESPAWN_KEEP_ATTRIBUTE_MODIFIERS: i8 = 1;
const RESPAWN_KEEP_ENTITY_DATA: i8 = 2;
const MAX_DIMENSION_TYPE_NBT_DEPTH: usize = 64;
const MAX_DIMENSION_TYPE_NBT_LIST_ITEMS: usize = 4096;

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
    #[serde(default)]
    pub cardinal_lighting: WorldCardinalLighting,
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

impl WorldLevelInfo {
    pub fn cardinal_lighting(&self) -> WorldCardinalLighting {
        self.cardinal_lighting
    }
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
    /// Vanilla `ClientLevel.Data.isHardcore` from the `ClientboundLoginPacket`
    /// `hardcore` flag (read by `Gui.extractHearts` for the hardcore heart
    /// sprite variants, Gui.java:834).
    #[serde(default)]
    pub hardcore: bool,
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
        if let Some(local_player_id) = self.local_player_id {
            self.clear_local_player_mount(local_player_id);
        } else {
            self.local_player_vehicle_id = None;
        }
        self.local_player_id = Some(login.player_id);
        self.gameplay.show_death_screen = login.show_death_screen;
        self.gameplay.do_limited_crafting = login.do_limited_crafting;
        self.gameplay.hardcore = login.hardcore;
        self.apply_spawn_info(&login.common_spawn_info);
    }

    pub fn apply_respawn(&mut self, respawn: &ProtocolRespawn) {
        self.counters.respawns_received += 1;
        self.reset_local_player_for_respawn(respawn.data_to_keep);
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

    pub fn advance_client_time(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }

        let Some(time) = self.world_time.as_mut() else {
            return;
        };

        let start_game_time = time.game_time;
        let game_time_delta = i64::from(ticks);
        time.game_time = time.game_time.saturating_add(game_time_delta);
        for clock in &mut time.clock_updates {
            advance_client_clock(clock, game_time_delta);
        }
        time.day_time = time
            .clock_updates
            .first()
            .map(|clock| clock.total_ticks)
            .unwrap_or(time.game_time);

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
        self.dimension = WorldDimension::default();
        self.level = None;
        self.gameplay = WorldGameplayState::default();
        self.world_border = crate::WorldBorderState::default();
        self.world_time = None;
        self.weather = crate::WorldWeatherState::default();
        self.sky_flash_time = 0;
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
        let cardinal_lighting = self
            .dimension_type_cardinal_lighting(spawn_info.dimension_type_id, &spawn_info.dimension);
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
            cardinal_lighting,
            last_death_location: spawn_info
                .last_death_location
                .as_ref()
                .map(world_global_pos_from_protocol),
            sea_level: spawn_info.sea_level,
            is_debug: spawn_info.is_debug,
            is_flat: spawn_info.is_flat,
        });
        self.set_game_type_from_spawn_info(
            i32::from(spawn_info.game_type),
            i32::from(spawn_info.previous_game_type),
        );
    }

    fn dimension_type_cardinal_lighting(
        &self,
        dimension_type_id: i32,
        dimension: &str,
    ) -> WorldCardinalLighting {
        dimension_type_cardinal_lighting_from_registry(
            self.registry_content("minecraft:dimension_type"),
            dimension_type_id,
        )
        .unwrap_or_else(|| built_in_dimension_cardinal_lighting(dimension_type_id, dimension))
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
        self.gameplay.game_type == VANILLA_SPECTATOR_GAME_TYPE_ID
    }

    pub fn world_time(&self) -> Option<&WorldTimeState> {
        self.world_time.as_ref()
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

fn dimension_type_cardinal_lighting_from_registry(
    registry: Option<&crate::RegistryContentState>,
    dimension_type_id: i32,
) -> Option<WorldCardinalLighting> {
    let registry = registry?;
    let index = usize::try_from(dimension_type_id).ok()?;
    let entry = registry.entries.get(index)?;
    let raw_data = entry.raw_data()?;
    dimension_type_cardinal_lighting_from_nbt(raw_data)
}

fn dimension_type_cardinal_lighting_from_nbt(raw_data: &[u8]) -> Option<WorldCardinalLighting> {
    match read_nbt_root_string_field(raw_data, "cardinal_light")? {
        Some(name) => cardinal_lighting_type_from_name(&name),
        None => Some(WorldCardinalLighting::Default),
    }
}

fn cardinal_lighting_type_from_name(name: &str) -> Option<WorldCardinalLighting> {
    match name {
        "default" => Some(WorldCardinalLighting::Default),
        "nether" => Some(WorldCardinalLighting::Nether),
        _ => None,
    }
}

fn read_nbt_root_string_field(raw_data: &[u8], field: &str) -> Option<Option<String>> {
    let mut decoder = Decoder::new(raw_data);
    let root_type = decoder.read_u8().ok()?;
    if root_type != 10 {
        return None;
    }

    loop {
        let tag_id = decoder.read_u8().ok()?;
        if tag_id == 0 {
            break;
        }
        let name = read_nbt_modified_utf8(&mut decoder)?;
        if name == field {
            if tag_id != 8 {
                return None;
            }
            let value = read_nbt_modified_utf8(&mut decoder)?;
            return Some(Some(value));
        }
        skip_nbt_payload(&mut decoder, tag_id, 0)?;
    }

    decoder.is_empty().then_some(None)
}

fn skip_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Option<()> {
    if depth > MAX_DIMENSION_TYPE_NBT_DEPTH {
        return None;
    }
    match tag_id {
        0 => Some(()),
        1 => decoder.read_exact(1, "nbt byte").ok().map(|_| ()),
        2 => decoder.read_exact(2, "nbt short").ok().map(|_| ()),
        3 | 5 => decoder.read_exact(4, "nbt int/float").ok().map(|_| ()),
        4 | 6 => decoder.read_exact(8, "nbt long/double").ok().map(|_| ()),
        7 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len, "nbt byte array").ok().map(|_| ())
        }
        8 => read_nbt_modified_utf8(decoder).map(|_| ()),
        9 => {
            let element_type = decoder.read_u8().ok()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_DIMENSION_TYPE_NBT_LIST_ITEMS || (element_type == 0 && len > 0) {
                return None;
            }
            for _ in 0..len {
                skip_nbt_payload(decoder, element_type, depth + 1)?;
            }
            Some(())
        }
        10 => {
            loop {
                let nested_type = decoder.read_u8().ok()?;
                if nested_type == 0 {
                    break;
                }
                read_nbt_modified_utf8(decoder)?;
                skip_nbt_payload(decoder, nested_type, depth + 1)?;
            }
            Some(())
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(4)?;
            decoder
                .read_exact(byte_len, "nbt int array")
                .ok()
                .map(|_| ())
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(8)?;
            decoder
                .read_exact(byte_len, "nbt long array")
                .ok()
                .map(|_| ())
        }
        _ => None,
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Option<usize> {
    let len = decoder.read_i32().ok()?;
    usize::try_from(len).ok()
}

fn read_nbt_modified_utf8(decoder: &mut Decoder<'_>) -> Option<String> {
    let len = decoder.read_u16().ok()? as usize;
    let bytes = decoder.read_exact(len, "nbt string").ok()?;
    let mut units = Vec::with_capacity(len);
    let mut cursor = 0;

    while cursor < bytes.len() {
        let b0 = bytes[cursor];
        if b0 & 0x80 == 0 {
            units.push(u16::from(b0));
            cursor += 1;
        } else if b0 & 0xe0 == 0xc0 {
            if cursor + 1 >= bytes.len() {
                return None;
            }
            let b1 = bytes[cursor + 1];
            if b1 & 0xc0 != 0x80 {
                return None;
            }
            units.push((u16::from(b0 & 0x1f) << 6) | u16::from(b1 & 0x3f));
            cursor += 2;
        } else if b0 & 0xf0 == 0xe0 {
            if cursor + 2 >= bytes.len() {
                return None;
            }
            let b1 = bytes[cursor + 1];
            let b2 = bytes[cursor + 2];
            if b1 & 0xc0 != 0x80 || b2 & 0xc0 != 0x80 {
                return None;
            }
            units.push(
                (u16::from(b0 & 0x0f) << 12) | (u16::from(b1 & 0x3f) << 6) | u16::from(b2 & 0x3f),
            );
            cursor += 3;
        } else {
            return None;
        }
    }

    String::from_utf16(&units).ok()
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
        AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
        AttributeSnapshot as ProtocolAttributeSnapshot,
        BlockDestruction as ProtocolBlockDestruction, BlockEvent as ProtocolBlockEvent,
        BlockPos as ProtocolBlockPos, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind, GlobalPos as ProtocolGlobalPos, LevelEvent as ProtocolLevelEvent,
        MobEffectFlags, PlayerExperience as ProtocolPlayerExperience,
        PlayerHealth as ProtocolPlayerHealth, RegistryData, RegistryDataEntry,
        SetCamera as ProtocolSetCamera, SetEntityData as ProtocolSetEntityData,
        UpdateAttributes as ProtocolUpdateAttributes, UpdateMobEffect as ProtocolUpdateMobEffect,
        Vec3d as ProtocolVec3d,
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
                last_death_location: Some(ProtocolGlobalPos {
                    dimension: "minecraft:overworld".to_string(),
                    pos: ProtocolBlockPos { x: 8, y: 64, z: -3 },
                }),
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
                hardcore: false,
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
            cardinal_lighting: WorldCardinalLighting::Nether,
            last_death_location: None,
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
        assert!(store.level_info().is_none());
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
    fn advance_client_time_ticks_game_time_and_clock_instances_like_vanilla() {
        let mut store = WorldStore::new();
        store.apply_world_time(ProtocolPlayTime {
            game_time: 100,
            clock_updates: vec![
                ProtocolClockUpdate {
                    clock_id: 1,
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
                    clock_id: 1,
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
                clock_id: 1,
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
                clock_id: 1,
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
                hardcore: false,
            }
        );
        assert_eq!(store.counters().game_event_packets, 3);
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
        assert_eq!(
            nether.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Nether
        );

        let mut remapped_nether = WorldStore::new();
        remapped_nether.apply_login(&login_packet(7, "minecraft:the_nether"));
        assert_eq!(
            remapped_nether.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Nether
        );

        let mut end = WorldStore::new();
        end.apply_login(&login_packet(2, "minecraft:the_end"));
        assert_eq!(
            end.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Default
        );
    }

    #[test]
    fn dimension_type_registry_cardinal_light_overrides_builtin_fallback() {
        let mut custom_nether = WorldStore::new();
        record_custom_dimension_type_registry(
            &mut custom_nether,
            dimension_type_nbt(Some("nether")),
        );
        custom_nether.apply_login(&login_packet(4, "example:custom"));
        assert_eq!(
            custom_nether.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Nether
        );

        let mut omitted_field = WorldStore::new();
        record_custom_dimension_type_registry(&mut omitted_field, dimension_type_nbt(None));
        omitted_field.apply_login(&login_packet(4, "minecraft:the_nether"));
        assert_eq!(
            omitted_field.level_info().unwrap().cardinal_lighting(),
            WorldCardinalLighting::Default
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

    fn dimension_type_nbt(cardinal_light: Option<&str>) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(3);
        write_nbt_string(&mut payload, "height");
        payload.extend_from_slice(&384_i32.to_be_bytes());
        if let Some(cardinal_light) = cardinal_light {
            payload.push(8);
            write_nbt_string(&mut payload, "cardinal_light");
            write_nbt_string(&mut payload, cardinal_light);
        }
        payload.push(0);
        payload
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
