//! `MinecraftServer.getStopwatches()` (Stopwatches.java:16-86).

use std::collections::BTreeMap;

use super::idle_timeout::monotonic_millis;
use super::ServerWorld;

/// `Stopwatch` (Stopwatch.java:3-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stopwatch {
    pub creation_time: i64,
    pub accumulated_elapsed_time: i64,
}

impl Stopwatch {
    fn created_now() -> Self {
        Self {
            creation_time: monotonic_millis(),
            accumulated_elapsed_time: 0,
        }
    }

    /// `Stopwatch.elapsedMilliseconds` (Stopwatch.java:8-11).
    pub(crate) fn elapsed_milliseconds(&self, now: i64) -> i64 {
        self.accumulated_elapsed_time + (now - self.creation_time)
    }

    /// `Stopwatch.elapsedSeconds` (Stopwatch.java:13-15).
    pub(crate) fn elapsed_seconds(&self, now: i64) -> f64 {
        self.elapsed_milliseconds(now) as f64 / 1000.0
    }
}

/// `Stopwatches` (Stopwatches.java:16-86).
#[derive(Clone, Debug, Default)]
pub(crate) struct Stopwatches {
    map: BTreeMap<String, Stopwatch>,
}

impl Stopwatches {
    /// `Stopwatches.add` (Stopwatches.java:47-54): `putIfAbsent`.
    pub(crate) fn add(&mut self, id: String, stopwatch: Stopwatch) -> bool {
        use std::collections::btree_map::Entry;
        match self.map.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(stopwatch);
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    pub(crate) fn restart(&mut self, id: &str) -> bool {
        if let Some(entry) = self.map.get_mut(id) {
            *entry = Stopwatch::created_now();
            true
        } else {
            false
        }
    }

    pub(crate) fn remove(&mut self, id: &str) -> bool {
        self.map.remove(id).is_some()
    }

    /// `Stopwatches.unpack` (`Stopwatches.java:29-34`): elapsed time keeps
    /// accumulating from load.
    pub(crate) fn unpack(saved: &BTreeMap<String, i64>) -> Self {
        let now = monotonic_millis();
        Self {
            map: saved
                .iter()
                .map(|(id, elapsed)| {
                    (
                        id.clone(),
                        Stopwatch {
                            creation_time: now,
                            accumulated_elapsed_time: *elapsed,
                        },
                    )
                })
                .collect(),
        }
    }

    /// `Stopwatches.pack` (`Stopwatches.java:36-41`).
    pub(crate) fn pack(&self) -> BTreeMap<String, i64> {
        let now = monotonic_millis();
        self.map
            .iter()
            .map(|(id, stopwatch)| (id.clone(), stopwatch.elapsed_milliseconds(now)))
            .collect()
    }
}

impl ServerWorld {
    /// `Stopwatches.add` with a stopwatch started now; `false` is an id
    /// already taken.
    pub(crate) fn create_stopwatch(&mut self, id: String) -> bool {
        self.cold.stopwatches.add(id, Stopwatch::created_now())
    }

    pub(crate) fn restart_stopwatch(&mut self, id: &str) -> bool {
        self.cold.stopwatches.restart(id)
    }

    pub(crate) fn remove_stopwatch(&mut self, id: &str) -> bool {
        self.cold.stopwatches.remove(id)
    }

    pub(crate) fn stopwatch(&self, id: &str) -> Option<&Stopwatch> {
        self.cold.stopwatches.map.get(id)
    }

    pub(crate) fn stopwatch_ids(&self) -> impl Iterator<Item = &str> {
        self.cold.stopwatches.map.keys().map(String::as_str)
    }
}
