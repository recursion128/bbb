use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioCounters {
    pub enabled: bool,
    #[serde(default)]
    pub disabled_reason: Option<String>,
    #[serde(default)]
    pub catalog_events: usize,
    #[serde(default)]
    pub registry_entries: usize,
    #[serde(default)]
    pub registry_updates: u64,
    #[serde(default)]
    pub commands_submitted: u64,
    #[serde(default)]
    pub resolve_failures: u64,
    #[serde(default)]
    pub submit_failures: u64,
    #[serde(default)]
    pub last_resolve_error: Option<String>,
    #[serde(default)]
    pub last_submit_error: Option<String>,
}

impl AudioCounters {
    pub fn disabled(reason: impl Into<String>) -> Self {
        Self {
            disabled_reason: Some(reason.into()),
            ..Self::default()
        }
    }
}
