//! The weather cycle: the save-wide `WeatherData` timers and the per-level rain
//! and thunder interpolation `ServerLevel.advanceWeatherCycle` moves.
//!

use crate::chunk::ChunkStore;
use crate::connection::ClientboundPlayPacket;
use crate::random::GameRandom;
use crate::world::LevelId;
use bbb_protocol::game_event_types::{
    VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID, VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID,
    VANILLA_CLIENT_GAME_EVENT_STOP_RAINING_ID, VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID;
use bbb_protocol::packets::{BlockPos, GameEvent};
use bbb_registry_codec::biome_climate::height_adjusted_temperature;
use bbb_server_datapack::worldgen::BiomeClimate;
use bbb_server_persistence::Java26WeatherData;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;

use crate::commands::CommandWeatherKind;

use super::WorldEvent;

/// `GameRules.ADVANCE_WEATHER` (`GameRules.java:25`).
const RULE_ADVANCE_WEATHER: &str = "minecraft:advance_weather";

/// The registry `Level.getBiome` resolves against.
const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";

/// `ServerLevel.RAIN_DELAY` (`ServerLevel.java:187`).
const RAIN_DELAY: (i32, i32) = (12_000, 180_000);
/// `ServerLevel.RAIN_DURATION` (`ServerLevel.java:188`).
const RAIN_DURATION: (i32, i32) = (12_000, 24_000);
/// `ServerLevel.THUNDER_DELAY` (`ServerLevel.java:189`).
const THUNDER_DELAY: (i32, i32) = (12_000, 180_000);
/// `ServerLevel.THUNDER_DURATION` (`ServerLevel.java:190`).
const THUNDER_DURATION: (i32, i32) = (3_600, 15_600);

fn sample_uniform_int(random: &GameRandom, (min, max): (i32, i32)) -> i32 {
    random.next_int(max - min + 1) + min
}

/// `Biome.Precipitation`, the three values `Level.precipitationAt` answers with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Precipitation {
    None,
    Rain,
    Snow,
}

/// `Biome.getPrecipitationAt` (`Biome.java:105-111`).
pub(super) fn precipitation_at(
    climate: BiomeClimate,
    pos: BlockPos,
    sea_level: i32,
) -> Precipitation {
    if !climate.has_precipitation {
        return Precipitation::None;
    }
    if cold_enough_to_snow(climate, pos, sea_level) {
        Precipitation::Snow
    } else {
        Precipitation::Rain
    }
}

pub(super) fn cold_enough_to_snow(climate: BiomeClimate, pos: BlockPos, sea_level: i32) -> bool {
    height_adjusted_temperature(
        climate.temperature,
        climate.modifier,
        pos.x,
        pos.y,
        pos.z,
        sea_level,
    ) < 0.15
}

/// The climate of every biome, indexed the way `ChunkStore::biome_id_at`
/// answers: by position in the published `minecraft:worldgen/biome` registry.
///
/// The published order is the one the block store's biome ids were assigned
/// against, so the table is built over it rather than over the datapack
/// generation's own element list; the generation only supplies each element's
/// climate. A level with no generation gets an empty table and never reports
/// precipitation rather than reading built-in registry data a session was never
/// told about.
pub(super) fn biome_climates(
    registries: &crate::registry_sync::SynchronizedRegistries,
    datapacks: Option<&bbb_server_datapack::startup::DatapackStartup>,
) -> Vec<Option<BiomeClimate>> {
    let Some(datapacks) = datapacks else {
        return Vec::new();
    };
    let Some(generation) = datapacks.registries.registry(BIOME_REGISTRY) else {
        return Vec::new();
    };
    registries
        .published()
        .iter()
        .find(|published| published.registry == BIOME_REGISTRY)
        .map(|published| {
            published
                .entries
                .iter()
                .map(|entry| {
                    generation
                        .elements
                        .iter()
                        .position(|element| element.id == entry.id)
                        .and_then(|index| datapacks.registries.worldgen.biome_climates.get(index))
                        .copied()
                })
                .collect()
        })
        .unwrap_or_default()
}

///
/// The three arms are not one broadcast: the first two go to the level's own
/// dimension (`:761`, `:767`) and the third goes to every session on the server
/// (`:772-778`), which is what the missing dimension argument on those four
/// `broadcastAll` calls means.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct WeatherBroadcast {
    /// `:758-762`, carrying `Level.rainLevel` itself.
    pub(super) rain_level_change: Option<f32>,
    /// `:764-768`, carrying `Level.thunderLevel` itself rather than
    /// `getThunderLevel(1.0F)`.
    pub(super) thunder_level_change: Option<f32>,
    /// `:770-779`, the whole-server arm.
    pub(super) raining_changed: Option<RainingChanged>,
}

