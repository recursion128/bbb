//! Vanilla `WinScreen`, demo `PopupScreen`, and trusted-link confirmation HUD.
//!
//! This module owns the shared draw/hit layout so pointer routing resolves
//! against the exact wrapped lines and button rectangles that are rendered.

use std::sync::Arc;
use winit::dpi::PhysicalSize;

use super::{
    absolute_hud_rect, hud_font_runs_width, hud_wrap_chat_runs_with_interactions,
    nine_slice_segments, push_hud_debug_options_button, push_hud_draw_with_uv_and_tint,
    push_hud_pause_background, push_hud_screen_text_draw, HudDebugOptionsButtonSprites,
    HudDrawCommand, HudFontGlyphMap, HudNineSliceSprite, HudObfuscatedGlyphPool, HudScreenTextDraw,
    HudSpriteGpu, HudStyledTextRun, HudTextStyle, HudUvRect, HudVertex, HUD_TINT_WHITE,
};

const WIN_LOGO_WIDTH: u32 = 256;
const WIN_LOGO_HEIGHT: u32 = 44;
const WIN_EDITION_WIDTH: u32 = 128;
const WIN_EDITION_HEIGHT: u32 = 14;
const WIN_EDITION_OVERLAP: i32 = 7;
const WIN_LOGO_TOP_OFFSET: i32 = 50;
const WIN_TEXT_TOP_OFFSET: i32 = 100;
const WIN_TEXT_LINE_HEIGHT: i32 = 12;
const WIN_LAST_LINE_CENTER_OFFSET: i32 = 6;

const POPUP_WIDTH: i32 = 250;
const POPUP_CONTENT_WIDTH: u32 = 214;
const POPUP_BORDER: i32 = 18;
const POPUP_CHILD_SPACING: i32 = 12;
const POPUP_BUTTON_GAP: i32 = 6;
const POPUP_BUTTON_HEIGHT: i32 = 20;
const POPUP_BUTTON_WIDTH: i32 = 104;
const FONT_LINE_HEIGHT: i32 = 9;

