//! Vanilla `DisconnectedScreen` layout, drawing, and generation-tagged hit testing.

use winit::dpi::PhysicalSize;

use super::{
    absolute_hud_rect, hud_component_runs_width, hud_font_runs_width,
    hud_wrap_component_runs_with_interactions, push_hud_component_screen_text_draw,
    push_hud_debug_options_button, push_hud_draw_with_uv_and_tint, push_hud_screen_text_draw,
    HudComponentScreenTextDraw, HudComponentTextRun, HudDebugOptionsButtonSprites, HudDrawCommand,
    HudFontGlyphBook, HudObfuscatedGlyphPool, HudScreenTextDraw, HudSpriteGpu, HudStyledTextRun,
    HudUvRect, HudVertex, HUD_TINT_WHITE,
};

const BUTTON_WIDTH: i32 = 200;
const BUTTON_HEIGHT: i32 = 20;
const FONT_LINE_HEIGHT: i32 = 9;
const LARGE_CELL_PADDING: i32 = 10;
const BUTTON_CELL_PADDING: i32 = 2;
const MENU_BACKGROUND_TILE_SIZE: f32 = 32.0;
const PANORAMA_OVERLAY_LOGICAL_WIDTH: f32 = 16.0;
const PANORAMA_OVERLAY_LOGICAL_HEIGHT: f32 = 128.0;
const MAX_SCREEN_TEXT_CHARS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDisconnectedScreenButton {
    ReportToServer,
    OpenReportDirectory,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudDisconnectedScreenHitTarget {
    pub generation: u64,
    pub button: HudDisconnectedScreenButton,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudDisconnectedScreen {
    pub generation: u64,
    pub panorama_rotation_degrees: f32,
    pub title: String,
    pub reason_runs: Vec<HudComponentTextRun>,
    pub report_to_server_label: Option<String>,
    pub open_report_directory_label: Option<String>,
    pub back_label: String,
    pub focused: HudDisconnectedScreenButton,
    pub hovered: Option<HudDisconnectedScreenButton>,
}

#[derive(Debug, Clone, PartialEq)]
struct DisconnectedLayout {
    title_y: i32,
    reason_y: i32,
    reason_lines: Vec<Vec<HudComponentTextRun>>,
    buttons: Vec<(HudDisconnectedScreenButton, ButtonRect)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ButtonRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl ButtonRect {
    fn contains(self, position: (f64, f64)) -> bool {
        position.0 >= f64::from(self.x)
            && position.1 >= f64::from(self.y)
            && position.0 < f64::from(self.x.saturating_add(self.width))
            && position.1 < f64::from(self.y.saturating_add(self.height))
    }
}

pub(super) fn sanitize_hud_disconnected_screen(
    mut screen: HudDisconnectedScreen,
) -> Option<HudDisconnectedScreen> {
    if !screen.panorama_rotation_degrees.is_finite() {
        screen.panorama_rotation_degrees = 0.0;
    }
    sanitize_string(&mut screen.title);
    sanitize_component_runs(&mut screen.reason_runs);
    if let Some(label) = screen.report_to_server_label.as_mut() {
        sanitize_string(label);
    }
    if let Some(label) = screen.open_report_directory_label.as_mut() {
        sanitize_string(label);
    }
    sanitize_string(&mut screen.back_label);
    let valid = screen_buttons(&screen);
    if !valid.contains(&screen.focused) {
        screen.focused = valid
            .first()
            .copied()
            .unwrap_or(HudDisconnectedScreenButton::Back);
    }
    screen.hovered = screen.hovered.filter(|button| valid.contains(button));
    Some(screen)
}

pub(super) fn push_panorama_overlay<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    overlay: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        overlay,
        surface_size,
        absolute_hud_rect(0.0, 0.0, surface_size.width, surface_size.height),
        HudUvRect {
            min: [0.0, 0.0],
            max: [
                surface_size.width as f32 / PANORAMA_OVERLAY_LOGICAL_WIDTH,
                surface_size.height as f32 / PANORAMA_OVERLAY_LOGICAL_HEIGHT,
            ],
        },
        HUD_TINT_WHITE,
    );
}

pub(super) fn hud_disconnected_screen_hit_target_at(
    screen: &HudDisconnectedScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudDisconnectedScreenHitTarget> {
    disconnected_layout(screen, fonts, surface_size)
        .buttons
        .into_iter()
        .find_map(|(button, rect)| {
            rect.contains(position)
                .then_some(HudDisconnectedScreenHitTarget {
                    generation: screen.generation,
                    button,
                })
        })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_disconnected_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    menu_background: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    fonts: &HudFontGlyphBook,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDisconnectedScreen,
) {
    push_menu_background(
        vertices,
        commands,
        white_pixel,
        menu_background,
        surface_size,
    );
    let Some(font_atlas) = font_atlas else { return };
    let glyphs = fonts.glyphs(None);
    let layout = disconnected_layout(screen, fonts, surface_size);
    let title_runs = [HudStyledTextRun::plain(screen.title.clone())];
    let title_width = hud_font_runs_width(&title_runs, glyphs).unwrap_or_default();
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
            runs: &title_runs,
            origin: (
                surface_size.width.max(1) as f32 * 0.5 - title_width as f32 * 0.5,
                layout.title_y as f32,
            ),
            scale: 1.0,
            tint: HUD_TINT_WHITE,
            backdrop_opacity: 0.0,
        },
    );
    for (line_index, line) in layout.reason_lines.iter().enumerate() {
        let width = hud_component_runs_width(line, fonts).unwrap_or_default();
        push_hud_component_screen_text_draw(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            fonts,
            frame_index,
            surface_size,
            &HudComponentScreenTextDraw {
                runs: line,
                origin: (
                    surface_size.width.max(1) as f32 * 0.5 - width as f32 * 0.5,
                    layout.reason_y.saturating_add(
                        i32::try_from(line_index)
                            .unwrap_or(i32::MAX)
                            .saturating_mul(FONT_LINE_HEIGHT),
                    ) as f32,
                ),
                scale: 1.0,
                tint: HUD_TINT_WHITE,
                backdrop_opacity: 0.0,
            },
        );
    }
    for (button, rect) in layout.buttons {
        let label = match button {
            HudDisconnectedScreenButton::ReportToServer => {
                screen.report_to_server_label.as_deref().unwrap_or_default()
            }
            HudDisconnectedScreenButton::OpenReportDirectory => screen
                .open_report_directory_label
                .as_deref()
                .unwrap_or_default(),
            HudDisconnectedScreenButton::Back => &screen.back_label,
        };
        push_hud_debug_options_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            label,
            true,
            screen.hovered == Some(button) || screen.focused == button,
            HUD_TINT_WHITE,
            button_sprites,
        );
    }
}

