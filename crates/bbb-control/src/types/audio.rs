use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioCounters {
    pub enabled: bool,
    #[serde(default)]
    pub disabled_reason: Option<String>,
    /// Render-owned Kira playback identity projected back to simulation.
    /// `None` includes explicit stop and natural completion.
    #[serde(default)]
    pub active_music_event_id: Option<String>,
    #[serde(default)]
    pub catalog_events: usize,
    #[serde(default)]
    pub registry_entries: usize,
    #[serde(default)]
    pub sound_cache_buffers: usize,
    #[serde(default)]
    pub sound_cache_bytes: u64,
    #[serde(default)]
    pub sound_cache_peak_bytes: u64,
    #[serde(default)]
    pub sound_cache_hits: u64,
    #[serde(default)]
    pub sound_cache_misses: u64,
    #[serde(default)]
    pub sound_cache_evictions: u64,
    #[serde(default)]
    pub sound_admission_rejections: u64,
    #[serde(default)]
    pub sound_streaming_starts: u64,
    #[serde(default)]
    pub sound_static_channels_used: usize,
    /// Kira static and streaming handles share this runtime admission upper
    /// bound; the duplicate fields preserve Vanilla's two-pool debug shape
    /// without claiming two independent bbb pools.
    #[serde(default)]
    pub sound_static_channels_capacity: usize,
    #[serde(default)]
    pub sound_streaming_channels_used: usize,
    /// See `sound_static_channels_capacity`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residency_observations_default_for_older_payloads() {
        let counters: AudioCounters = serde_json::from_str(r#"{"enabled":true}"#).unwrap();

        assert_eq!(counters.sound_cache_peak_bytes, 0);
        assert_eq!(counters.sound_cache_hits, 0);
        assert_eq!(counters.sound_cache_misses, 0);
        assert_eq!(counters.sound_cache_evictions, 0);
        assert_eq!(counters.sound_admission_rejections, 0);
        assert_eq!(counters.sound_streaming_starts, 0);
        assert_eq!(counters.active_music_event_id, None);
    }
}
