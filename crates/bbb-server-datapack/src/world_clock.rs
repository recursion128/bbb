//! The `Registries.WORLD_CLOCK` and `Registries.TIMELINE` facts a clock manager
//! runs on: which clocks exist, which timeline drives each of them, and which
//! time markers each clock carries.

use bbb_protocol::NbtSummaryValue as Value;

use crate::startup::DatapackStartup;
use crate::vanilla_snapshot::VanillaDatapackSnapshot;

/// `Registries.WORLD_CLOCK`, whose elements `ServerClockManager.init` builds one
/// `ClockInstance` each for (`ServerClockManager.java:41-44`).
pub const WORLD_CLOCK_REGISTRY: &str = "minecraft:world_clock";

/// `Registries.TIMELINE`.
pub const TIMELINE_REGISTRY: &str = "minecraft:timeline";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClockCatalogError(pub String);

impl std::fmt::Display for WorldClockCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for WorldClockCatalogError {}

type Result<T> = std::result::Result<T, WorldClockCatalogError>;

fn error(message: impl Into<String>) -> WorldClockCatalogError {
    WorldClockCatalogError(message.into())
}

/// One `ClockTimeMarker` (`ClockTimeMarker.java:8`) as
/// `Timeline.registerTimeMarkers` builds it (`Timeline.java:138-143`): the
/// declaring timeline's own `period_ticks` travels onto every marker it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockTimeMarker {
    pub resource_id: String,
    pub ticks: i32,
    pub period_ticks: Option<i32>,
    pub show_in_commands: bool,
}

impl ClockTimeMarker {
    /// `ClockTimeMarker.resolveTimeToMoveTo` (`ClockTimeMarker.java:20-36`).
    #[must_use]
    pub fn resolve_time_to_move_to(&self, total_ticks: i64) -> i64 {
        let Some(period) = self.period_ticks else {
            return i64::from(self.ticks);
        };
        let period = i64::from(period);
        let duration = i64::from(self.ticks) - total_ticks % period;
        let duration = if duration > 0 {
            duration
        } else {
            period + duration
        };
        total_ticks.wrapping_add(duration)
    }
}

/// One `WorldClock` registry element with the markers registered onto it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClockDefinition {
    pub resource_id: String,
    /// Sorted by id: vanilla files them in a `Reference2ObjectOpenHashMap`
    /// (`ServerClockManager.java:148`), whose order is an identity hash.
    pub time_markers: Vec<ClockTimeMarker>,
}

impl WorldClockDefinition {
    #[must_use]
    pub fn time_marker(&self, resource_id: &str) -> Option<&ClockTimeMarker> {
        self.time_markers
            .iter()
            .find(|marker| marker.resource_id == resource_id)
    }
}

/// One `Timeline` registry element reduced to what a clock reader needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineDefinition {
    pub resource_id: String,
    pub clock: String,
    pub period_ticks: Option<i32>,
}

impl TimelineDefinition {
    /// `Timeline.getCurrentTicks` (`Timeline.java:121-124`).
    #[must_use]
    pub fn current_ticks(&self, total_ticks: i64) -> i64 {
        self.period_ticks
            .map_or(total_ticks, |period| total_ticks % i64::from(period))
    }

    /// `Timeline.getPeriodCount` (`Timeline.java:112-119`), which narrows the
    /// quotient to `int`.
    #[must_use]
    pub fn period_count(&self, total_ticks: i64) -> i32 {
        self.period_ticks
            .map_or(0, |period| (total_ticks / i64::from(period)) as i32)
    }
}

/// The clock and timeline registries of one datapack generation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldClockCatalog {
    clocks: Vec<WorldClockDefinition>,
    timelines: Vec<TimelineDefinition>,
}

impl WorldClockCatalog {
    /// `ServerClockManager.init` (`ServerClockManager.java:39-53`): an instance
    /// per world clock, then every timeline hands its markers to the clock it
    /// names.
    pub fn from_startup(startup: &DatapackStartup) -> Result<Self> {
        let clocks = startup
            .registries
            .registry(WORLD_CLOCK_REGISTRY)
            .ok_or_else(|| error("missing world_clock registry"))?;
        let timelines = startup
            .registries
            .registry(TIMELINE_REGISTRY)
            .ok_or_else(|| error("missing timeline registry"))?;
        let timeline_values = timelines
            .elements
            .iter()
            .map(|element| {
                let retained = element.codec_value.as_ref().ok_or_else(|| {
                    error(format!(
                        "timeline {} retained codec value is unavailable",
                        element.id
                    ))
                })?;
                Ok((element.id.clone(), &retained.network_nbt))
            })
            .collect::<Result<Vec<_>>>()?;
        Self::build(
            clocks.elements.iter().map(|element| element.id.clone()),
            timeline_values,
        )
    }