/// The three packets `:771-778` sends when `isRaining` flipped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RainingChanged {
    /// `START_RAINING` when it began, `STOP_RAINING` when it ended
    /// (`:772-774`).
    pub(super) started: bool,
    /// `:777`, `Level.rainLevel`.
    pub(super) rain_level: f32,
    /// `:778`, `Level.thunderLevel`.
    pub(super) thunder_level: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LevelWeather {
    can_have_weather: bool,
    rain_level: f32,
    o_rain_level: f32,
    thunder_level: f32,
    o_thunder_level: f32,
    /// Climate by `minecraft:worldgen/biome` registry index, which is what
    /// `ChunkStore::biome_id_at` answers with. Empty for a level built with no
    /// datapack generation, which then never reports precipitation.
    biome_climates: Vec<Option<BiomeClimate>>,
}

impl LevelWeather {
    pub(super) fn new(
        has_skylight: bool,
        has_ceiling: bool,
        is_end: bool,
        biome_climates: Vec<Option<BiomeClimate>>,
    ) -> Self {
        Self {
            can_have_weather: has_skylight && !has_ceiling && !is_end,
            rain_level: 0.0,
            o_rain_level: 0.0,
            thunder_level: 0.0,
            o_thunder_level: 0.0,
            biome_climates,
        }
    }

    ///
    /// It writes the current levels and leaves the previous ones at zero, so a
    /// save restored mid-storm reports rain on its first tick.
    pub(super) fn prepare(&mut self, data: Java26WeatherData) {
        if !self.can_have_weather {
            return;
        }
        if data.raining {
            self.rain_level = 1.0;
            if data.thundering {
                self.thunder_level = 1.0;
            }
        }
    }

    pub(crate) fn rain_level(&self) -> f32 {
        self.rain_level
    }

    pub(crate) fn thunder_level(&self) -> f32 {
        self.thunder_level * self.rain_level
    }

    pub(crate) fn can_have_weather(&self) -> bool {
        self.can_have_weather
    }

    pub(crate) fn is_raining(&self) -> bool {
        self.can_have_weather && self.rain_level() > 0.2
    }

    pub(crate) fn is_thundering(&self) -> bool {
        self.can_have_weather && self.thunder_level() > 0.9
    }

    pub(super) fn loot_weather(&self) -> crate::loot_runtime::LootWeather {
        crate::loot_runtime::LootWeather {
            raining: self.is_raining(),
            thundering: self.is_thundering(),
        }
    }

    ///
    /// `advance_weather` is the `minecraft:advance_weather` game rule
    /// (`:699`); with it off the timers stand still and only the interpolation
    /// keeps moving towards the flags already stored.
    pub(super) fn advance(
        &mut self,
        data: &mut Java26WeatherData,
        advance_weather: bool,
        random: &GameRandom,
    ) -> WeatherBroadcast {
        let was_raining = self.is_raining();
        if self.can_have_weather {
            if advance_weather {
                let mut clear_weather_time = data.clear_weather_time;
                let mut thunder_time = data.thunder_time;
                let mut rain_time = data.rain_time;
                let mut thundering = data.thundering;
                let mut raining = data.raining;
                if clear_weather_time > 0 {
                    // `:705-710`: a clear countdown overrides both timers and
                    // leaves each one tick from expiring if it was set.
                    clear_weather_time -= 1;
                    thunder_time = i32::from(!thundering);
                    rain_time = i32::from(!raining);
                    thundering = false;
                    raining = false;
                } else {
                    if thunder_time > 0 {
                        thunder_time -= 1;
                        if thunder_time == 0 {
                            thundering = !thundering;
                        }
                    } else if thundering {
                        thunder_time = sample_uniform_int(random, THUNDER_DURATION);
                    } else {
                        thunder_time = sample_uniform_int(random, THUNDER_DELAY);
                    }
                    if rain_time > 0 {
                        rain_time -= 1;
                        if rain_time == 0 {
                            raining = !raining;
                        }
                    } else if raining {
                        rain_time = sample_uniform_int(random, RAIN_DURATION);
                    } else {
                        rain_time = sample_uniform_int(random, RAIN_DELAY);
                    }
                }
                data.thunder_time = thunder_time;
                data.rain_time = rain_time;
                data.clear_weather_time = clear_weather_time;
                data.thundering = thundering;
                data.raining = raining;
            }
            self.o_thunder_level = self.thunder_level;
            self.thunder_level += if data.thundering { 0.01 } else { -0.01 };
            self.thunder_level = self.thunder_level.clamp(0.0, 1.0);
            self.o_rain_level = self.rain_level;
            self.rain_level += if data.raining { 0.01 } else { -0.01 };
            self.rain_level = self.rain_level.clamp(0.0, 1.0);
        }
        WeatherBroadcast {
            rain_level_change: (self.o_rain_level != self.rain_level).then_some(self.rain_level),
            thunder_level_change: (self.o_thunder_level != self.thunder_level)
                .then_some(self.thunder_level),
            raining_changed: (was_raining != self.is_raining()).then_some(RainingChanged {
                started: !was_raining,
                rain_level: self.rain_level,
                thunder_level: self.thunder_level,
            }),
        }
    }

