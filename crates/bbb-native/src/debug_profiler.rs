use std::{collections::BTreeMap, time::Duration};

use bbb_renderer::{HudDebugProfilerChart, HudDebugProfilerSlice};

const ROOT_PATH: &str = "root";
const UNSPECIFIED: &str = "unspecified";
const PATH_SEPARATOR: char = '\u{001e}';

#[derive(Debug, Clone, Copy)]
pub(crate) struct DebugProfilerSectionSample {
    pub(crate) name: &'static str,
    pub(crate) duration: Duration,
}

#[derive(Debug, Default)]
pub(crate) struct DebugProfiler {
    entries: BTreeMap<String, DebugProfilerEntry>,
    path: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct DebugProfilerEntry {
    duration_nanos: u64,
}

#[derive(Debug, Clone, PartialEq)]
struct DebugProfilerResultField {
    name: String,
    percentage: f64,
    global_percentage: f64,
}

impl DebugProfiler {
    pub(crate) fn record_frame(
        &mut self,
        total: Duration,
        sections: &[DebugProfilerSectionSample],
    ) {
        self.entries.clear();
        self.entries.insert(
            ROOT_PATH.to_string(),
            DebugProfilerEntry {
                duration_nanos: duration_nanos(total),
            },
        );
        for section in sections {
            if section.name.is_empty() || section.name.contains(PATH_SEPARATOR) {
                continue;
            }
            self.entries.insert(
                profiler_path(ROOT_PATH, section.name),
                DebugProfilerEntry {
                    duration_nanos: duration_nanos(section.duration),
                },
            );
        }
        if self.path.is_empty() || !self.entries.contains_key(&self.path) {
            self.path = ROOT_PATH.to_string();
        }
    }

    pub(crate) fn chart(&self) -> Option<HudDebugProfilerChart> {
        if self.entries.is_empty() {
            return None;
        }
        let mut times = self.get_times(&self.path);
        if times.is_empty() {
            return None;
        }
        let current = times.remove(0);
        Some(HudDebugProfilerChart {
            current_node_name: current.name,
            current_global_percentage: current.global_percentage,
            slices: times
                .into_iter()
                .map(|field| HudDebugProfilerSlice {
                    name: field.name,
                    percentage: field.percentage,
                    global_percentage: field.global_percentage,
                })
                .collect(),
        })
    }

    pub(crate) fn navigate(&mut self, digit: u8) {
        if self.entries.is_empty() {
            return;
        }
        let mut times = self.get_times(&self.path);
        if times.is_empty() {
            return;
        }
        let current = times.remove(0);
        if digit == 0 {
            if !current.name.is_empty() {
                if let Some(pos) = self.path.rfind(PATH_SEPARATOR) {
                    self.path.truncate(pos);
                }
            }
            return;
        }

        let child_index = usize::from(digit.saturating_sub(1));
        let Some(child) = times.get(child_index) else {
            return;
        };
        if child.name == UNSPECIFIED {
            return;
        }
        if !self.path.is_empty() {
            self.path.push(PATH_SEPARATOR);
        }
        self.path.push_str(&child.name);
    }

    pub(crate) fn current_path(&self) -> &str {
        if self.path.is_empty() {
            ROOT_PATH
        } else {
            &self.path
        }
    }

    fn get_times(&self, raw_path: &str) -> Vec<DebugProfilerResultField> {
        let root_entry = self.entry(ROOT_PATH);
        let current_entry = self.entry(raw_path);
        let mut global_time = root_entry.duration_nanos.max(1);
        let self_time = current_entry.duration_nanos;
        let prefix = profiler_child_prefix(raw_path);
        let mut total_time = self
            .entries
            .keys()
            .filter(|key| is_direct_child(&prefix, key))
            .map(|key| self.entry(key).duration_nanos)
            .sum::<u64>();
        let old_time = total_time;
        total_time = total_time.max(self_time).max(1);
        global_time = global_time.max(total_time);

        let mut result = self
            .entries
            .iter()
            .filter(|(key, _)| is_direct_child(&prefix, key))
            .map(|(key, entry)| {
                let time = entry.duration_nanos;
                DebugProfilerResultField {
                    name: key[prefix.len()..].to_string(),
                    percentage: time as f64 * 100.0 / total_time as f64,
                    global_percentage: time as f64 * 100.0 / global_time as f64,
                }
            })
            .collect::<Vec<_>>();

        if total_time > old_time {
            let unspecified = total_time - old_time;
            result.push(DebugProfilerResultField {
                name: UNSPECIFIED.to_string(),
                percentage: unspecified as f64 * 100.0 / total_time as f64,
                global_percentage: unspecified as f64 * 100.0 / global_time as f64,
            });
        }

        result.sort_by(|left, right| {
            right
                .percentage
                .partial_cmp(&left.percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.name.cmp(&left.name))
        });
        result.insert(
            0,
            DebugProfilerResultField {
                name: raw_path.to_string(),
                percentage: 100.0,
                global_percentage: total_time as f64 * 100.0 / global_time as f64,
            },
        );
        result
    }

