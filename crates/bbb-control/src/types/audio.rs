use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioCounters {
    pub enabled: bool,
    #[serde(default)]
    pub disabled_reason: Option<String>,
    /// Render-owned playback identity projected back to simulation. Stopped
    /// instances remain visible through Vanilla's minimum delete lifetime.
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
    pub sound_preload_generation: u64,
    #[serde(default)]
    pub sound_preload_requested: usize,
    #[serde(default)]
    pub sound_preload_completed: usize,
    #[serde(default)]
    pub sound_preload_failures: usize,
    #[serde(default)]
    pub sound_preload_pending: usize,
    #[serde(default)]
    pub sound_preload_worker_jobs: u64,
    #[serde(default)]
    pub sound_preload_cancelled_jobs: u64,
    #[serde(default)]
    pub sound_preload_stale_results: u64,
    #[serde(default)]
    pub last_sound_preload_error: Option<String>,
    #[serde(default)]
    pub sound_static_channels_used: usize,
    /// Static admission limit derived with Vanilla's pool formula. The
    /// companion origin identifies whether the source count is device-backed.
    #[serde(default)]
    pub sound_static_channels_capacity: usize,
    #[serde(default)]
    pub sound_streaming_channels_used: usize,
    /// Independent streaming admission limit derived from the same source
    /// count; saturating either pool must not block the other.
    #[serde(default)]
    pub sound_streaming_channels_capacity: usize,
    #[serde(default)]
    pub sound_channel_admission_rejections: u64,
    #[serde(default)]
    pub sound_channel_capacity_origin: Option<String>,
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
    /// Empty `Options.soundDevice` is projected as `None`; any non-empty
    /// display name remains attributable in control receipts.
    #[serde(default)]
    pub preferred_device: Option<String>,
    #[serde(default)]
    pub current_device: Option<String>,
    #[serde(default)]
    pub default_device: Option<String>,
    #[serde(default)]
    pub available_device_count: usize,
    #[serde(default)]
    pub device_reloads: u64,
    #[serde(default)]
    pub device_reload_failures: u64,
    #[serde(default)]
    pub device_query_failures: u64,
    #[serde(default)]
    pub preferred_device_fallbacks: u64,
    #[serde(default)]
    pub last_device_reload_reason: Option<String>,
    #[serde(default)]
    pub last_device_error: Option<String>,
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
        assert_eq!(counters.sound_preload_requested, 0);
        assert_eq!(counters.sound_preload_completed, 0);
        assert_eq!(counters.sound_preload_pending, 0);
        assert_eq!(counters.last_sound_preload_error, None);
        assert_eq!(counters.sound_channel_admission_rejections, 0);
        assert_eq!(counters.sound_channel_capacity_origin, None);
        assert_eq!(counters.active_music_event_id, None);
        assert_eq!(counters.preferred_device, None);
        assert_eq!(counters.available_device_count, 0);
        assert_eq!(counters.device_reloads, 0);
    }
}
