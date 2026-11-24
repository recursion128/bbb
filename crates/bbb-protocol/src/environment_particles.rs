//! Pure configured environment particle values.

use crate::{packets::ItemStackSummary, NbtSummaryEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticleOptions {
    pub resource_id: String,
    pub fields: Vec<NbtSummaryEntry>,
    #[serde(default)]
    pub option: ParticleOptionValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParticleOptionValue {
    #[default]
    Empty,
    Block {
        block_state_id: i32,
    },
    Power {
        power_bits: u32,
    },
    Dust {
        color: u32,
        scale_bits: u32,
    },
    DustColorTransition {
        from_color: u32,
        to_color: u32,
        scale_bits: u32,
    },
    Spell {
        color: u32,
        power_bits: u32,
    },
    Color {
        argb: u32,
    },
    SculkCharge {
        roll_bits: u32,
    },
    Item {
        stack: ItemStackSummary,
    },
    Vibration {
        block_pos: [i32; 3],
        arrival_in_ticks: i32,
    },
    Trail {
        target_bits: [u64; 3],
        color: u32,
        duration: i32,
    },
    Shriek {
        delay: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientParticle {
    pub particle: ParticleOptions,
    pub probability_bits: u32,
}