    ///
    /// The `canSeeSky` gate (`:954-956`) is not a separate read here:
    /// `Level.canSeeSky` is the sky brightness at the maximum light level, and
    /// this server publishes a full-brightness sky array for every level with
    /// sky light (`simulation_brightness`), so under `canHaveWeather` -- which
    /// already requires sky light -- it always holds. The roof case it would
    /// catch is caught by the heightmap gate on the line after it.
    ///
    /// A column with no heightmap yet, or a biome this reader could not turn
    /// into a climate, answers `None` rather than guessing at rain.
    pub(super) fn precipitation_at(&self, chunks: &ChunkStore, pos: BlockPos) -> Precipitation {
        if !self.is_raining() {
            return Precipitation::None;
        }
        let Some(height) = chunks.final_heightmap_first_available(
            pos.x >> 4,
            pos.z >> 4,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            pos.x & 15,
            pos.z & 15,
        ) else {
            return Precipitation::None;
        };
        if height > pos.y {
            return Precipitation::None;
        }
        let Some(climate) = self.climate_at(chunks, pos) else {
            return Precipitation::None;
        };
        precipitation_at(climate, pos, chunks.sea_level())
    }

    pub(super) fn has_biome_climates(&self) -> bool {
        !self.biome_climates.is_empty()
    }

    /// The climate of `Level.getBiome(pos)`, or `None` for a biome this reader
    /// could not turn into one.
    pub(super) fn climate_at(&self, chunks: &ChunkStore, pos: BlockPos) -> Option<BiomeClimate> {
        let biome = chunks.biome_id_at(pos.x, pos.y, pos.z)?;
        *self.biome_climates.get(usize::try_from(biome).ok()?)?
    }

    pub(super) fn is_raining_at(&self, chunks: &ChunkStore, pos: BlockPos) -> bool {
        self.precipitation_at(chunks, pos) == Precipitation::Rain
    }
}

impl super::ServerWorld {
    pub(super) fn advance_weather_cycle(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let advance_weather = !matches!(
            self.admin.game_rule_value(RULE_ADVANCE_WEATHER),
            Some(GameRuleValue::Boolean(false))
        );
        let mut data = self.weather;
        let level = self.level_mut_by_id(dimension);
        let broadcast = level
            .weather
            .advance(&mut data, advance_weather, &level.random);
        self.weather = data;
        // `:758-768`: each level packet goes to the sessions in this level's
        // own dimension.
        if let Some(rain_level) = broadcast.rain_level_change {
            self.broadcast_weather_in(
                dimension,
                VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID,
                rain_level,
                events,
            );
        }
        if let Some(thunder_level) = broadcast.thunder_level_change {
            self.broadcast_weather_in(
                dimension,
                VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
                thunder_level,
                events,
            );
        }
        // `:770-779`: the start/stop arm and the two levels it re-sends carry
        // no dimension, so they reach every session on the server.
        if let Some(changed) = broadcast.raining_changed {
            let started = if changed.started {
                VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID
            } else {
                VANILLA_CLIENT_GAME_EVENT_STOP_RAINING_ID
            };
            self.broadcast(weather_packet(started, 0.0), events);
            self.broadcast(
                weather_packet(
                    VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID,
                    changed.rain_level,
                ),
                events,
            );
            self.broadcast(
                weather_packet(
                    VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
                    changed.thunder_level,
                ),
                events,
            );
        }
    }

