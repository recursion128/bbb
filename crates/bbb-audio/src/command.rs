use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioCommand {
    PlayLocalSound(PlayLocalSoundCommand),
    PlayPositionedSound(PlayPositionedSoundCommand),
    PlayEntitySound(PlayEntitySoundCommand),
    StopSound(StopSoundCommand),
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopSoundCommand {
    pub category: Option<AudioCategory>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickEntitySoundPositionsCommand {
    pub listener: Option<AudioListenerState>,
    pub entities: Vec<EntitySoundPosition>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedSound {
    pub event_id: String,
    pub sound_name: String,
    pub ogg_path: PathBuf,
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
    if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
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