    fn entry(&self, path: &str) -> DebugProfilerEntry {
        self.entries.get(path).copied().unwrap_or_default()
    }
}

fn profiler_path(parent: &str, child: &str) -> String {
    format!("{parent}{PATH_SEPARATOR}{child}")
}

fn profiler_child_prefix(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!("{path}{PATH_SEPARATOR}")
    }
}

fn is_direct_child(prefix: &str, path: &str) -> bool {
    path.len() > prefix.len()
        && path.starts_with(prefix)
        && !path[prefix.len()..].contains(PATH_SEPARATOR)
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_uses_vanilla_profile_results_percentages_and_unspecified_slice() {
        let mut profiler = DebugProfiler::default();
        profiler.record_frame(
            Duration::from_nanos(100),
            &[
                DebugProfilerSectionSample {
                    name: "update",
                    duration: Duration::from_nanos(30),
                },
                DebugProfilerSectionSample {
                    name: "render",
                    duration: Duration::from_nanos(50),
                },
                DebugProfilerSectionSample {
                    name: "publish",
                    duration: Duration::from_nanos(10),
                },
            ],
        );

        let chart = profiler.chart().expect("profiler chart");
        assert_eq!(chart.current_node_name, ROOT_PATH);
        assert_eq!(chart.current_global_percentage, 100.0);
        assert_eq!(
            chart
                .slices
                .iter()
                .map(|slice| (
                    slice.name.as_str(),
                    slice.percentage.round() as i32,
                    slice.global_percentage.round() as i32
                ))
                .collect::<Vec<_>>(),
            vec![
                ("render", 50, 50),
                ("update", 30, 30),
                ("unspecified", 10, 10),
                ("publish", 10, 10),
            ]
        );
    }

    #[test]
    fn navigation_matches_vanilla_zero_parent_and_ignores_unspecified() {
        let mut profiler = DebugProfiler::default();
        profiler.record_frame(
            Duration::from_nanos(100),
            &[
                DebugProfilerSectionSample {
                    name: "update",
                    duration: Duration::from_nanos(30),
                },
                DebugProfilerSectionSample {
                    name: "render",
                    duration: Duration::from_nanos(50),
                },
            ],
        );

        profiler.navigate(1);
        assert_eq!(profiler.current_path(), "root\u{001e}render");
        let chart = profiler.chart().expect("child chart");
        assert_eq!(chart.current_node_name, "root\u{001e}render");
        assert_eq!(chart.current_global_percentage, 50.0);
        assert_eq!(chart.slices.len(), 1);
        assert_eq!(chart.slices[0].name, UNSPECIFIED);

        profiler.navigate(1);
        assert_eq!(profiler.current_path(), "root\u{001e}render");
        profiler.navigate(0);
        assert_eq!(profiler.current_path(), ROOT_PATH);
        profiler.navigate(0);
        assert_eq!(profiler.current_path(), ROOT_PATH);
    }

    #[test]
    fn stale_navigation_path_resets_when_new_results_do_not_have_that_node() {
        let mut profiler = DebugProfiler::default();
        profiler.record_frame(
            Duration::from_nanos(50),
            &[DebugProfilerSectionSample {
                name: "render",
                duration: Duration::from_nanos(50),
            }],
        );
        profiler.navigate(1);
        assert_eq!(profiler.current_path(), "root\u{001e}render");

        profiler.record_frame(
            Duration::from_nanos(50),
            &[DebugProfilerSectionSample {
                name: "update",
                duration: Duration::from_nanos(50),
            }],
        );
        assert_eq!(profiler.current_path(), ROOT_PATH);
    }
}