    /// `PlayerList.broadcastAll(packet, dimension)`, the level-scoped arm.
    fn broadcast_weather_in(
        &self,
        dimension: &LevelId,
        event_id: u8,
        param: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let packet = weather_packet(event_id, param);
        let mut recipients = Vec::new();
        for (&connection_id, player) in self.levels.player_entries() {
            if &player.dimension != dimension {
                continue;
            }
            recipients.push(connection_id);
        }
        crate::world::player_packets::broadcast_to(recipients, packet, events);
    }

    pub(super) fn weather_join_packets(&self, dimension: &LevelId) -> Vec<ClientboundPlayPacket> {
        let weather = &self.level_by_id(dimension).weather;
        if !weather.is_raining() {
            return Vec::new();
        }
        vec![
            weather_packet(VANILLA_CLIENT_GAME_EVENT_START_RAINING_ID, 0.0),
            weather_packet(
                VANILLA_CLIENT_GAME_EVENT_RAIN_LEVEL_CHANGE_ID,
                weather.rain_level(),
            ),
            weather_packet(
                VANILLA_CLIENT_GAME_EVENT_THUNDER_LEVEL_CHANGE_ID,
                weather.thunder_level(),
            ),
        ]
    }

    /// `WeatherCommand.setClear` / `setRain` / `setThunder`
    /// (`WeatherCommand.java:48-63`): `getDuration` (`:44-46`) draws an absent
    /// duration off the source level's own random.
    pub(crate) fn set_weather_by_command(
        &mut self,
        kind: crate::commands::CommandWeatherKind,
        duration: Option<i32>,
        level: &bbb_server_types::save::Java26LevelKey,
    ) {
        use crate::commands::CommandWeatherKind;
        let duration = match duration {
            Some(duration) => duration,
            None => sample_default_duration(
                kind,
                &self.level_mut_by_id(&self.require_level_id(level)).random,
            ),
        };
        match kind {
            CommandWeatherKind::Clear => self.set_weather_parameters(duration, 0, false, false),
            CommandWeatherKind::Rain => self.set_weather_parameters(0, duration, true, false),
            CommandWeatherKind::Thunder => self.set_weather_parameters(0, duration, true, true),
        }
    }

    pub(super) fn set_weather_parameters(
        &mut self,
        clear_time: i32,
        rain_time: i32,
        raining: bool,
        thundering: bool,
    ) {
        self.weather = Java26WeatherData {
            clear_weather_time: clear_time,
            rain_time,
            thunder_time: rain_time,
            raining,
            thundering,
        };
    }
}

pub(super) fn sample_default_duration(kind: CommandWeatherKind, random: &GameRandom) -> i32 {
    sample_uniform_int(
        random,
        match kind {
            CommandWeatherKind::Clear => RAIN_DELAY,
            CommandWeatherKind::Rain => RAIN_DURATION,
            CommandWeatherKind::Thunder => THUNDER_DURATION,
        },
    )
}

/// The readers no production body has taken up yet: these two separate
/// `Level.thunderLevel` from `getThunderLevel(1.0F)`, which only the tests need
/// to tell apart.
#[cfg(test)]
impl LevelWeather {
    pub(crate) fn thunder_level_field(&self) -> f32 {
        self.thunder_level
    }
}

#[cfg(test)]
impl super::ServerWorld {
    /// The live weather of one level.
    pub(crate) fn level_weather(&self, dimension: Java26BuiltinDimension) -> &LevelWeather {
        &self.level(dimension).weather
    }

    /// The save-wide store.
    pub(crate) fn weather_data(&self) -> Java26WeatherData {
        self.weather
    }

    pub(crate) fn is_raining_at(&self, dimension: Java26BuiltinDimension, pos: BlockPos) -> bool {
        let level = self.level(dimension);
        level.weather.is_raining_at(&level.chunks, pos)
    }

    pub(crate) fn motion_blocking_height(
        &self,
        dimension: Java26BuiltinDimension,
        x: i32,
        z: i32,
    ) -> Option<i32> {
        self.level(dimension)
            .chunks
            .final_heightmap_first_available(
                x >> 4,
                z >> 4,
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
                x & 15,
                z & 15,
            )
    }

    pub(crate) fn precipitation_at(
        &self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) -> Precipitation {
        let level = self.level(dimension);
        level.weather.precipitation_at(&level.chunks, pos)
    }
}