fn disconnected_layout(
    screen: &HudDisconnectedScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
) -> DisconnectedLayout {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let reason_width = u32::try_from(width.saturating_sub(50).max(1)).unwrap_or(1);
    let mut reason_lines =
        hud_wrap_component_runs_with_interactions(&screen.reason_runs, &[], reason_width, fonts)
            .into_iter()
            .map(|line| line.runs)
            .collect::<Vec<_>>();
    if reason_lines.is_empty() {
        reason_lines.push(Vec::new());
    }
    let reason_height = i32::try_from(reason_lines.len())
        .unwrap_or(i32::MAX / FONT_LINE_HEIGHT)
        .saturating_mul(FONT_LINE_HEIGHT);
    let buttons = screen_buttons(screen);
    let content_height = FONT_LINE_HEIGHT
        .saturating_add(LARGE_CELL_PADDING * 2)
        .saturating_add(reason_height)
        .saturating_add(LARGE_CELL_PADDING * 2)
        .saturating_add(
            i32::try_from(buttons.len())
                .unwrap_or(i32::MAX / (BUTTON_HEIGHT + BUTTON_CELL_PADDING * 2))
                .saturating_mul(BUTTON_HEIGHT + BUTTON_CELL_PADDING * 2),
        );
    let top = height.saturating_sub(content_height) / 2;
    let title_y = top.saturating_add(LARGE_CELL_PADDING);
    let reason_y = title_y
        .saturating_add(FONT_LINE_HEIGHT)
        .saturating_add(LARGE_CELL_PADDING * 2);
    let button_top = reason_y
        .saturating_add(reason_height)
        .saturating_add(LARGE_CELL_PADDING)
        .saturating_add(BUTTON_CELL_PADDING);
    let x = width / 2 - BUTTON_WIDTH / 2;
    let buttons = buttons
        .into_iter()
        .enumerate()
        .map(|(index, button)| {
            let y = button_top.saturating_add(
                i32::try_from(index)
                    .unwrap_or(i32::MAX / (BUTTON_HEIGHT + BUTTON_CELL_PADDING * 2))
                    .saturating_mul(BUTTON_HEIGHT + BUTTON_CELL_PADDING * 2),
            );
            (
                button,
                ButtonRect {
                    x,
                    y,
                    width: BUTTON_WIDTH,
                    height: BUTTON_HEIGHT,
                },
            )
        })
        .collect();
    DisconnectedLayout {
        title_y,
        reason_y,
        reason_lines,
        buttons,
    }
}

fn screen_buttons(screen: &HudDisconnectedScreen) -> Vec<HudDisconnectedScreenButton> {
    let mut buttons = Vec::with_capacity(3);
    if screen.report_to_server_label.is_some() {
        buttons.push(HudDisconnectedScreenButton::ReportToServer);
    }
    if screen.open_report_directory_label.is_some() {
        buttons.push(HudDisconnectedScreenButton::OpenReportDirectory);
    }
    buttons.push(HudDisconnectedScreenButton::Back);
    buttons
}

