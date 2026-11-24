//! Shared environment audio values.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SoundEventHolderValue {
    Reference {
        resource_id: String,
    },
    Direct {
        sound_id: String,
        range_bits: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Music {
    pub sound: SoundEventHolderValue,
    pub min_delay: i32,
    pub max_delay: i32,
    pub replace_current_music: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundMusic {
    pub default: Option<Music>,
    pub creative: Option<Music>,
    pub underwater: Option<Music>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientMood {
    pub sound: SoundEventHolderValue,
    pub tick_delay: i32,
    pub block_search_extent: i32,
    pub offset_bits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientAddition {
    pub sound: SoundEventHolderValue,
    pub tick_chance_bits: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientSounds {
    pub loop_sound: Option<SoundEventHolderValue>,
    pub mood: Option<AmbientMood>,
    pub additions: Vec<AmbientAddition>,
}