const CONFIRM_BUTTON_WIDTH: i32 = 100;
const CONFIRM_BUTTON_HEIGHT: i32 = 20;
const CONFIRM_BUTTON_GAP: i32 = 4;
const CONFIRM_CHILD_SPACING: i32 = 8;
const CONFIRM_BUTTON_TOP_PADDING: i32 = 16;
const MENU_BACKGROUND_TILE_SIZE: f32 = 32.0;
const MAX_SCREEN_TEXT_CHARS: usize = 65_536;
const MAX_WIN_LINES: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub struct HudWinScreenLine {
    pub runs: Vec<HudStyledTextRun>,
    pub centered: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudWinScreen {
    pub generation: u64,
    pub scroll: f32,
    pub show_easter_egg_logo: bool,
    pub lines: Arc<[HudWinScreenLine]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudGameEventButton {
    DemoBuy,
    DemoLater,
    LinkOpen,
    LinkCopy,
    LinkCancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudDemoPopupScreen {
    pub generation: u64,
    pub title: String,
    /// The joined movement, mouse, jump, and inventory explanation.
    pub controls_message: String,
    pub duration_message: String,
    pub buy_label: String,
    pub later_label: String,
    pub focused: HudGameEventButton,
    pub hovered: Option<HudGameEventButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudDemoLinkConfirmationScreen {
    pub generation: u64,
    pub title: String,
    pub message: String,
    pub open_label: String,
    pub copy_label: String,
    pub cancel_label: String,
    pub focused: HudGameEventButton,
    pub hovered: Option<HudGameEventButton>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HudGameEventScreen {
    Win(HudWinScreen),
    DemoPopup(HudDemoPopupScreen),
    DemoLinkConfirmation(HudDemoLinkConfirmationScreen),
}

impl HudGameEventScreen {
    pub fn generation(&self) -> u64 {
        match self {
            Self::Win(screen) => screen.generation,
            Self::DemoPopup(screen) => screen.generation,
            Self::DemoLinkConfirmation(screen) => screen.generation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudGameEventHitTarget {
    pub generation: u64,
    pub button: HudGameEventButton,
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

#[derive(Debug, Clone, PartialEq)]
struct PopupLayout {
    content_x: i32,
    content_y: i32,
    content_height: i32,
    title_lines: Vec<Vec<HudStyledTextRun>>,
    controls_lines: Vec<Vec<HudStyledTextRun>>,
    duration_lines: Vec<Vec<HudStyledTextRun>>,
    buy: ButtonRect,
    later: ButtonRect,
}

#[derive(Debug, Clone, PartialEq)]
struct ConfirmationLayout {
    title_lines: Vec<Vec<HudStyledTextRun>>,
    message_lines: Vec<Vec<HudStyledTextRun>>,
    content_y: i32,
    open: ButtonRect,
    copy: ButtonRect,
    cancel: ButtonRect,
}

pub(super) fn sanitize_hud_game_event_screen(
    screen: HudGameEventScreen,
) -> Option<HudGameEventScreen> {
    Some(match screen {
        HudGameEventScreen::Win(mut screen) => {
            if !screen.scroll.is_finite() {
                return None;
            }
            screen.scroll = screen.scroll.max(0.0);
            if screen.lines.len() > MAX_WIN_LINES {
                screen.lines = Arc::from(&screen.lines[..MAX_WIN_LINES]);
            }
            if screen
                .lines
                .iter()
                .any(|line| runs_need_sanitizing(&line.runs))
            {
                let lines = Arc::make_mut(&mut screen.lines);
                for line in lines {
                    sanitize_runs(&mut line.runs);
                }
            }
            HudGameEventScreen::Win(screen)
        }
        HudGameEventScreen::DemoPopup(mut screen) => {
            sanitize_string(&mut screen.title);
            sanitize_string(&mut screen.controls_message);
            sanitize_string(&mut screen.duration_message);
            sanitize_string(&mut screen.buy_label);
            sanitize_string(&mut screen.later_label);
            if !matches!(
                screen.focused,
                HudGameEventButton::DemoBuy | HudGameEventButton::DemoLater
            ) {
                screen.focused = HudGameEventButton::DemoBuy;
            }
            screen.hovered = screen.hovered.filter(|button| {
                matches!(
                    button,
                    HudGameEventButton::DemoBuy | HudGameEventButton::DemoLater
                )
            });
            HudGameEventScreen::DemoPopup(screen)
        }
        HudGameEventScreen::DemoLinkConfirmation(mut screen) => {
            sanitize_string(&mut screen.title);
            sanitize_string(&mut screen.message);
            sanitize_string(&mut screen.open_label);
            sanitize_string(&mut screen.copy_label);
            sanitize_string(&mut screen.cancel_label);
            if !matches!(
                screen.focused,
                HudGameEventButton::LinkOpen
                    | HudGameEventButton::LinkCopy
                    | HudGameEventButton::LinkCancel
            ) {
                screen.focused = HudGameEventButton::LinkOpen;
            }
            screen.hovered = screen.hovered.filter(|button| {
                matches!(
                    button,
                    HudGameEventButton::LinkOpen
                        | HudGameEventButton::LinkCopy
                        | HudGameEventButton::LinkCancel
                )
            });
            HudGameEventScreen::DemoLinkConfirmation(screen)
        }
    })
}

fn sanitize_string(value: &mut String) {
    if value.chars().count() > MAX_SCREEN_TEXT_CHARS {
        *value = value.chars().take(MAX_SCREEN_TEXT_CHARS).collect();
    }
}

fn sanitize_runs(runs: &mut Vec<HudStyledTextRun>) {
    runs.truncate(MAX_SCREEN_TEXT_CHARS);
    let mut remaining = MAX_SCREEN_TEXT_CHARS;
    for run in runs {
        if run.text.chars().count() > remaining {
            run.text = run.text.chars().take(remaining).collect();
        }
        remaining = remaining.saturating_sub(run.text.chars().count());
    }
}

fn runs_need_sanitizing(runs: &[HudStyledTextRun]) -> bool {
    if runs.len() > MAX_SCREEN_TEXT_CHARS {
        return true;
    }
    let mut remaining = MAX_SCREEN_TEXT_CHARS;
    for run in runs {
        let count = run.text.chars().count();
        if count > remaining {
            return true;
        }
        remaining -= count;
    }
    false
}

pub(super) fn hud_game_event_hit_target_at(
    screen: &HudGameEventScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudGameEventHitTarget> {
    let (generation, button) = match screen {
        HudGameEventScreen::Win(_) => return None,
        HudGameEventScreen::DemoPopup(screen) => {
            let layout = popup_layout(screen, glyphs, surface_size);
            let button = [
                (HudGameEventButton::DemoBuy, layout.buy),
                (HudGameEventButton::DemoLater, layout.later),
            ]
            .into_iter()
            .find_map(|(button, rect)| rect.contains(position).then_some(button))?;
            (screen.generation, button)
        }
        HudGameEventScreen::DemoLinkConfirmation(screen) => {
            let layout = confirmation_layout(screen, glyphs, surface_size);
            let button = [
                (HudGameEventButton::LinkOpen, layout.open),
                (HudGameEventButton::LinkCopy, layout.copy),
                (HudGameEventButton::LinkCancel, layout.cancel),
            ]
            .into_iter()
            .find_map(|(button, rect)| rect.contains(position).then_some(button))?;
            (screen.generation, button)
        }
    };
    Some(HudGameEventHitTarget { generation, button })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_game_event_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    inworld_menu_background: Option<&'a HudSpriteGpu>,
    popup_background: Option<&'a HudNineSliceSprite>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    logo: Option<&'a HudSpriteGpu>,
    easter_egg_logo: Option<&'a HudSpriteGpu>,
    edition: Option<&'a HudSpriteGpu>,
    screen: &HudGameEventScreen,
) {
    match screen {
        HudGameEventScreen::Win(screen) => push_win_screen(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            logo,
            easter_egg_logo,
            edition,
            screen,
        ),
        HudGameEventScreen::DemoPopup(screen) => {
            push_game_event_menu_background(
                vertices,
                commands,
                white_pixel,
                inworld_menu_background,
                surface_size,
            );
            push_demo_popup(
                vertices,
                commands,
                white_pixel,
                popup_background,
                button_sprites,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                screen,
            );
        }
        HudGameEventScreen::DemoLinkConfirmation(screen) => {
            push_game_event_menu_background(
                vertices,
                commands,
                white_pixel,
                inworld_menu_background,
                surface_size,
            );
            push_link_confirmation(
                vertices,
                commands,
                white_pixel,
                button_sprites,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                screen,
            );
        }
    }
}

fn push_game_event_menu_background<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    inworld_menu_background: Option<&'a HudSpriteGpu>,
    surface_size: PhysicalSize<u32>,
) {
    let Some(background) = inworld_menu_background else {
        push_hud_pause_background(vertices, commands, white_pixel, surface_size);
        return;
    };
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        background,
        surface_size,
        absolute_hud_rect(
            0.0,
            0.0,
            surface_size.width.max(1),
            surface_size.height.max(1),
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [
                surface_size.width as f32 / MENU_BACKGROUND_TILE_SIZE,
                surface_size.height as f32 / MENU_BACKGROUND_TILE_SIZE,
            ],
        },
        HUD_TINT_WHITE,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_win_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    logo: Option<&'a HudSpriteGpu>,
    easter_egg_logo: Option<&'a HudSpriteGpu>,
    edition: Option<&'a HudSpriteGpu>,
    screen: &HudWinScreen,
) {
    let screen_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let screen_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let logo_x = screen_width / 2 - i32::try_from(WIN_LOGO_WIDTH / 2).unwrap_or_default();
    let logo_y = screen_height.saturating_add(WIN_LOGO_TOP_OFFSET);
    let y_offset = -screen.scroll;
    let selected_logo = if screen.show_easter_egg_logo {
        easter_egg_logo.or(logo)
    } else {
        logo
    };
    if let Some(selected_logo) = selected_logo {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            selected_logo,
            surface_size,
            absolute_hud_rect(
                logo_x as f32,
                logo_y as f32 + y_offset,
                WIN_LOGO_WIDTH,
                WIN_LOGO_HEIGHT,
            ),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, WIN_LOGO_HEIGHT as f32 / 64.0],
            },
            HUD_TINT_WHITE,
        );
    }
    if let Some(edition) = edition {
        let edition_x = screen_width / 2 - i32::try_from(WIN_EDITION_WIDTH / 2).unwrap_or_default();
        let edition_y = logo_y
            .saturating_add(i32::try_from(WIN_LOGO_HEIGHT).unwrap_or_default())
            .saturating_sub(WIN_EDITION_OVERLAP);
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            edition,
            surface_size,
            absolute_hud_rect(
                edition_x as f32,
                edition_y as f32 + y_offset,
                WIN_EDITION_WIDTH,
                WIN_EDITION_HEIGHT,
            ),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, WIN_EDITION_HEIGHT as f32 / 16.0],
            },
            HUD_TINT_WHITE,
        );
    }
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let mut line_y = logo_y.saturating_add(WIN_TEXT_TOP_OFFSET);
    let last_index = screen.lines.len().saturating_sub(1);
    for (index, line) in screen.lines.iter().enumerate() {
        let mut presented_y = line_y as f32 + y_offset;
        if index == last_index {
            let center_y = screen_height / 2 - WIN_LAST_LINE_CENTER_OFFSET;
            presented_y = presented_y.max(center_y as f32);
        }
        if presented_y + 20.0 > 0.0 && presented_y < screen_height as f32 {
            let width = hud_font_runs_width(&line.runs, glyphs).unwrap_or(0);
            let x = if line.centered {
                logo_x as f32 + (WIN_LOGO_WIDTH.saturating_sub(width) as f32 * 0.5)
            } else {
                logo_x as f32
            };
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
                    runs: &line.runs,
                    origin: (x, presented_y),
                    scale: 1.0,
                    tint: HUD_TINT_WHITE,
                    backdrop_opacity: 0.0,
                },
            );
        }
        line_y = line_y.saturating_add(WIN_TEXT_LINE_HEIGHT);
    }
}

#[allow(clippy::too_many_arguments)]
fn push_demo_popup<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    popup_background: Option<&'a HudNineSliceSprite>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDemoPopupScreen,
) {
    let layout = popup_layout(screen, glyphs, surface_size);
    let background_rect = absolute_hud_rect(
        (layout.content_x - POPUP_BORDER) as f32,
        (layout.content_y - POPUP_BORDER) as f32,
        POPUP_WIDTH as u32,
        layout
            .content_height
            .saturating_add(POPUP_BORDER.saturating_mul(2))
            .max(0) as u32,
    );
    if let Some(background) = popup_background {
        for segment in nine_slice_segments(background_rect, background.scaling) {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                &background.gpu,
                surface_size,
                segment.rect,
                segment.uv,
                HUD_TINT_WHITE,
            );
        }
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            background_rect,
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [0.08, 0.08, 0.08, 0.96],
        );
    }
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let mut y = layout.content_y;
    push_centered_lines(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        layout.content_x,
        POPUP_CONTENT_WIDTH,
        y,
        &layout.title_lines,
        HUD_TINT_WHITE,
    );
    y = y
        .saturating_add(lines_height(&layout.title_lines))
        .saturating_add(POPUP_CHILD_SPACING);
    push_centered_lines(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        layout.content_x,
        POPUP_CONTENT_WIDTH,
        y,
        &layout.controls_lines,
        HUD_TINT_WHITE,
    );
    y = y
        .saturating_add(lines_height(&layout.controls_lines))
        .saturating_add(POPUP_CHILD_SPACING);
    push_centered_lines(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        layout.content_x,
        POPUP_CONTENT_WIDTH,
        y,
        &layout.duration_lines,
        HUD_TINT_WHITE,
    );
    for (button, rect, label) in [
        (HudGameEventButton::DemoBuy, layout.buy, &screen.buy_label),
        (
            HudGameEventButton::DemoLater,
            layout.later,
            &screen.later_label,
        ),
    ] {
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

#[allow(clippy::too_many_arguments)]
fn push_link_confirmation<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDemoLinkConfirmationScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = confirmation_layout(screen, glyphs, surface_size);
    let width = surface_size.width.saturating_sub(50).max(1);
    let x = 25;
    let mut y = layout.content_y;
    push_centered_lines(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        x,
        width,
        y,
        &layout.title_lines,
        HUD_TINT_WHITE,
    );
    y = y
        .saturating_add(lines_height(&layout.title_lines))
        .saturating_add(CONFIRM_CHILD_SPACING);
    push_centered_lines(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        x,
        width,
        y,
        &layout.message_lines,
        HUD_TINT_WHITE,
    );
    for (button, rect, label) in [
        (
            HudGameEventButton::LinkOpen,
            layout.open,
            &screen.open_label,
        ),
        (
            HudGameEventButton::LinkCopy,
            layout.copy,
            &screen.copy_label,
        ),
        (
            HudGameEventButton::LinkCancel,
            layout.cancel,
            &screen.cancel_label,
        ),
    ] {
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

#[allow(clippy::too_many_arguments)]
fn push_centered_lines<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    x: i32,
    width: u32,
    y: i32,
    lines: &[Vec<HudStyledTextRun>],
    tint: [f32; 4],
) {
    for (index, line) in lines.iter().enumerate() {
        let line_width = hud_font_runs_width(line, glyphs).unwrap_or(0);
        let origin_x = x as f32 + width.saturating_sub(line_width) as f32 * 0.5;
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
                runs: line,
                origin: (
                    origin_x,
                    y.saturating_add(index as i32 * FONT_LINE_HEIGHT) as f32,
                ),
                scale: 1.0,
                tint,
                backdrop_opacity: 0.0,
            },
        );
    }
}

