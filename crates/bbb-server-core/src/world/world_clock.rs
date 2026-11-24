use std::sync::Arc;

use crate::connection::ClientboundPlayPacket;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{ClockUpdate, PlayTime};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use bbb_server_datapack::world_clock::WorldClockCatalog;
use bbb_server_persistence::{Java26WorldClockEntry, Java26WorldClockState, Java26WorldClocks};
use bbb_server_types::save::{SaveError, SaveResult};
use bbb_server_types::{Result, ServerWorldError};

use crate::commands::CommandClockChange;
use crate::registry_sync::SynchronizedRegistries;

use super::{ServerWorld, WorldEvent};

pub(super) const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";
pub(super) const OVERWORLD_CLOCK: &str = "minecraft:overworld";
const ADVANCE_TIME_RULE: &str = "minecraft:advance_time";
const DAY_TICKS: i64 = 24_000;
const NOON_TICKS: i64 = 6_000;

/// `ClockTimeMarker.resolveTimeToMoveTo` (`ClockTimeMarker.java:20-36`).
pub(super) fn strict_next_noon(total_ticks: i64) -> i64 {
    let from = total_ticks % DAY_TICKS;
    let duration = NOON_TICKS - from;
    let duration = if duration > 0 {
        duration
    } else {
        DAY_TICKS + duration
    };
    total_ticks.wrapping_add(duration)
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ServerWorldClocks {
    states: Vec<Java26WorldClockState>,
    catalog: Arc<WorldClockCatalog>,
}

impl ServerWorldClocks {
    /// `ServerClockManager.init` (`ServerClockManager.java:39-53`), whose clock
    /// instances follow the registry the session is told about: the catalog is
    /// checked against that publication so an index means the same clock to the
    /// save, the wire and the command surface.
    pub(super) fn defaults(
        registries: &SynchronizedRegistries,
        startup: Option<&DatapackStartup>,
    ) -> Result<Self> {
        let count = registries
            .registry_len(WORLD_CLOCK_REGISTRY)
            .ok_or_else(|| {
                ServerWorldError::InvalidConfiguration(
                    "the synchronized catalog contains no world_clock registry".to_string(),
                )
            })?;
        let catalog = world_clock_catalog(startup)?;
        if catalog.clocks().len() != count {
            return Err(ServerWorldError::InvalidConfiguration(
                "the world-clock catalog and the published registry differ in length".to_string(),
            ));
        }
        for (index, clock) in catalog.clocks().iter().enumerate() {
            let index_i32 = i32::try_from(index).map_err(|_| {
                ServerWorldError::InvalidConfiguration(
                    "world-clock index does not fit the protocol".to_string(),
                )
            })?;
            let published = registries
                .registry_element_id(WORLD_CLOCK_REGISTRY, index_i32)
                .ok_or_else(|| {
                    ServerWorldError::InvalidConfiguration(
                        "world-clock index is not published".to_string(),
                    )
                })?;
            if clock.resource_id != published {
                return Err(ServerWorldError::InvalidConfiguration(format!(
                    "world-clock catalog holds {} at index {index}, published holder is {published}",
                    clock.resource_id
                )));
            }
        }
        Ok(Self {
            states: vec![Java26WorldClockState::default(); count],
            catalog,
        })
    }

    fn restore(
        &mut self,
        registries: &SynchronizedRegistries,
        saved: &Java26WorldClocks,
    ) -> SaveResult<()> {
        if saved.entries().len() != self.states.len() {
            return Err(clock_sequence_error(
                "saved world-clock count differs from the published registry",
            ));
        }
        for (index, entry) in saved.entries().iter().enumerate() {
            let index_i32 = i32::try_from(index)
                .map_err(|_| clock_sequence_error("world-clock index does not fit the protocol"))?;
            let published = registries
                .registry_element_id(WORLD_CLOCK_REGISTRY, index_i32)
                .ok_or_else(|| clock_sequence_error("world-clock index is not published"))?;
            if entry.resource_id != published {
                return Err(clock_sequence_error(format!(
                    "saved holder {} occupies index {index}, published holder is {published}",
                    entry.resource_id
                )));
            }
            self.states[index] = entry.state;
        }
        Ok(())
    }

    fn persistent_snapshot(
        &self,
        registries: &SynchronizedRegistries,
    ) -> SaveResult<Java26WorldClocks> {
        Ok(Java26WorldClocks::new(
            self.states
                .iter()
                .copied()
                .enumerate()
                .map(|(index, state)| {
                    let index_i32 = i32::try_from(index).map_err(|_| {
                        clock_sequence_error("world-clock index does not fit the protocol")
                    })?;
                    let resource_id = registries
                        .registry_element_id(WORLD_CLOCK_REGISTRY, index_i32)
                        .ok_or_else(|| clock_sequence_error("world-clock index is not published"))?
                        .to_string();
                    Ok(Java26WorldClockEntry { resource_id, state })
                })
                .collect::<SaveResult<Vec<_>>>()?,
        ))
    }

    /// `ServerClockManager.ClockInstance.tick` (`ServerClockManager.java:161-167`).
    fn tick(&mut self) {
        for state in &mut self.states {
            if state.paused {
                continue;
            }
            state.partial_tick += state.rate;
            let full_ticks = state.partial_tick.floor() as i32;
            state.partial_tick -= full_ticks as f32;
            state.total_ticks = state.total_ticks.wrapping_add(i64::from(full_ticks));
        }
    }

    fn network_updates(&self, advance_time: bool) -> Vec<ClockUpdate> {
        (0..self.states.len())
            .map(|index| self.network_update(index, advance_time))
            .collect()
    }

    /// `ServerClockManager.ClockInstance.packNetworkState`
    /// (`ServerClockManager.java:174-177`).
    fn network_update(&self, index: usize, advance_time: bool) -> ClockUpdate {
        let state = self.states[index];
        ClockUpdate {
            clock_id: i32::try_from(index).expect("world clock count fits the protocol"),
            total_ticks: state.total_ticks,
            partial_tick: state.partial_tick,
            rate: if state.paused || !advance_time {
                0.0
            } else {
                state.rate
            },
        }
    }

    /// The four `ServerClockManager` writers `modifyClock` runs
    /// (`ServerClockManager.java:80-110`). `setTotalTicks` and
    /// `moveToTimeMarker` drop the fractional tick (`:83`, `:93`); `addTicks`
    /// keeps it and floors the sum at zero (`:101`).
    fn modify(&mut self, index: usize, change: CommandClockChange) {
        let Some(state) = self.states.get_mut(index) else {
            return;
        };
        match change {
            CommandClockChange::SetTotalTicks(total_ticks) => {
                state.total_ticks = total_ticks;
                state.partial_tick = 0.0;
            }
            CommandClockChange::AddTicks(ticks) => {
                state.total_ticks = state.total_ticks.wrapping_add(i64::from(ticks)).max(0);
            }
            CommandClockChange::SetPaused(paused) => state.paused = paused,
            CommandClockChange::SetRate(rate) => state.rate = rate,
        }
    }

    pub(super) fn total_ticks(&self, index: usize) -> Option<i64> {
        self.states.get(index).map(|state| state.total_ticks)
    }

    pub(super) fn total_ticks_for_clock(&self, clock: &str) -> Option<i64> {
        let index = self
            .catalog
            .clocks()
            .iter()
            .position(|entry| entry.resource_id == clock)?;
        self.total_ticks(index)
    }

    pub(super) fn move_to(&mut self, index: usize, total_ticks: i64) -> Result<()> {
        let state = self.states.get_mut(index).ok_or_else(|| {
            ServerWorldError::InvalidConfiguration(
                "world-clock index is outside the synchronized registry".to_string(),
            )
        })?;
        // `ServerClockManager.moveToTimeMarker` resets the fractional state
        // (`ServerClockManager.java:87-97`).
        state.total_ticks = total_ticks;
        state.partial_tick = 0.0;
        Ok(())
    }
}

/// The built-in catalog costs a datapack startup of its own, so the whole
/// process shares one, the way the sky-light layers do.
fn world_clock_catalog(startup: Option<&DatapackStartup>) -> Result<Arc<WorldClockCatalog>> {
    let Some(startup) = startup else {
        static BUILTIN: std::sync::OnceLock<std::result::Result<Arc<WorldClockCatalog>, String>> =
            std::sync::OnceLock::new();
        return BUILTIN
            .get_or_init(|| {
                let startup = DatapackStartup::load(&DatapackStartupOptions {
                    datapacks_dir: None,
                    saved_data_packs: None,
                    safe_mode: false,
                    limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(
                    ),
                })
                .map_err(|error| error.to_string())?;
                WorldClockCatalog::from_startup(&startup)
                    .map(Arc::new)
                    .map_err(|error| error.to_string())
            })
            .clone()
            .map_err(ServerWorldError::InvalidConfiguration);
    };
    WorldClockCatalog::from_startup(startup)
        .map(Arc::new)
        .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))
}

