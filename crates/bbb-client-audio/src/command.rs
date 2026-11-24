use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioCommand {
    PlayLocalSound(PlayLocalSoundCommand),
    PlayPositionedSound(PlayPositionedSoundCommand),
    PlayEntitySound(PlayEntitySoundCommand),
    PlayJukeboxSong(PlayJukeboxSongCommand),
    StopJukeboxSong(StopJukeboxSongCommand),
    StopSound(StopSoundCommand),
    SetMusicGain(SetMusicGainCommand),
    SyncAmbientLoops(SyncAmbientLoopsCommand),
    TickEntitySoundPositions(TickEntitySoundPositionsCommand),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayLocalSoundCommand {
    pub sound: ResolvedSound,
    pub category: AudioCategory,
    pub packet_volume: f32,
    pub packet_pitch: f32,
    pub gain: f32,
    pub channel_gain: f32,
    pub playback_rate: f32,
    pub seed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayPositionedSoundCommand {
    pub sound: ResolvedSound,
    pub category: AudioCategory,
    pub position: [f64; 3],
    pub packet_volume: f32,
    pub packet_pitch: f32,
    pub gain: f32,
    pub channel_gain: f32,
    pub playback_rate: f32,
    pub seed: i64,
    pub fixed_range: Option<f32>,
    #[serde(default)]
    pub distance_delay: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayEntitySoundCommand {
    pub sound: ResolvedSound,
    pub category: AudioCategory,
    pub entity_id: i32,
    pub position: Option<[f64; 3]>,
    pub packet_volume: f32,
    pub packet_pitch: f32,
    pub gain: f32,
    pub channel_gain: f32,
    pub playback_rate: f32,
    pub seed: i64,
    pub fixed_range: Option<f32>,
    #[serde(default)]
    pub looping: bool,
    #[serde(default = "default_true")]
    pub linear_attenuation: bool,
    #[serde(default)]
    pub tickable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayJukeboxSongCommand {
    pub sound: ResolvedSound,
    pub category: AudioCategory,
    pub position: [f64; 3],
    pub jukebox_pos: [i32; 3],
    pub packet_volume: f32,
    pub packet_pitch: f32,
    pub gain: f32,
    pub channel_gain: f32,
    pub playback_rate: f32,
    pub seed: i64,
    pub fixed_range: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopJukeboxSongCommand {
    pub jukebox_pos: [i32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopSoundCommand {
    pub category: Option<AudioCategory>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetMusicGainCommand {
    pub gain: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncAmbientLoopsCommand {
    pub loops: Vec<AmbientLoopSoundCommand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientLoopSoundCommand {
    pub sound: ResolvedSound,
    pub channel_gain: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickEntitySoundPositionsCommand {
    pub listener: Option<AudioListenerState>,
    pub entities: Vec<EntitySoundPosition>,
    #[serde(default)]
    pub tickable_sounds: Vec<TickableEntitySoundUpdate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickableEntitySoundUpdate {
    pub event_id: String,
    pub entity_id: i32,
    pub position: [f64; 3],
    pub channel_gain: f32,
    pub playback_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioListenerState {
    pub position: [f64; 3],
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntitySoundPosition {
    pub entity_id: i32,
    pub position: [f64; 3],
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedSound {
    pub event_id: String,
    pub sound_name: String,
    pub ogg_path: PathBuf,
    #[serde(default)]
    pub resource_generation: u64,
    pub stream: bool,
    pub preload: bool,
    pub attenuation_distance: i32,
    pub entry_volume: f32,
    pub entry_pitch: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCategory {
    Master,
    Music,
    Records,
    Weather,
    Blocks,
    Hostile,
    Neutral,
    Players,
    Ambient,
    Voice,
    Ui,
    Unknown(String),
}

impl AudioCategory {
    pub fn from_world_source(source: &str) -> Self {
        match source {
            "master" => Self::Master,
            "music" => Self::Music,
            "record" => Self::Records,
            "weather" => Self::Weather,
            "block" => Self::Blocks,
            "hostile" => Self::Hostile,
            "neutral" => Self::Neutral,
            "player" => Self::Players,
            "ambient" => Self::Ambient,
            "voice" => Self::Voice,
            "ui" => Self::Ui,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_world_source(&self) -> &str {
        match self {
            Self::Master => "master",
            Self::Music => "music",
            Self::Records => "record",
            Self::Weather => "weather",
            Self::Blocks => "block",
            Self::Hostile => "hostile",
            Self::Neutral => "neutral",
            Self::Players => "player",
            Self::Ambient => "ambient",
            Self::Voice => "voice",
            Self::Ui => "ui",
            Self::Unknown(source) => source,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioVolumeSettings {
    pub master: f32,
    pub music: f32,
    pub records: f32,
    pub weather: f32,
    pub blocks: f32,
    pub hostile: f32,
    pub neutral: f32,
    pub players: f32,
    pub ambient: f32,
    pub voice: f32,
    pub ui: f32,
}

impl Default for AudioVolumeSettings {
    fn default() -> Self {
        Self {
            master: 1.0,
            music: 1.0,
            records: 1.0,
            weather: 1.0,
            blocks: 1.0,
            hostile: 1.0,
            neutral: 1.0,
            players: 1.0,
            ambient: 1.0,
            voice: 1.0,
            ui: 1.0,
        }
    }
}

impl AudioVolumeSettings {
    pub fn final_source_volume(&self, category: &AudioCategory) -> f32 {
        let source = clamp_audio_volume(self.source_volume(category));
        if matches!(category, AudioCategory::Master) {
            source
        } else {
            source * clamp_audio_volume(self.master)
        }
    }

    pub fn channel_gain(&self, instance_gain: f32, category: &AudioCategory) -> f32 {
        clamp_audio_volume(instance_gain) * self.final_source_volume(category)
    }

    fn source_volume(&self, category: &AudioCategory) -> f32 {
        match category {
            AudioCategory::Master => self.master,
            AudioCategory::Music => self.music,
            AudioCategory::Records => self.records,
            AudioCategory::Weather => self.weather,
            AudioCategory::Blocks => self.blocks,
            AudioCategory::Hostile => self.hostile,
            AudioCategory::Neutral => self.neutral,
            AudioCategory::Players => self.players,
            AudioCategory::Ambient => self.ambient,
            AudioCategory::Voice => self.voice,
            AudioCategory::Ui => self.ui,
            AudioCategory::Unknown(_) => 1.0,
        }
    }
}

fn clamp_audio_volume(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioPlaybackEvent {
    pub event_id: String,
    pub position: [f64; 3],
    pub range: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioChannelObservation {
    pub static_used: usize,
    pub streaming_used: usize,
    pub static_capacity: usize,
    pub streaming_capacity: usize,
    pub admission_rejections: u64,
    pub native_static_buffers: usize,
    pub native_static_bytes: u64,
    pub native_static_evictions: u64,
    /// Exact capability owner for the active backend.
    pub capacity_origin: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioPreloadRequest {
    pub sound_name: String,
    pub ogg_path: PathBuf,
    pub resource_generation: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioPreloadObservation {
    pub resource_generation: u64,
    pub requested: usize,
    pub completed: usize,
    pub failures: usize,
    pub pending: usize,
    pub worker_jobs: u64,
    pub cancelled_jobs: u64,
    pub stale_results: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioResidencyObservation {
    pub static_buffers: usize,
    pub static_decoded_bytes: u64,
    pub peak_static_decoded_bytes: u64,
    pub static_cache_hits: u64,
    pub static_cache_misses: u64,
    pub static_cache_evictions: u64,
    pub audio_admission_rejections: u64,
    pub streaming_starts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFileProbe {
    pub sample_rate: u32,
    pub frames: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioDeviceSnapshot {
    pub default_device: Option<String>,
    pub all_devices: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDeviceReloadReason {
    CurrentDeviceDisconnected,
    CurrentDeviceDisappeared,
    SystemDefaultChanged,
    PreferredDeviceAvailable,
}

impl AudioDeviceReloadReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CurrentDeviceDisconnected => "current-device-disconnected",
            Self::CurrentDeviceDisappeared => "current-device-disappeared",
            Self::SystemDefaultChanged => "system-default-changed",
            Self::PreferredDeviceAvailable => "preferred-device-available",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioDeviceObservation {
    pub preferred_device: Option<String>,
    pub current_device: Option<String>,
    pub default_device: Option<String>,
    pub available_device_count: usize,
    pub reloads: u64,
    pub reload_failures: u64,
    pub query_failures: u64,
    pub preferred_fallbacks: u64,
    pub last_reload_reason: Option<String>,
    pub last_error: Option<String>,
    pub requested_directional_audio: bool,
    pub hrtf_extension: bool,
    pub hrtf_specifiers: i32,
    pub hrtf_status: i32,
    pub output_limiter: bool,
    pub supports_disconnection: bool,
    pub tracking_mode: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioDevicePoll {
    pub reload_reason: Option<AudioDeviceReloadReason>,
    pub error: Option<String>,
    pub observation_changed: bool,
}

/// `Options.isSoundDeviceDefault`: only the empty persisted string means the
/// system default. Whitespace and display labels are real preferred names.
pub fn is_sound_device_default(device_name: &str) -> bool {
    device_name.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_source_volume_matches_vanilla_master_and_category_mix() {
        let settings = AudioVolumeSettings {
            master: 0.5,
            music: 0.25,
            blocks: 2.0,
            hostile: -1.0,
            ..AudioVolumeSettings::default()
        };

        assert_eq!(settings.final_source_volume(&AudioCategory::Master), 0.5);
        assert_eq!(settings.final_source_volume(&AudioCategory::Music), 0.125);
        assert_eq!(settings.final_source_volume(&AudioCategory::Blocks), 0.5);
        assert_eq!(settings.final_source_volume(&AudioCategory::Hostile), 0.0);
        assert_eq!(
            settings.final_source_volume(&AudioCategory::Unknown("custom".to_string())),
            0.5
        );
    }

    #[test]
    fn channel_gain_clamps_instance_gain_before_source_volume() {
        let settings = AudioVolumeSettings {
            master: 0.5,
            neutral: 0.25,
            ..AudioVolumeSettings::default()
        };

        assert_eq!(settings.channel_gain(2.0, &AudioCategory::Neutral), 0.125);
        assert_eq!(settings.channel_gain(-1.0, &AudioCategory::Neutral), 0.0);
        assert!(settings
            .channel_gain(f32::NAN, &AudioCategory::Neutral)
            .is_nan());
    }
}
