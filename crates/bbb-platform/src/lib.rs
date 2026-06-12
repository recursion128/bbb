use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalSize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "bbb-native".to_string(),
            width: 1280,
            height: 720,
        }
    }
}

impl WindowConfig {
    pub fn physical_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.width, self.height)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputSnapshot {
    pub close_requested: bool,
    pub focused: bool,
}
