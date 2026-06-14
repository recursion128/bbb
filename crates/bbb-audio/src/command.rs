use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioCommand {
    PlayPositionedSound(PlayPositionedSoundCommand),
    PlayEntitySound(PlayEntitySoundCommand),
    StopSound(StopSoundCommand),
    TickEntitySoundPositions(TickEntitySoundPositionsCommand),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayPositionedSoundCommand {
    pub sound: ResolvedSound,
    pub category: AudioCategory,
    pub position: [f64; 3],
    pub packet_volume: f32,
    pub packet_pitch: f32,
    pub gain: f32,
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