fn clock_sequence_error(detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: detail.into(),
    }
}

impl ServerWorld {
    // ClockTimeMarker.java:20-36; ServerLevel.java:354-358; ServerClockManager.java:87-122.
    pub(super) fn move_sleep_clock_to_marker(
        &mut self,
        dimension: &crate::world::LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(dimension_type) = self.cold.level_dimension_types.get(dimension) else {
            return;
        };
        let Some((clock, ticks, period)) = self
            .cold
            .dimension_sky_lights
            .sleep_clock_marker(dimension_type)
        else {
            return;
        };
        let Some(index) = self
            .registries
            .registry_element_index(WORLD_CLOCK_REGISTRY, clock)
            .and_then(|index| usize::try_from(index).ok())
        else {
            return;
        };
        let Some(now) = self.world_clock_total_ticks(index) else {
            return;
        };
        let next = if let Some(period) = period {
            let duration = i64::from(ticks) - (now % i64::from(period));
            now.wrapping_add(if duration > 0 {
                duration
            } else {
                i64::from(period) + duration
            })
        } else {
            i64::from(ticks)
        };
        self.world_clocks
            .move_to(index, next)
            .expect("registered clock");
        let advance = self.world_clocks_advance();
        let update = self.world_clocks.network_update(index, advance);
        let game_time = self.game_time();
        self.broadcast(
            ClientboundPlayPacket::SetTime(PlayTime {
                game_time,
                clock_updates: vec![update],
            }),
            events,
        );
    }