fn sanitize_string(value: &mut String) {
    *value = value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(MAX_SCREEN_TEXT_CHARS)
        .collect();
}

fn sanitize_component_runs(runs: &mut Vec<HudComponentTextRun>) {
    runs.truncate(1_024);
    let mut remaining = MAX_SCREEN_TEXT_CHARS;
    for run in runs.iter_mut() {
        run.visual.text = run
            .visual
            .text
            .chars()
            .filter(|ch| *ch == '\n' || !ch.is_control())
            .take(remaining)
            .collect();
        remaining = remaining.saturating_sub(run.visual.text.chars().count());
        if remaining == 0 {
            break;
        }
    }
    runs.retain(|run| !run.visual.text.is_empty());
}

pub(super) fn push_menu_background<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    surface_size: PhysicalSize<u32>,
) {
    let (sprite, uv, tint) = background.map_or(
        (
            white_pixel,
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.12, 0.12, 0.12, 0.96],
        ),
        |background| {
            (
                background,
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [
                        surface_size.width as f32 / MENU_BACKGROUND_TILE_SIZE,
                        surface_size.height as f32 / MENU_BACKGROUND_TILE_SIZE,
                    ],
                },
                HUD_TINT_WHITE,
            )
        },
    );
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        absolute_hud_rect(0.0, 0.0, surface_size.width, surface_size.height),
        uv,
        tint,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{HudAsciiGlyph, HudFontGlyphMap, HudTextStyle};

    fn fonts_for(text: &str) -> HudFontGlyphBook {
        let mut glyphs = HudFontGlyphMap::new();
        let glyph = HudAsciiGlyph {
            width: 5,
            height: 8,
            advance: 6,
            ..HudAsciiGlyph::default()
        };
        for ch in text.chars().chain(std::iter::once('?')) {
            glyphs.insert_first_wins(ch, glyph);
        }
        HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        )
    }

    fn screen() -> HudDisconnectedScreen {
        HudDisconnectedScreen {
            generation: 7,
            panorama_rotation_degrees: 0.0,
            title: "Connection Lost".to_string(),
            reason_runs: vec![HudComponentTextRun {
                visual: HudStyledTextRun {
                    text: "A reason long enough to wrap on a narrow surface".to_string(),
                    style: HudTextStyle::default(),
                    color: Some(0xFF_00_00),
                },
                font: None,
            }],
            report_to_server_label: Some("Report to Server".to_string()),
            open_report_directory_label: Some("Open Report Directory".to_string()),
            back_label: "Back to Server List".to_string(),
            focused: HudDisconnectedScreenButton::ReportToServer,
            hovered: None,
        }
    }

    #[test]
    fn layout_centers_content_wraps_reason_and_keeps_vanilla_button_order() {
        let fonts = fonts_for("A reason long enough to wrap on a narrow surface");
        let layout = disconnected_layout(&screen(), &fonts, PhysicalSize::new(180, 160));
        assert!(layout.reason_lines.len() > 1);
        assert_eq!(layout.buttons.len(), 3);
        assert_eq!(
            layout.buttons[0].0,
            HudDisconnectedScreenButton::ReportToServer
        );
        assert_eq!(
            layout.buttons[1].0,
            HudDisconnectedScreenButton::OpenReportDirectory
        );
        assert_eq!(layout.buttons[2].0, HudDisconnectedScreenButton::Back);
        assert_eq!(layout.buttons[0].1.width, 200);
        assert!(layout.title_y >= 0);
    }

    #[test]
    fn hit_target_carries_generation_and_sanitizer_repairs_stale_focus() {
        let fonts = fonts_for("A reason long enough to wrap on a narrow surface");
        let mut value = screen();
        value.report_to_server_label = None;
        value.focused = HudDisconnectedScreenButton::ReportToServer;
        value.hovered = Some(HudDisconnectedScreenButton::ReportToServer);
        let value = sanitize_hud_disconnected_screen(value).unwrap();
        assert_eq!(
            value.focused,
            HudDisconnectedScreenButton::OpenReportDirectory
        );
        assert_eq!(value.hovered, None);
        let layout = disconnected_layout(&value, &fonts, PhysicalSize::new(320, 240));
        let rect = layout.buttons[0].1;
        assert_eq!(
            hud_disconnected_screen_hit_target_at(
                &value,
                &fonts,
                PhysicalSize::new(320, 240),
                (f64::from(rect.x + 1), f64::from(rect.y + 1)),
            ),
            Some(HudDisconnectedScreenHitTarget {
                generation: 7,
                button: HudDisconnectedScreenButton::OpenReportDirectory,
            })
        );
    }
}
