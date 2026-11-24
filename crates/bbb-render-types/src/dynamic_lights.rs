//! Shared dynamic point-light value types passed between world extraction and renderer uniforms.

use serde::{Deserialize, Serialize};

pub const MAX_DYNAMIC_POINT_LIGHTS: usize = 32;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DynamicLightsMode {
    #[default]
    Off,
    Fast,
    Fancy,
}

impl DynamicLightsMode {
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub const fn shader_mode_id(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Fast => 1.0,
            Self::Fancy => 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DynamicPointLight {
    pub position: [f64; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}
