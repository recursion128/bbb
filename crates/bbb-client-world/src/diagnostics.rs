use serde::{Deserialize, Serialize};

use crate::WorldStore;

const MAX_WORLD_APPLY_ERRORS: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldApplyDiagnosticsState {
    #[serde(default)]
    pub apply_errors: Vec<WorldApplyErrorState>,
    #[serde(default)]
    pub last_apply_error: Option<WorldApplyErrorState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldApplyErrorState {
    pub source: String,
    pub message: String,
}

impl WorldStore {
    pub fn record_apply_error(
        &mut self,
        source: impl Into<String>,
        err: impl std::fmt::Display,
    ) -> WorldApplyErrorState {
        let error = WorldApplyErrorState {
            source: source.into(),
            message: err.to_string(),
        };
        self.counters.world_apply_errors += 1;
        self.apply_diagnostics.last_apply_error = Some(error.clone());
        self.apply_diagnostics.apply_errors.push(error.clone());
        let overflow = self
            .apply_diagnostics
            .apply_errors
            .len()
            .saturating_sub(MAX_WORLD_APPLY_ERRORS);
        if overflow > 0 {
            self.apply_diagnostics.apply_errors.drain(0..overflow);
        }
        error
    }

    pub fn apply_diagnostics(&self) -> &WorldApplyDiagnosticsState {
        &self.apply_diagnostics
    }

    pub fn world_apply_error_messages(&self) -> Vec<String> {
        self.apply_diagnostics
            .apply_errors
            .iter()
            .map(|error| error.message.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_apply_error_tracks_last_error_and_counter() {
        let mut store = WorldStore::new();

        store.record_apply_error("light_update", "missing chunk");
        store.record_apply_error("chunks_biomes", "invalid biome payload");

        let diagnostics = store.apply_diagnostics();
        assert_eq!(diagnostics.apply_errors.len(), 2);
        assert_eq!(diagnostics.apply_errors[0].source, "light_update");
        assert_eq!(diagnostics.apply_errors[0].message, "missing chunk");
        assert_eq!(
            diagnostics.last_apply_error,
            Some(WorldApplyErrorState {
                source: "chunks_biomes".to_string(),
                message: "invalid biome payload".to_string(),
            })
        );
        assert_eq!(store.counters().world_apply_errors, 2);
        assert_eq!(
            store.world_apply_error_messages(),
            vec![
                "missing chunk".to_string(),
                "invalid biome payload".to_string()
            ]
        );
    }

    #[test]
    fn record_apply_error_keeps_recent_errors_but_total_count() {
        let mut store = WorldStore::new();

        for index in 0..70 {
            store.record_apply_error("level_chunk_with_light", format!("error {index}"));
        }

        let diagnostics = store.apply_diagnostics();
        assert_eq!(store.counters().world_apply_errors, 70);
        assert_eq!(diagnostics.apply_errors.len(), MAX_WORLD_APPLY_ERRORS);
        assert_eq!(diagnostics.apply_errors[0].message, "error 6");
        assert_eq!(
            diagnostics
                .last_apply_error
                .as_ref()
                .map(|error| error.message.as_str()),
            Some("error 69")
        );
    }
}
