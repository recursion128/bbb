//! Camera-overlay vignette state.
//!
//! The GPU owner lives here rather than in the general HUD command stream
//! because vanilla's `RenderPipelines.VIGNETTE` uses a destination-multiplying
//! blend that cannot share the ordinary alpha-blended HUD pipeline.

use crate::Renderer;

/// One frame's vanilla `Gui.extractVignette` projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudVignette {
    /// Smoothed `Gui.vignetteBrightness`, clamped to `[0, 1]`.
    pub brightness: f32,
    /// World-border warning strength after the vanilla clamp.
    pub border_warning_strength: f32,
    /// Opaque RGB color multiplied into `textures/misc/vignette.png`.
    pub color: [f32; 3],
}

impl HudVignette {
    fn sanitized(self) -> Option<Self> {
        if !self.brightness.is_finite()
            || !self.border_warning_strength.is_finite()
            || !self.color.iter().all(|channel| channel.is_finite())
        {
            return None;
        }
        Some(Self {
            brightness: self.brightness.clamp(0.0, 1.0),
            border_warning_strength: self.border_warning_strength.clamp(0.0, 1.0),
            color: self.color.map(|channel| channel.clamp(0.0, 1.0)),
        })
    }
}

impl Renderer {
    /// Replaces the latest camera-overlay vignette projection.
    pub fn set_hud_vignette(&mut self, vignette: Option<HudVignette>) {
        self.hud_vignette = vignette.and_then(HudVignette::sanitized);
    }

    /// Returns the sanitized vignette state currently owned by the renderer.
    pub fn hud_vignette(&self) -> Option<HudVignette> {
        self.hud_vignette
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vignette_projection_sanitizes_ranges_and_rejects_non_finite_values() {
        assert_eq!(
            HudVignette {
                brightness: 2.0,
                border_warning_strength: -1.0,
                color: [-0.5, 0.5, 1.5],
            }
            .sanitized(),
            Some(HudVignette {
                brightness: 1.0,
                border_warning_strength: 0.0,
                color: [0.0, 0.5, 1.0],
            })
        );
        assert_eq!(
            HudVignette {
                brightness: f32::NAN,
                border_warning_strength: 0.0,
                color: [1.0; 3],
            }
            .sanitized(),
            None
        );
    }
}