fn weather_packet(event_id: u8, param: f32) -> ClientboundPlayPacket {
    ClientboundPlayPacket::GameEvent(GameEvent { event_id, param })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overworld() -> LevelWeather {
        LevelWeather::new(true, false, false, Vec::new())
    }

    fn stormy() -> Java26WeatherData {
        Java26WeatherData {
            clear_weather_time: 0,
            rain_time: 5,
            thunder_time: 7,
            raining: true,
            thundering: true,
        }
    }

    fn seeded() -> GameRandom {
        GameRandom::with_seed(42)
    }

    #[test]
    fn only_a_sky_lit_ceilingless_non_end_level_has_weather() {
        assert!(LevelWeather::new(true, false, false, Vec::new()).can_have_weather());
        // The nether: a ceiling and no sky light.
        assert!(!LevelWeather::new(false, true, false, Vec::new()).can_have_weather());
        // The end is refused by name even though it has no sky light either.
        assert!(!LevelWeather::new(true, false, true, Vec::new()).can_have_weather());
    }

    #[test]
    fn prepare_puts_a_restored_storm_at_full_rain_and_thunder() {
        let mut weather = overworld();
        assert!(!weather.is_raining());
        weather.prepare(stormy());
        assert_eq!(weather.rain_level(), 1.0);
        assert_eq!(weather.thunder_level_field(), 1.0);
        assert!(weather.is_raining());
        assert!(weather.is_thundering());
    }

    /// Rain without thunder leaves the thunder level alone (`:689-691`).
    #[test]
    fn prepare_leaves_thunder_at_zero_for_rain_alone() {
        let mut weather = overworld();
        weather.prepare(Java26WeatherData {
            raining: true,
            thundering: false,
            ..stormy()
        });
        assert_eq!(weather.rain_level(), 1.0);
        assert_eq!(weather.thunder_level_field(), 0.0);
        assert!(weather.is_raining());
        assert!(!weather.is_thundering());
    }

    #[test]
    fn the_thunder_level_the_join_sends_is_scaled_by_rain_and_the_field_is_not() {
        let mut weather = overworld();
        weather.prepare(stormy());
        let mut data = stormy();
        // One tick with `advance_weather` off moves only the interpolation, and
        // both levels are already clamped at one, so drive the rain down.
        data.raining = false;
        weather.advance(&mut data, false, &seeded());
        assert_eq!(weather.rain_level(), 0.99);
        assert_eq!(weather.thunder_level_field(), 1.0);
        assert_eq!(weather.thunder_level(), 0.99);
    }

    #[test]
    fn a_clear_countdown_zeroes_both_flags_and_arms_both_timers() {
        let mut weather = overworld();
        let mut data = Java26WeatherData {
            clear_weather_time: 10,
            rain_time: 4000,
            thunder_time: 4000,
            raining: true,
            thundering: true,
        };
        weather.advance(&mut data, true, &seeded());
        assert_eq!(data.clear_weather_time, 9);
        // Both timers are set from the flag that was true, so a storm that was
        // running is left at zero and one that was not is left one tick out.
        assert_eq!(data.thunder_time, 0);
        assert_eq!(data.rain_time, 0);
        assert!(!data.raining);
        assert!(!data.thundering);

        let mut clear = Java26WeatherData {
            clear_weather_time: 10,
            rain_time: 4000,
            thunder_time: 4000,
            raining: false,
            thundering: false,
        };
        overworld().advance(&mut clear, true, &seeded());
        assert_eq!(clear.thunder_time, 1);
        assert_eq!(clear.rain_time, 1);
    }

    /// `:712-730`: a timer that reaches zero flips its flag.
    #[test]
    fn a_timer_that_runs_out_flips_its_flag() {
        let mut data = Java26WeatherData {
            clear_weather_time: 0,
            rain_time: 1,
            thunder_time: 1,
            raining: false,
            thundering: false,
        };
        overworld().advance(&mut data, true, &seeded());
        assert_eq!(data.rain_time, 0);
        assert_eq!(data.thunder_time, 0);
        assert!(data.raining);
        assert!(data.thundering);
    }

    /// `:716-720` and `:726-730`: an expired timer is resampled from the
    /// provider the *current* flag selects.
    #[test]
    fn an_expired_timer_resamples_the_provider_its_flag_selects() {
        let mut dry = Java26WeatherData::default();
        let random = seeded();
        overworld().advance(&mut dry, true, &random);
        // Not raining and not thundering: both delays, both 12000..=180000.
        assert!((12_000..=180_000).contains(&dry.thunder_time));
        assert!((12_000..=180_000).contains(&dry.rain_time));

        let mut wet = Java26WeatherData {
            raining: true,
            thundering: true,
            ..Java26WeatherData::default()
        };
        overworld().advance(&mut wet, true, &seeded());
        // The durations are narrower than the delays and cannot be mistaken
        // for them: thunder 3600..=15600, rain 12000..=24000.
        assert!((3_600..=15_600).contains(&wet.thunder_time));
        assert!((12_000..=24_000).contains(&wet.rain_time));
    }

    /// `:740-755`: one hundredth per tick, clamped, with the previous level
    /// kept for the change test.
    #[test]
    fn the_interpolation_steps_one_hundredth_and_clamps() {
        let mut weather = overworld();
        let mut data = Java26WeatherData {
            raining: true,
            thundering: false,
            ..Java26WeatherData::default()
        };
        for _ in 0..3 {
            weather.advance(&mut data, false, &seeded());
        }
        assert!((weather.rain_level() - 0.03).abs() < 1e-6);
        assert_eq!(weather.thunder_level_field(), 0.0);
        // A level already at the floor does not go below it.
        let mut dry = overworld();
        dry.advance(&mut Java26WeatherData::default(), false, &seeded());
        assert_eq!(dry.rain_level(), 0.0);
    }

    /// `:758-779`: the level arms fire only on a change, and the raining arm
    /// carries the raw thunder level rather than the scaled one.
    #[test]
    fn the_broadcast_names_only_the_arms_that_changed() {
        let mut weather = overworld();
        let mut data = Java26WeatherData {
            raining: true,
            thundering: true,
            ..Java26WeatherData::default()
        };
        let first = weather.advance(&mut data, false, &seeded());
        assert_eq!(first.rain_level_change, Some(0.01));
        assert_eq!(first.thunder_level_change, Some(0.01));
        // `isRaining` needs a rain level above 0.2, so twenty more ticks pass
        // before the whole-server arm fires.
        assert_eq!(first.raining_changed, None);
        let mut flipped = None;
        for _ in 0..40 {
            if let Some(change) = weather.advance(&mut data, false, &seeded()).raining_changed {
                flipped = Some(change);
                break;
            }
        }
        let flipped = flipped.expect("the rain level crosses 0.2");
        assert!(flipped.started);
        assert_eq!(flipped.rain_level, weather.rain_level());
        assert_eq!(flipped.thunder_level, weather.thunder_level_field());
        assert_ne!(flipped.thunder_level, weather.thunder_level());
    }

    /// `:697`: a level without weather advances neither the timers nor the
    /// interpolation, and broadcasts nothing.
    #[test]
    fn a_level_without_weather_advances_nothing() {
        let mut nether = LevelWeather::new(false, true, false, Vec::new());
        let mut data = stormy();
        let before = data;
        let broadcast = nether.advance(&mut data, true, &seeded());
        assert_eq!(data, before);
        assert_eq!(broadcast, WeatherBroadcast::default());
        assert_eq!(nether.rain_level(), 0.0);
        assert!(!nether.is_raining());
    }

    /// `:699`: with the game rule off the timers stand still, but the
    /// interpolation still moves towards the flags already stored.
    #[test]
    fn the_game_rule_gates_the_timers_and_not_the_interpolation() {
        let mut weather = overworld();
        let mut data = stormy();
        let before = data;
        weather.advance(&mut data, false, &seeded());
        assert_eq!(data, before);
        assert_eq!(weather.rain_level(), 0.01);
    }

    #[test]
    fn published_biome_climates_decide_between_rain_snow_and_nothing() {
        let startup = crate::registry_sync::builtin_datapack_startup_shared();
        let registries =
            crate::registry_sync::SynchronizedRegistries::load(Some(&startup)).unwrap();
        let mut reordered = (*startup).clone();
        reordered
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == BIOME_REGISTRY)
            .unwrap()
            .elements
            .reverse();
        reordered.registries.worldgen.biome_climates.reverse();
        let climates = biome_climates(&registries, Some(&reordered));
        let pos = BlockPos { x: 0, y: 64, z: 0 };
        for (biome, expected) in [
            ("minecraft:plains", Precipitation::Rain),
            ("minecraft:snowy_taiga", Precipitation::Snow),
            ("minecraft:desert", Precipitation::None),
        ] {
            let index = registries
                .registry_element_index(BIOME_REGISTRY, biome)
                .unwrap();
            assert_eq!(
                precipitation_at(climates[index as usize].unwrap(), pos, 63),
                expected
            );
        }
    }
}
