//! Vanilla `LevelLoadingScreen` background selection and centered status line.

use winit::dpi::PhysicalSize;

use super::disconnected_screen::push_menu_background;
use super::{
    absolute_hud_rect, hud_font_runs_width, hud_quad_vertices, push_hud_screen_text_draw,
    HudDrawCommand, HudFontGlyphMap, HudObfuscatedGlyphPool, HudScreenTextDraw, HudSpriteGpu,
    HudStyledTextRun, HudUvRect, HudVertex, HUD_TINT_WHITE,
};

/// Vanilla `LevelLoadingScreen.extractRenderState` places the status line at
/// `guiHeight / 2 - 50` when the server sends no chunk status view, which is
/// always the case for an external client.
const STATUS_LINE_OFFSET_ABOVE_CENTER: i32 = 50;
const MAX_STATUS_LINE_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HudLevelLoadingBackground {
    Panorama { rotation_degrees: f32 },
    NetherPortal { sprite_uv: HudUvRect },
    EndPortal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudLevelLoadingScreen {
    pub background: HudLevelLoadingBackground,
    pub downloading_terrain: String,
}

pub(super) fn sanitize_hud_level_loading_screen(
    mut screen: HudLevelLoadingScreen,
) -> Option<HudLevelLoadingScreen> {
    if let HudLevelLoadingBackground::Panorama { rotation_degrees } = screen.background {
        if !rotation_degrees.is_finite() {
            screen.background = HudLevelLoadingBackground::Panorama {
                rotation_degrees: 0.0,
            };
        }
    }
    if let HudLevelLoadingBackground::NetherPortal { sprite_uv } = screen.background {
        if !sprite_uv
            .min
            .iter()
            .chain(sprite_uv.max.iter())
            .all(|value| value.is_finite())
        {
            return None;
        }
    }
    screen.downloading_terrain = screen
        .downloading_terrain
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_STATUS_LINE_CHARS)
        .collect();
    Some(screen)
}

pub(super) fn hud_level_loading_status_line_top(surface_size: PhysicalSize<u32>) -> i32 {
    i32::try_from(surface_size.height.max(1)).unwrap_or(i32::MAX) / 2
        - STATUS_LINE_OFFSET_ABOVE_CENTER
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_level_loading_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    inworld_menu_background: Option<&'a HudSpriteGpu>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudLevelLoadingScreen,
) {
    match screen.background {
        HudLevelLoadingBackground::Panorama { .. } => {
            push_menu_background(
                vertices,
                commands,
                white_pixel,
                inworld_menu_background,
                surface_size,
            );
        }
        HudLevelLoadingBackground::NetherPortal { sprite_uv } => {
            let start = vertices.len() as u32;
            vertices.extend_from_slice(&hud_quad_vertices(
                surface_size,
                absolute_hud_rect(0.0, 0.0, surface_size.width, surface_size.height),
                sprite_uv,
                HUD_TINT_WHITE,
            ));
            let end = vertices.len() as u32;
            commands.push(HudDrawCommand::TerrainAtlas { start, end });
        }
        HudLevelLoadingBackground::EndPortal => {}
    }
    let Some(font_atlas) = font_atlas else { return };
    let runs = [HudStyledTextRun::plain(screen.downloading_terrain.as_str())];
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or_default() as f32;
    let status_line_top = hud_level_loading_status_line_top(surface_size);
    push_hud_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &HudScreenTextDraw {
            runs: &runs,
            origin: (
                surface_size.width.max(1) as f32 * 0.5 - width * 0.5,
                status_line_top as f32,
            ),
            scale: 1.0,
            tint: HUD_TINT_WHITE,
            backdrop_opacity: 0.0,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_line_sits_fifty_gui_pixels_above_the_centre() {
        assert_eq!(
            hud_level_loading_status_line_top(PhysicalSize::new(320, 240)),
            70
        );
        assert_eq!(
            hud_level_loading_status_line_top(PhysicalSize::new(854, 480)),
            190
        );
    }

    #[test]
    fn sanitizing_drops_control_characters_and_a_non_finite_portal_sprite() {
        let sanitized = sanitize_hud_level_loading_screen(HudLevelLoadingScreen {
            background: HudLevelLoadingBackground::Panorama {
                rotation_degrees: f32::NAN,
            },
            downloading_terrain: "Loading\u{7}terrain".to_string(),
        })
        .expect("panorama background survives");
        assert_eq!(
            sanitized.background,
            HudLevelLoadingBackground::Panorama {
                rotation_degrees: 0.0
            }
        );
        assert_eq!(sanitized.downloading_terrain, "Loadingterrain");

        assert!(sanitize_hud_level_loading_screen(HudLevelLoadingScreen {
            background: HudLevelLoadingBackground::NetherPortal {
                sprite_uv: HudUvRect {
                    min: [0.0, f32::INFINITY],
                    max: [1.0, 1.0],
                },
            },
            downloading_terrain: String::new(),
        })
        .is_none());
    }
}