    pub(super) fn is_at_dimension_time_marker(
        &self,
        dimension: &crate::world::LevelId,
        marker: &str,
    ) -> Option<bool> {
        self.levels.get(dimension)?;
        let dimension_type = self.cold.level_dimension_types.get(dimension)?;
        self.cold
            .dimension_sky_lights
            .time_marker_matches(dimension_type, marker, |clock| {
                let index = self
                    .registries
                    .registry_element_index(WORLD_CLOCK_REGISTRY, clock)?;
                self.world_clock_total_ticks(usize::try_from(index).ok()?)
            })
    }

    fn world_clocks_advance(&self) -> bool {
        !matches!(
            self.admin.game_rule_value(ADVANCE_TIME_RULE),
            Some(GameRuleValue::Boolean(false))
        )
    }

    pub(super) fn tick_world_clocks(&mut self) {
        if self.world_clocks_advance() {
            self.world_clocks.tick();
        }
    }

    pub(super) fn world_clock_updates(&self) -> Vec<ClockUpdate> {
        self.world_clocks
            .network_updates(self.world_clocks_advance())
    }

    pub(super) fn restore_world_clocks(&mut self, saved: &Java26WorldClocks) -> SaveResult<()> {
        self.world_clocks.restore(&self.registries, saved)
    }

    pub(super) fn world_clock_snapshot(&self) -> SaveResult<Java26WorldClocks> {
        self.world_clocks.persistent_snapshot(&self.registries)
    }

    pub(super) fn world_clock_total_ticks(&self, index: usize) -> Option<i64> {
        self.world_clocks.total_ticks(index)
    }

    pub(crate) fn clock_catalog(&self) -> &WorldClockCatalog {
        &self.world_clocks.catalog
    }

    /// `ServerClockManager.getTotalTicks` (`ServerClockManager.java:125-127`).
    pub(crate) fn clock_total_ticks(&self, clock: &str) -> Option<i64> {
        self.world_clocks.total_ticks_for_clock(clock)
    }

    /// `ServerClockManager.modifyClock` (`ServerClockManager.java:112-122`):
    /// the write, then a `ClientboundSetTimePacket` carrying only the clock
    /// that moved. Vanilla also invalidates each level's environment-attribute
    /// tick cache (`:119-121`); this server samples those layers from the clock
    /// on every read and keeps no such cache.
    pub(crate) fn apply_clock_change(
        &mut self,
        clock: &str,
        change: CommandClockChange,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(index) = self
            .registries
            .registry_element_index(WORLD_CLOCK_REGISTRY, clock)
            .and_then(|index| usize::try_from(index).ok())
        else {
            return;
        };
        self.world_clocks.modify(index, change);
        let advance = self.world_clocks_advance();
        let update = self.world_clocks.network_update(index, advance);
        let game_time = self.game_time();
        self.broadcast(
            ClientboundPlayPacket::SetTime(PlayTime {
                game_time,
                clock_updates: vec![update],
            }),
            events,
        );
    }

