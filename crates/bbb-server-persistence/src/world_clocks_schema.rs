use std::collections::BTreeMap;

use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::save::{
    NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

use crate::schema_support::{
    bool_or, compound_or_empty, field, float_or, long_value, require_java_26_1_root,
};

const SCHEMA: &str = "world_clocks";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Java26WorldClockState {
    pub total_ticks: i64,
    pub partial_tick: f32,
    pub rate: f32,
    pub paused: bool,
}

impl Default for Java26WorldClockState {
    fn default() -> Self {
        Self {
            total_ticks: 0,
            partial_tick: 0.0,
            rate: 1.0,
            paused: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26WorldClockEntry {
    pub resource_id: String,
    pub state: Java26WorldClockState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26WorldClocks {
    entries: Vec<Java26WorldClockEntry>,
}

impl Java26WorldClocks {
    pub fn new(entries: Vec<Java26WorldClockEntry>) -> Self {
        Self { entries }
    }

    pub fn defaults(live_holder_ids: &[String]) -> Self {
        Self::new(
            live_holder_ids
                .iter()
                .map(|resource_id| Java26WorldClockEntry {
                    resource_id: resource_id.clone(),
                    state: Java26WorldClockState::default(),
                })
                .collect(),
        )
    }

    pub fn entries(&self) -> &[Java26WorldClockEntry] {
        &self.entries
    }
}

/// `PackedClockStates.CODEC` (`PackedClockStates.java:7-11`) is decoded in the
/// live world-clock registry context installed before the manager is created.
pub fn load_java_26_1_world_clocks(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
    live_holder_ids: &[String],
) -> SaveResult<Java26WorldClocks> {
    let defaults = Java26WorldClocks::defaults(live_holder_ids);
    let path = paths.world_clocks_dat();
    let Some((document, _)) = crate::level_file::read_optional_gzip_compound(&path, limits)? else {
        return Ok(defaults);
    };
    let root = require_java_26_1_root(&document, SCHEMA)?;
    let mut saved = BTreeMap::new();
    for entry in compound_or_empty(root, "data") {
        let Some(resource_id) = entry
            .name
            .try_to_string()
            .ok()
            .and_then(|raw| normalize_resource_location(raw).ok())
        else {
            continue;
        };
        let NbtSummaryValue::Compound(fields) = &entry.value else {
            continue;
        };
        // `ClockState.CODEC` (`ClockState.java:8-16`).
        let Some(total_ticks) = field(fields, "total_ticks").and_then(long_value) else {
            continue;
        };
        let partial_tick = float_or(fields, "partial_tick", 0.0);
        let rate = float_or(fields, "rate", 1.0);
        saved.entry(resource_id).or_insert(Java26WorldClockState {
            total_ticks,
            partial_tick: if partial_tick.is_finite() {
                partial_tick
            } else {
                0.0
            },
            rate: if rate > 0.0 && rate.is_finite() {
                rate
            } else {
                1.0
            },
            paused: bool_or(fields, "paused", false),
        });
    }
    // Clocks absent from the live registry are dropped.
    Ok(Java26WorldClocks::new(
        defaults
            .entries()
            .iter()
            .map(|entry| Java26WorldClockEntry {
                resource_id: entry.resource_id.clone(),
                state: saved.get(&entry.resource_id).copied().unwrap_or_default(),
            })
            .collect(),
    ))
}

/// `SavedDataStorage.encodeUnchecked` (`SavedDataStorage.java:198-204`) puts
/// every live clock's packed state below the root `data` field.
pub fn java_26_1_world_clocks_document(clocks: &Java26WorldClocks) -> NamedCompound {
    let data = clocks
        .entries()
        .iter()
        .map(|entry| {
            let state = entry.state;
            let mut fields = vec![NbtSummaryEntry {
                name: "total_ticks".into(),
                value: NbtSummaryValue::Long(state.total_ticks),
            }];
            if state.partial_tick.to_bits() != 0.0_f32.to_bits() {
                fields.push(NbtSummaryEntry {
                    name: "partial_tick".into(),
                    value: NbtSummaryValue::Float(state.partial_tick.to_bits()),
                });
            }
            if state.rate.to_bits() != 1.0_f32.to_bits() {
                fields.push(NbtSummaryEntry {
                    name: "rate".into(),
                    value: NbtSummaryValue::Float(state.rate.to_bits()),
                });
            }
            if state.paused {
                fields.push(NbtSummaryEntry {
                    name: "paused".into(),
                    value: NbtSummaryValue::Byte(1),
                });
            }
            NbtSummaryEntry {
                name: entry.resource_id.clone().into(),
                value: NbtSummaryValue::Compound(fields),
            }
        })
        .collect();
    NamedCompound::new(
        "",
        vec![
            NbtSummaryEntry {
                name: "DataVersion".into(),
                value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
            },
            NbtSummaryEntry {
                name: "data".into(),
                value: NbtSummaryValue::Compound(data),
            },
        ],
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::level_file::encode_gzip_named_compound;

    use super::*;

    fn holders() -> Vec<String> {
        ["minecraft:overworld", "example:clock", "minecraft:the_end"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn scratch(name: &str) -> WorldSavePaths {
        let root = std::env::temp_dir().join(format!(
            "bbb-client-world-clocks-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let paths = WorldSavePaths::new(root).unwrap();
        fs::create_dir_all(paths.saved_data_dir().join("minecraft")).unwrap();
        paths
    }

    fn write(paths: &WorldSavePaths, document: &NamedCompound) {
        let limits = WorldSaveLimits::default();
        fs::write(
            paths.world_clocks_dat(),
            encode_gzip_named_compound(&paths.world_clocks_dat(), document, limits).unwrap(),
        )
        .unwrap();
    }

    fn data(document: &NamedCompound) -> &[NbtSummaryEntry] {
        let NbtSummaryValue::Compound(data) = &document
            .entries()
            .iter()
            .find(|entry| entry.name == "data")
            .unwrap()
            .value
        else {
            panic!("data must be a compound")
        };
        data
    }

    fn document(entries: Vec<NbtSummaryEntry>) -> NamedCompound {
        NamedCompound::new(
            "",
            vec![
                NbtSummaryEntry {
                    name: "DataVersion".into(),
                    value: NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION),
                },
                NbtSummaryEntry {
                    name: "data".into(),
                    value: NbtSummaryValue::Compound(entries),
                },
            ],
        )
    }

    fn clock(name: &str, fields: Vec<NbtSummaryEntry>) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Compound(fields),
        }
    }

    fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn a_missing_file_materializes_every_live_holder_in_order() {
        let paths = scratch("missing");
        let loaded =
            load_java_26_1_world_clocks(&paths, WorldSaveLimits::default(), &holders()).unwrap();
        assert_eq!(
            loaded
                .entries()
                .iter()
                .map(|entry| entry.resource_id.as_str())
                .collect::<Vec<_>>(),
            ["minecraft:overworld", "example:clock", "minecraft:the_end"]
        );
        assert!(loaded
            .entries()
            .iter()
            .all(|entry| entry.state == Java26WorldClockState::default()));
    }

    #[test]
    fn shuffled_saved_entries_project_to_live_holder_order_and_fill_missing_keys() {
        let paths = scratch("shuffled");
        write(
            &paths,
            &document(vec![
                clock(
                    "minecraft:the_end",
                    vec![field("total_ticks", NbtSummaryValue::Long(31))],
                ),
                clock(
                    "minecraft:overworld",
                    vec![field("total_ticks", NbtSummaryValue::Long(17))],
                ),
            ]),
        );
        let loaded =
            load_java_26_1_world_clocks(&paths, WorldSaveLimits::default(), &holders()).unwrap();
        assert_eq!(
            loaded
                .entries()
                .iter()
                .map(|entry| entry.state.total_ticks)
                .collect::<Vec<_>>(),
            [17, 0, 31]
        );
    }

    #[test]
    fn all_four_fields_round_trip_and_defaults_are_omitted() {
        let clocks = Java26WorldClocks::new(vec![
            Java26WorldClockEntry {
                resource_id: "minecraft:overworld".to_string(),
                state: Java26WorldClockState {
                    total_ticks: -4,
                    partial_tick: 0.25,
                    rate: 2.5,
                    paused: true,
                },
            },
            Java26WorldClockEntry {
                resource_id: "minecraft:the_end".to_string(),
                state: Java26WorldClockState::default(),
            },
        ]);
        let document = java_26_1_world_clocks_document(&clocks);
        let NbtSummaryValue::Compound(default_fields) = &data(&document)[1].value else {
            panic!("clock must be a compound")
        };
        assert_eq!(
            default_fields
                .iter()
                .map(|field| &field.name)
                .collect::<Vec<_>>(),
            ["total_ticks"]
        );
        let paths = scratch("round-trip");
        write(&paths, &document);
        assert_eq!(
            load_java_26_1_world_clocks(
                &paths,
                WorldSaveLimits::default(),
                &[
                    "minecraft:overworld".to_string(),
                    "minecraft:the_end".to_string()
                ]
            )
            .unwrap(),
            clocks
        );
    }

    #[test]
    fn non_finite_partial_tick_sanitizes_to_zero() {
        let clocks = Java26WorldClocks::new(vec![Java26WorldClockEntry {
            resource_id: "minecraft:overworld".to_string(),
            state: Java26WorldClockState {
                total_ticks: 10,
                partial_tick: f32::NAN,
                rate: 1.0,
                paused: false,
            },
        }]);
        let document = java_26_1_world_clocks_document(&clocks);
        let paths = scratch("nan-partial-tick");
        write(&paths, &document);
        let loaded = load_java_26_1_world_clocks(
            &paths,
            WorldSaveLimits::default(),
            &["minecraft:overworld".to_string()],
        )
        .unwrap();
        assert_eq!(loaded.entries()[0].state.partial_tick, 0.0);
    }
}
