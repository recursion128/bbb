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
    pub sound_cache_buffers: usize,
    #[serde(default)]
    pub sound_cache_bytes: u64,
    #[serde(default)]
    pub sound_static_channels_used: usize,
    #[serde(default)]
    pub sound_static_channels_capacity: usize,
    #[serde(default)]
    pub sound_streaming_channels_used: usize,
    #[serde(default)]
    pub sound_streaming_channels_capacity: usize,
    #[serde(default)]
    pub sound_mood_percent: u8,
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