    fn build<'a>(
        clock_ids: impl IntoIterator<Item = String>,
        timelines: impl IntoIterator<Item = (String, &'a Value)>,
    ) -> Result<Self> {
        let mut clocks = clock_ids
            .into_iter()
            .map(|resource_id| WorldClockDefinition {
                resource_id,
                time_markers: Vec::new(),
            })
            .collect::<Vec<_>>();
        let mut definitions = Vec::new();
        for (resource_id, value) in timelines {
            let Value::String(clock) = field(value, "clock").unwrap() else {
                unreachable!("the exact timeline codec projects a clock string");
            };
            let clock = clock.try_to_string().unwrap();
            let period_ticks = field(value, "period_ticks").map(int);
            let definition = clocks
                .iter_mut()
                .find(|definition| definition.resource_id == clock)
                .ok_or_else(|| {
                    error(format!(
                        "timeline {resource_id} names unknown clock {clock}"
                    ))
                })?;
            read_time_markers(value, period_ticks, definition)?;
            definitions.push(TimelineDefinition {
                resource_id,
                clock,
                period_ticks,
            });
        }
        for clock in &mut clocks {
            clock
                .time_markers
                .sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
        }
        Ok(Self {
            clocks,
            timelines: definitions,
        })
    }

    #[must_use]
    pub fn clocks(&self) -> &[WorldClockDefinition] {
        &self.clocks
    }

    #[must_use]
    pub fn clock(&self, resource_id: &str) -> Option<&WorldClockDefinition> {
        self.clocks
            .iter()
            .find(|clock| clock.resource_id == resource_id)
    }

    #[must_use]
    pub fn timelines(&self) -> &[TimelineDefinition] {
        &self.timelines
    }

    #[must_use]
    pub fn timeline(&self, resource_id: &str) -> Option<&TimelineDefinition> {
        self.timelines
            .iter()
            .find(|timeline| timeline.resource_id == resource_id)
    }
}

/// The clock registries of the embedded vanilla pack, for a reader that holds
/// no loaded generation.
pub fn builtin_world_clock_catalog() -> &'static WorldClockCatalog {
    static BUILTIN: std::sync::OnceLock<WorldClockCatalog> = std::sync::OnceLock::new();
    BUILTIN.get_or_init(|| {
        let snapshot = VanillaDatapackSnapshot::embedded_shared()
            .expect("the embedded built-in datapack decodes");
        let clock_ids = builtin_element_ids(&snapshot, "world_clock/");
        let timelines = builtin_element_ids(&snapshot, "timeline/")
            .into_iter()
            .map(|id| {
                let path = format!("timeline/{}.json", id.trim_start_matches("minecraft:"));
                let bytes = snapshot
                    .read_path(&path)
                    .expect("the built-in pack holds each timeline it lists");
                let source = bbb_registry_codec::decode_direct_registry_json(bytes)
                    .expect("the built-in timeline source decodes");
                let value = bbb_registry_codec::decode_exact_synchronized_registry_element(
                    TIMELINE_REGISTRY,
                    &source,
                )
                .expect("the built-in timeline validates")
                .projected_network_nbt()
                .expect("the built-in timeline projects");
                (id, value)
            })
            .collect::<Vec<_>>();
        WorldClockCatalog::build(
            clock_ids,
            timelines.iter().map(|(id, value)| (id.clone(), value)),
        )
        .expect("the built-in clock registries are consistent")
    })
}

fn builtin_element_ids(snapshot: &VanillaDatapackSnapshot, prefix: &str) -> Vec<String> {
    let mut paths = snapshot
        .paths()
        .filter(|path| path.starts_with(prefix) && path.ends_with(".json"))
        .collect::<Vec<_>>();
    paths.sort_unstable();
    paths
        .into_iter()
        .map(|path| {
            format!(
                "minecraft:{}",
                path.trim_start_matches(prefix).trim_end_matches(".json")
            )
        })
        .collect()
}

fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    let Value::Compound(entries) = value else {
        unreachable!("the exact timeline codec projects a compound");
    };
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn int(value: &Value) -> i32 {
    let Value::Int(value) = value else {
        unreachable!("the exact timeline codec projects integer ticks");
    };
    *value
}