fn popup_layout(
    screen: &HudDemoPopupScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> PopupLayout {
    let title_lines = wrap_text(&screen.title, POPUP_CONTENT_WIDTH, glyphs, true, 0);
    let controls_lines = wrap_text(
        &screen.controls_message,
        POPUP_CONTENT_WIDTH,
        glyphs,
        false,
        0,
    );
    let duration_lines = wrap_text(
        &screen.duration_message,
        POPUP_CONTENT_WIDTH,
        glyphs,
        false,
        0,
    );
    let content_height = lines_height(&title_lines)
        .saturating_add(POPUP_CHILD_SPACING)
        .saturating_add(lines_height(&controls_lines))
        .saturating_add(POPUP_CHILD_SPACING)
        .saturating_add(lines_height(&duration_lines))
        .saturating_add(POPUP_CHILD_SPACING)
        .saturating_add(POPUP_BUTTON_HEIGHT);
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let content_x = surface_width / 2 - i32::try_from(POPUP_CONTENT_WIDTH / 2).unwrap_or_default();
    let content_y = surface_height / 2 - content_height / 2;
    let button_y = content_y
        .saturating_add(content_height)
        .saturating_sub(POPUP_BUTTON_HEIGHT);
    PopupLayout {
        content_x,
        content_y,
        content_height,
        title_lines,
        controls_lines,
        duration_lines,
        buy: ButtonRect {
            x: content_x,
            y: button_y,
            width: POPUP_BUTTON_WIDTH,
            height: POPUP_BUTTON_HEIGHT,
        },
        later: ButtonRect {
            x: content_x
                .saturating_add(POPUP_BUTTON_WIDTH)
                .saturating_add(POPUP_BUTTON_GAP),
            y: button_y,
            width: POPUP_BUTTON_WIDTH,
            height: POPUP_BUTTON_HEIGHT,
        },
    }
}

fn confirmation_layout(
    screen: &HudDemoLinkConfirmationScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> ConfirmationLayout {
    let max_width = surface_size.width.saturating_sub(50).max(1);
    let title_lines = wrap_text(&screen.title, max_width, glyphs, false, 1);
    let mut message_lines = wrap_text(&screen.message, max_width, glyphs, false, 15);
    message_lines.truncate(15);
    let content_height = lines_height(&title_lines)
        .saturating_add(CONFIRM_CHILD_SPACING)
        .saturating_add(lines_height(&message_lines))
        .saturating_add(CONFIRM_CHILD_SPACING)
        .saturating_add(CONFIRM_BUTTON_TOP_PADDING)
        .saturating_add(CONFIRM_BUTTON_HEIGHT);
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let content_y = surface_height / 2 - content_height / 2;
    let buttons_width = CONFIRM_BUTTON_WIDTH
        .saturating_mul(3)
        .saturating_add(CONFIRM_BUTTON_GAP.saturating_mul(2));
    let button_x = surface_width / 2 - buttons_width / 2;
    let button_y = content_y
        .saturating_add(content_height)
        .saturating_sub(CONFIRM_BUTTON_HEIGHT);
    let rect = |index: i32| ButtonRect {
        x: button_x.saturating_add(
            index.saturating_mul(CONFIRM_BUTTON_WIDTH.saturating_add(CONFIRM_BUTTON_GAP)),
        ),
        y: button_y,
        width: CONFIRM_BUTTON_WIDTH,
        height: CONFIRM_BUTTON_HEIGHT,
    };
    ConfirmationLayout {
        title_lines,
        message_lines,
        content_y,
        open: rect(0),
        copy: rect(1),
        cancel: rect(2),
    }
}

fn wrap_text(
    text: &str,
    max_width: u32,
    glyphs: &HudFontGlyphMap,
    bold: bool,
    max_rows: usize,
) -> Vec<Vec<HudStyledTextRun>> {
    let runs = [HudStyledTextRun {
        text: text.to_string(),
        style: HudTextStyle {
            bold,
            ..HudTextStyle::default()
        },
        color: None,
    }];
    let mut lines = hud_wrap_chat_runs_with_interactions(&runs, &[], max_width.max(1), glyphs)
        .into_iter()
        .map(|line| line.runs)
        .collect::<Vec<_>>();
    if max_rows != 0 {
        lines.truncate(max_rows);
    }
    lines
}

fn lines_height(lines: &[Vec<HudStyledTextRun>]) -> i32 {
    i32::try_from(lines.len())
        .unwrap_or(i32::MAX)
        .saturating_mul(FONT_LINE_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::HudAsciiGlyph;

    fn glyphs_for(text: &str) -> HudFontGlyphMap {
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
        glyphs
    }

    fn popup() -> HudDemoPopupScreen {
        HudDemoPopupScreen {
            generation: 7,
            title: "Minecraft Demo Mode".to_string(),
            controls_message: "Move with W, A, S, D\nLook around\nJump\nInventory".to_string(),
            duration_message: "Five days".to_string(),
            buy_label: "Purchase Now!".to_string(),
            later_label: "Continue Playing!".to_string(),
            focused: HudGameEventButton::DemoBuy,
            hovered: None,
        }
    }

    #[test]
    fn hud_demo_popup_uses_vanilla_250_width_and_shared_button_hit_rects() {
        let screen = popup();
        let glyphs =
            glyphs_for("Minecraft Demo ModeMove with W, A, S, DLook aroundJumpInventoryFive days");
        let size = PhysicalSize::new(800, 600);
        let layout = popup_layout(&screen, &glyphs, size);
        assert_eq!(layout.content_x - POPUP_BORDER, 275);
        assert_eq!(layout.buy.width, 104);
        assert_eq!(layout.later.x - layout.buy.x, 110);
        assert_eq!(
            hud_game_event_hit_target_at(
                &HudGameEventScreen::DemoPopup(screen),
                &glyphs,
                size,
                (f64::from(layout.later.x + 1), f64::from(layout.later.y + 1)),
            ),
            Some(HudGameEventHitTarget {
                generation: 7,
                button: HudGameEventButton::DemoLater,
            })
        );
    }

    #[test]
    fn hud_demo_popup_trusted_link_confirmation_centers_three_100_pixel_buttons() {
        let screen = HudDemoLinkConfirmationScreen {
            generation: 9,
            title: "Open Link?".to_string(),
            message: "Do you want to open this link or copy it to your clipboard?".to_string(),
            open_label: "Open in Browser".to_string(),
            copy_label: "Copy to Clipboard".to_string(),
            cancel_label: "Cancel".to_string(),
            focused: HudGameEventButton::LinkOpen,
            hovered: None,
        };
        let glyphs =
            glyphs_for("Open Link?Do you want to open this link or copy it to your clipboard?");
        let size = PhysicalSize::new(800, 600);
        let layout = confirmation_layout(&screen, &glyphs, size);
        assert_eq!(layout.open.x, 246);
        assert_eq!(layout.copy.x, 350);
        assert_eq!(layout.cancel.x, 454);
        assert_eq!(
            hud_game_event_hit_target_at(
                &HudGameEventScreen::DemoLinkConfirmation(screen),
                &glyphs,
                size,
                (455.0, f64::from(layout.cancel.y + 1)),
            )
            .map(|target| target.button),
            Some(HudGameEventButton::LinkCancel)
        );
    }

    #[test]
    fn hud_win_screen_last_line_never_rises_above_screen_center() {
        let center = 600 / 2 - WIN_LAST_LINE_CENTER_OFFSET;
        let line_y = 600 + WIN_LOGO_TOP_OFFSET + WIN_TEXT_TOP_OFFSET;
        assert!((line_y as f32 - 10_000.0).max(center as f32) == center as f32);
    }

    #[test]
    fn hud_win_screen_sanitization_preserves_unchanged_shared_lines() {
        let lines: Arc<[HudWinScreenLine]> = vec![HudWinScreenLine {
            runs: vec![HudStyledTextRun::plain("credits")],
            centered: true,
        }]
        .into();
        let screen = sanitize_hud_game_event_screen(HudGameEventScreen::Win(HudWinScreen {
            generation: 1,
            scroll: 0.0,
            show_easter_egg_logo: false,
            lines: Arc::clone(&lines),
        }))
        .expect("valid win screen");
        let HudGameEventScreen::Win(screen) = screen else {
            panic!("win screen");
        };
        assert!(Arc::ptr_eq(&screen.lines, &lines));
    }

    #[test]
    fn hud_demo_popup_title_wrap_uses_bold_glyph_advance() {
        let glyphs = glyphs_for("a");
        let normal = wrap_text(&"a".repeat(35), POPUP_CONTENT_WIDTH, &glyphs, false, 0);
        let bold = wrap_text(&"a".repeat(35), POPUP_CONTENT_WIDTH, &glyphs, true, 0);
        assert_eq!(normal.len(), 1);
        assert_eq!(bold.len(), 2);
    }

    #[test]
    fn hud_demo_popup_link_confirmation_omits_untrusted_warning_color() {
        assert_eq!(
            super::super::hud_argb_to_tint(0xFFFFCCCC),
            [1.0, 0.8, 0.8, 1.0]
        );
    }
}