    #[cfg(test)]
    pub(crate) fn world_clock_states_for_test(&self) -> Vec<Java26WorldClockState> {
        self.world_clocks.states.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_world_clock_state_for_test(
        &mut self,
        index: usize,
        state: Java26WorldClockState,
    ) {
        self.world_clocks.states[index] = state;
    }

    #[cfg(test)]
    pub(crate) fn set_advance_time_for_test(&mut self, advance: bool) {
        assert!(self
            .admin
            .set_game_rule(ADVANCE_TIME_RULE, GameRuleValue::Boolean(advance)));
    }

    #[cfg(test)]
    pub(crate) fn world_clock_updates_for_test(&self) -> Vec<ClockUpdate> {
        self.world_clock_updates()
    }

    #[cfg(test)]
    pub(crate) fn world_clock_snapshot_for_test(&self) -> Java26WorldClocks {
        self.world_clock_snapshot().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_noon_uses_java_remainder_and_wrapping_long_addition() {
        assert_eq!(strict_next_noon(0), 6_000);
        assert_eq!(strict_next_noon(1_234), 6_000);
        assert_eq!(strict_next_noon(6_000), 30_000);
        assert_eq!(strict_next_noon(7_000), 30_000);
        assert_eq!(strict_next_noon(-1), 6_000);
        assert_eq!(strict_next_noon(-18_000), 6_000);
    }

    fn state(total_ticks: i64, partial_tick: f32) -> ServerWorldClocks {
        ServerWorldClocks {
            states: vec![Java26WorldClockState {
                total_ticks,
                partial_tick,
                rate: 1.0,
                paused: false,
            }],
            catalog: Arc::default(),
        }
    }

    /// The four `ServerClockManager` writers (`ServerClockManager.java:80-110`):
    /// only the two absolute ones drop the fractional tick, and only `addTicks`
    /// has a floor.
    #[test]
    fn each_clock_writer_touches_only_what_its_vanilla_owner_does() {
        let mut clocks = state(40, 0.75);
        clocks.modify(0, CommandClockChange::SetTotalTicks(1_000));
        assert_eq!(clocks.states[0].total_ticks, 1_000);
        assert_eq!(clocks.states[0].partial_tick, 0.0);

        let mut clocks = state(40, 0.75);
        clocks.modify(0, CommandClockChange::AddTicks(10));
        assert_eq!(clocks.states[0].total_ticks, 50);
        assert_eq!(clocks.states[0].partial_tick, 0.75);

        let mut clocks = state(40, 0.75);
        clocks.modify(0, CommandClockChange::AddTicks(-500));
        assert_eq!(clocks.states[0].total_ticks, 0);
        assert_eq!(clocks.states[0].partial_tick, 0.75);

        let mut clocks = state(40, 0.75);
        clocks.modify(0, CommandClockChange::SetPaused(true));
        clocks.modify(0, CommandClockChange::SetRate(0.25));
        assert!(clocks.states[0].paused);
        assert_eq!(clocks.states[0].rate, 0.25);
        assert_eq!(clocks.states[0].total_ticks, 40);
        // A paused clock stands still and a rate below one advances in whole
        // ticks only (`ServerClockManager.java:161-167`).
        clocks.tick();
        assert_eq!(clocks.states[0].total_ticks, 40);
        clocks.modify(0, CommandClockChange::SetPaused(false));
        clocks.tick();
        assert_eq!(clocks.states[0].total_ticks, 41);
        assert_eq!(clocks.states[0].partial_tick, 0.0);
    }

    /// `packNetworkState` (`ServerClockManager.java:174-177`) reports rate zero
    /// for a clock that is paused or a world that is not advancing time.
    #[test]
    fn the_network_rate_is_zero_while_a_clock_is_stopped() {
        let mut clocks = state(40, 0.0);
        assert_eq!(clocks.network_update(0, true).rate, 1.0);
        assert_eq!(clocks.network_update(0, false).rate, 0.0);
        clocks.modify(0, CommandClockChange::SetPaused(true));
        assert_eq!(clocks.network_update(0, true).rate, 0.0);
    }
}