/// `Timeline.TimeMarkerInfo.CODEC` (`Timeline.java:204-215`): a bare
/// non-negative tick count, or the two-field form. `Timeline.validateRegistry`
/// (`Timeline.java:72-83`) refuses the same marker twice on one clock.
fn read_time_markers(
    value: &Value,
    period_ticks: Option<i32>,
    clock: &mut WorldClockDefinition,
) -> Result<()> {
    let Some(value) = field(value, "time_markers") else {
        return Ok(());
    };
    let Value::Compound(entries) = value else {
        unreachable!("the exact timeline codec projects a marker compound");
    };
    for entry in entries {
        let resource_id = entry.name.try_to_string().unwrap();
        let (ticks, show_in_commands) = match &entry.value {
            Value::Int(ticks) => (*ticks, false),
            info @ Value::Compound(_) => (int(field(info, "ticks").unwrap()), true),
            _ => unreachable!("the exact timeline codec projects integer or visible markers"),
        };
        if clock.time_marker(&resource_id).is_some() {
            return Err(error(format!(
                "time marker {resource_id} was defined multiple times in {}",
                clock.resource_id
            )));
        }
        clock.time_markers.push(ClockTimeMarker {
            resource_id,
            ticks,
            period_ticks,
            show_in_commands,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overworld() -> &'static WorldClockDefinition {
        builtin_world_clock_catalog()
            .clock("minecraft:overworld")
            .unwrap()
    }

    /// `WorldClocks.bootstrap` (`WorldClocks.java:12-15`) and the four built-in
    /// timelines, every one of which drives the overworld clock.
    #[test]
    fn the_builtin_catalog_holds_the_two_clocks_and_their_timelines() {
        let catalog = builtin_world_clock_catalog();
        assert_eq!(
            catalog
                .clocks()
                .iter()
                .map(|clock| clock.resource_id.as_str())
                .collect::<Vec<_>>(),
            ["minecraft:overworld", "minecraft:the_end"]
        );
        assert_eq!(
            catalog
                .timelines()
                .iter()
                .map(|timeline| (
                    timeline.resource_id.as_str(),
                    timeline.clock.as_str(),
                    timeline.period_ticks
                ))
                .collect::<Vec<_>>(),
            [
                ("minecraft:day", "minecraft:overworld", Some(24_000)),
                ("minecraft:early_game", "minecraft:overworld", None),
                ("minecraft:moon", "minecraft:overworld", Some(192_000)),
                (
                    "minecraft:villager_schedule",
                    "minecraft:overworld",
                    Some(24_000)
                ),
            ]
        );
        assert!(catalog
            .clock("minecraft:the_end")
            .unwrap()
            .time_markers
            .is_empty());
    }

    /// `TimeMarkerInfo.CODEC` (`Timeline.java:204-215`) reads both shapes, and
    /// `registerTimeMarkers` (`:138-143`) hands each marker the declaring
    /// timeline's period.
    #[test]
    fn a_time_marker_carries_its_timelines_period_and_its_own_visibility() {
        assert_eq!(
            overworld().time_marker("minecraft:noon").unwrap(),
            &ClockTimeMarker {
                resource_id: "minecraft:noon".to_string(),
                ticks: 6_000,
                period_ticks: Some(24_000),
                show_in_commands: true,
            }
        );
        // The bare-int form defaults `show_in_commands` to false (`:207`).
        assert_eq!(
            overworld()
                .time_marker("minecraft:roll_village_siege")
                .unwrap(),
            &ClockTimeMarker {
                resource_id: "minecraft:roll_village_siege".to_string(),
                ticks: 18_000,
                period_ticks: Some(24_000),
                show_in_commands: false,
            }
        );
    }

    #[test]
    fn canonical_source_values_reach_clock_markers() {
        let source = bbb_registry_codec::decode_direct_registry_json(
            br#"{"clock":"overworld","period_ticks":24000.0,"time_markers":{"noon":6000.0}}"#,
        )
        .unwrap();
        let value = bbb_registry_codec::decode_exact_synchronized_registry_element(
            TIMELINE_REGISTRY,
            &source,
        )
        .unwrap()
        .projected_network_nbt()
        .unwrap();
        let catalog = WorldClockCatalog::build(
            ["minecraft:overworld".to_string()],
            [("example:day".to_string(), &value)],
        )
        .unwrap();
        assert_eq!(
            catalog.clock("minecraft:overworld").unwrap().time_markers,
            [ClockTimeMarker {
                resource_id: "minecraft:noon".to_string(),
                ticks: 6_000,
                period_ticks: Some(24_000),
                show_in_commands: false,
            }]
        );
    }

    /// `ClockTimeMarker.resolveTimeToMoveTo` (`ClockTimeMarker.java:20-36`),
    /// whose periodic branch always moves forward.
    #[test]
    fn resolving_a_marker_never_stands_still() {
        let noon = overworld().time_marker("minecraft:noon").unwrap();
        assert_eq!(noon.resolve_time_to_move_to(0), 6_000);
        assert_eq!(noon.resolve_time_to_move_to(5_999), 6_000);
        assert_eq!(noon.resolve_time_to_move_to(6_000), 30_000);
        assert_eq!(noon.resolve_time_to_move_to(30_001), 54_000);
        // `total + duration` is a long addition that wraps (`:26`).
        assert_eq!(noon.resolve_time_to_move_to(i64::MAX), i64::MIN + 22_192);
        // A marker with no period names one absolute tick (`:21-23`).
        let absolute = ClockTimeMarker {
            resource_id: "example:once".to_string(),
            ticks: 40,
            period_ticks: None,
            show_in_commands: false,
        };
        assert_eq!(absolute.resolve_time_to_move_to(1_000), 40);
    }

    /// `Timeline.getCurrentTicks` (`Timeline.java:121-124`) and
    /// `getPeriodCount` (`:112-119`).
    #[test]
    fn a_timeline_without_a_period_reports_the_whole_clock_and_no_repetitions() {
        let catalog = builtin_world_clock_catalog();
        let day = catalog.timeline("minecraft:day").unwrap();
        assert_eq!(day.current_ticks(50_000), 2_000);
        assert_eq!(day.period_count(50_000), 2);
        let early_game = catalog.timeline("minecraft:early_game").unwrap();
        assert_eq!(early_game.current_ticks(50_000), 50_000);
        assert_eq!(early_game.period_count(50_000), 0);
    }
}
