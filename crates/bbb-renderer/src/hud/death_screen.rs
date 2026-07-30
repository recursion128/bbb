//! Vanilla death, exit-confirmation, and trusted-link-confirmation HUD.
//!
//! This module owns the shared draw/hit layout. Native input receives only
//! generation-tagged targets produced from the same rectangles and rich-text
//! runs that are rendered here.

use winit::dpi::PhysicalSize;

use super::{
    absolute_hud_rect, hud_component_runs_width, hud_font_runs_width, hud_quad_vertices,
    hud_wrap_chat_runs_with_interactions, push_hud_component_screen_text_draw,
    push_hud_debug_options_button, push_hud_screen_text_draw, HudComponentScreenTextDraw,
    HudComponentTextRun, HudDebugOptionsButtonSprites, HudDrawCommand, HudFontGlyphBook,
    HudFontGlyphMap, HudObfuscatedGlyphPool, HudScreenTextDraw, HudSpriteGpu, HudStyledTextRun,
    HudTextStyle, HudUvRect, HudVertex, HUD_TINT_WHITE,
};

const MAIN_BUTTON_WIDTH: i32 = 200;
const BUTTON_HEIGHT: i32 = 20;
const MAIN_RESPAWN_Y_OFFSET: i32 = 72;
const MAIN_TITLE_Y_OFFSET: i32 = 96;
const TITLE_Y: f32 = 30.0;
const CAUSE_Y: f32 = 85.0;
const SCORE_Y: f32 = 100.0;
const FONT_LINE_HEIGHT: i32 = 9;
const TITLE_SCALE: f32 = 2.0;
const QUIT_BUTTON_WIDTH: i32 = 150;
const LINK_BUTTON_WIDTH: i32 = 100;
const CONFIRM_BUTTON_GAP: i32 = 4;
const CONFIRM_CHILD_SPACING: i32 = 8;
const CONFIRM_BUTTON_TOP_PADDING: i32 = 16;
const MAX_SCREEN_TEXT_CHARS: usize = 65_536;
const DEATH_BACKGROUND_TOP: [f32; 4] = [80.0 / 255.0, 0.0, 0.0, 96.0 / 255.0];
const DEATH_BACKGROUND_BOTTOM: [f32; 4] =
    [128.0 / 255.0, 48.0 / 255.0, 48.0 / 255.0, 160.0 / 255.0];
const DISABLED_TEXT_TINT: [f32; 4] = [0.63, 0.63, 0.63, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDeathScreenButton {
    Respawn,
    TitleScreen,
    QuitTitleScreen,
    QuitRespawn,
    LinkOpen,
    LinkCopy,
    LinkCancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDeathScreenHitTargetKind {
    Button(HudDeathScreenButton),
    CauseLink { run_index: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudDeathScreenHitTarget {
    pub generation: u64,
    pub kind: HudDeathScreenHitTargetKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudDeathMainScreen {
    pub generation: u64,
    pub title: String,
    pub cause_runs: Vec<HudComponentTextRun>,
    /// Canonical component-run index for each visual cause run. Non-links are
    /// `None`; native input revalidates every `Some` index before acting.
    pub cause_link_run_indices: Vec<Option<u16>>,
    pub score_runs: Vec<HudComponentTextRun>,
    pub respawn_label: String,
    pub title_screen_label: String,
    pub buttons_active: bool,
    pub focused: HudDeathScreenButton,
    pub hovered: Option<HudDeathScreenButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudDeathQuitConfirmationScreen {
    pub generation: u64,
    pub title: String,
    pub title_screen_label: String,
    pub respawn_label: String,
    pub buttons_active: bool,
    pub focused: HudDeathScreenButton,
    pub hovered: Option<HudDeathScreenButton>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudDeathLinkConfirmationScreen {
    pub generation: u64,
    pub title: String,
    pub message: String,
    pub open_label: String,
    pub copy_label: String,
    pub cancel_label: String,
    pub focused: HudDeathScreenButton,
    pub hovered: Option<HudDeathScreenButton>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HudDeathScreen {
    Main(HudDeathMainScreen),
    QuitConfirmation(HudDeathQuitConfirmationScreen),
    LinkConfirmation(HudDeathLinkConfirmationScreen),
}

impl HudDeathScreen {
    pub fn generation(&self) -> u64 {
        match self {
            Self::Main(screen) => screen.generation,
            Self::QuitConfirmation(screen) => screen.generation,
            Self::LinkConfirmation(screen) => screen.generation,
        }
    }
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
struct ConfirmationLayout {
    title_lines: Vec<Vec<HudStyledTextRun>>,
    message_lines: Vec<Vec<HudStyledTextRun>>,
    content_y: i32,
    buttons: Vec<ButtonRect>,
}

pub(super) fn sanitize_hud_death_screen(screen: HudDeathScreen) -> Option<HudDeathScreen> {
    Some(match screen {
        HudDeathScreen::Main(mut screen) => {
            sanitize_string(&mut screen.title);
            sanitize_component_runs(&mut screen.cause_runs);
            screen
                .cause_link_run_indices
                .truncate(screen.cause_runs.len());
            screen
                .cause_link_run_indices
                .resize(screen.cause_runs.len(), None);
            sanitize_component_runs(&mut screen.score_runs);
            sanitize_string(&mut screen.respawn_label);
            sanitize_string(&mut screen.title_screen_label);
            if !matches!(
                screen.focused,
                HudDeathScreenButton::Respawn | HudDeathScreenButton::TitleScreen
            ) {
                screen.focused = HudDeathScreenButton::Respawn;
            }
            screen.hovered = screen.hovered.filter(|button| {
                matches!(
                    button,
                    HudDeathScreenButton::Respawn | HudDeathScreenButton::TitleScreen
                )
            });
            HudDeathScreen::Main(screen)
        }
        HudDeathScreen::QuitConfirmation(mut screen) => {
            sanitize_string(&mut screen.title);
            sanitize_string(&mut screen.title_screen_label);
            sanitize_string(&mut screen.respawn_label);
            if !matches!(
                screen.focused,
                HudDeathScreenButton::QuitTitleScreen | HudDeathScreenButton::QuitRespawn
            ) {
                screen.focused = HudDeathScreenButton::QuitTitleScreen;
            }
            screen.hovered = screen.hovered.filter(|button| {
                matches!(
                    button,
                    HudDeathScreenButton::QuitTitleScreen | HudDeathScreenButton::QuitRespawn
                )
            });
            HudDeathScreen::QuitConfirmation(screen)
        }
        HudDeathScreen::LinkConfirmation(mut screen) => {
            sanitize_string(&mut screen.title);
            sanitize_string(&mut screen.message);
            sanitize_string(&mut screen.open_label);
            sanitize_string(&mut screen.copy_label);
            sanitize_string(&mut screen.cancel_label);
            if !matches!(
                screen.focused,
                HudDeathScreenButton::LinkOpen
                    | HudDeathScreenButton::LinkCopy
                    | HudDeathScreenButton::LinkCancel
            ) {
                screen.focused = HudDeathScreenButton::LinkOpen;
            }
            screen.hovered = screen.hovered.filter(|button| {
                matches!(
                    button,
                    HudDeathScreenButton::LinkOpen
                        | HudDeathScreenButton::LinkCopy
                        | HudDeathScreenButton::LinkCancel
                )
            });
            HudDeathScreen::LinkConfirmation(screen)
        }
    })
}

fn sanitize_string(value: &mut String) {
    if value.chars().count() > MAX_SCREEN_TEXT_CHARS {
        *value = value.chars().take(MAX_SCREEN_TEXT_CHARS).collect();
    }
}

fn sanitize_component_runs(runs: &mut Vec<HudComponentTextRun>) {
    runs.truncate(MAX_SCREEN_TEXT_CHARS);
    let mut remaining = MAX_SCREEN_TEXT_CHARS;
    for run in runs {
        if run.visual.text.chars().count() > remaining {
            run.visual.text = run.visual.text.chars().take(remaining).collect();
        }
        remaining = remaining.saturating_sub(run.visual.text.chars().count());
    }
}

pub(super) fn hud_death_screen_hit_target_at(
    screen: &HudDeathScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudDeathScreenHitTarget> {
    let glyphs = fonts.glyphs(None);
    let generation = screen.generation();
    let kind = match screen {
        HudDeathScreen::Main(screen) => {
            if let Some(run_index) = cause_link_at(screen, fonts, surface_size, position) {
                HudDeathScreenHitTargetKind::CauseLink { run_index }
            } else {
                if !screen.buttons_active {
                    return None;
                }
                let (respawn, title) = main_button_rects(surface_size);
                let button = [
                    (HudDeathScreenButton::Respawn, respawn),
                    (HudDeathScreenButton::TitleScreen, title),
                ]
                .into_iter()
                .find_map(|(button, rect)| rect.contains(position).then_some(button))?;
                HudDeathScreenHitTargetKind::Button(button)
            }
        }
        HudDeathScreen::QuitConfirmation(screen) => {
            if !screen.buttons_active {
                return None;
            }
            let layout = quit_confirmation_layout(screen, glyphs, surface_size);
            let button = [
                HudDeathScreenButton::QuitTitleScreen,
                HudDeathScreenButton::QuitRespawn,
            ]
            .into_iter()
            .zip(layout.buttons)
            .find_map(|(button, rect)| rect.contains(position).then_some(button))?;
            HudDeathScreenHitTargetKind::Button(button)
        }
        HudDeathScreen::LinkConfirmation(screen) => {
            let layout = link_confirmation_layout(screen, glyphs, surface_size);
            let button = [
                HudDeathScreenButton::LinkOpen,
                HudDeathScreenButton::LinkCopy,
                HudDeathScreenButton::LinkCancel,
            ]
            .into_iter()
            .zip(layout.buttons)
            .find_map(|(button, rect)| rect.contains(position).then_some(button))?;
            HudDeathScreenHitTargetKind::Button(button)
        }
    };
    Some(HudDeathScreenHitTarget { generation, kind })
}

fn cause_link_at(
    screen: &HudDeathMainScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<u16> {
    if position.1 < f64::from(CAUSE_Y) || position.1 >= f64::from(CAUSE_Y + 9.0) {
        return None;
    }
    let total_width = hud_component_runs_width(&screen.cause_runs, fonts)?;
    let mut x = surface_size.width.max(1) as f64 * 0.5 - f64::from(total_width) * 0.5;
    for (run, run_index) in screen
        .cause_runs
        .iter()
        .zip(screen.cause_link_run_indices.iter().copied())
    {
        let width = hud_component_runs_width(std::slice::from_ref(run), fonts)?;
        let next_x = x + f64::from(width);
        if position.0 >= x && position.0 < next_x {
            return run_index;
        }
        x = next_x;
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_death_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    fonts: &HudFontGlyphBook,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDeathScreen,
) {
    push_death_background(vertices, commands, white_pixel, surface_size);
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let glyphs = fonts.glyphs(None);
    match screen {
        HudDeathScreen::Main(screen) => push_main(
            vertices,
            commands,
            white_pixel,
            button_sprites,
            font_atlas,
            fonts,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            screen,
        ),
        HudDeathScreen::QuitConfirmation(screen) => push_quit_confirmation(
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
        ),
        HudDeathScreen::LinkConfirmation(screen) => push_link_confirmation(
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
        ),
    }
}

fn push_death_background<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
) {
    let start = vertices.len() as u32;
    let mut quad = hud_quad_vertices(
        surface_size,
        absolute_hud_rect(
            0.0,
            0.0,
            surface_size.width.max(1),
            surface_size.height.max(1),
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        DEATH_BACKGROUND_TOP,
    );
    for index in [2, 4, 5] {
        quad[index].tint = DEATH_BACKGROUND_BOTTOM;
    }
    vertices.extend_from_slice(&quad);
    commands.push(HudDrawCommand::Sprite {
        sprite: white_pixel,
        start,
        end: vertices.len() as u32,
    });
}

#[allow(clippy::too_many_arguments)]
fn push_main<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDeathMainScreen,
) {
    let title_runs = [HudStyledTextRun::plain(&screen.title)];
    push_centered_runs(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &title_runs,
        TITLE_Y,
        TITLE_SCALE,
    );
    push_centered_component_runs(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        fonts,
        frame_index,
        surface_size,
        &screen.cause_runs,
        CAUSE_Y,
        1.0,
    );
    push_centered_component_runs(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        fonts,
        frame_index,
        surface_size,
        &screen.score_runs,
        SCORE_Y,
        1.0,
    );
    let (respawn, title) = main_button_rects(surface_size);
    for (button, rect, label) in [
        (
            HudDeathScreenButton::Respawn,
            respawn,
            screen.respawn_label.as_str(),
        ),
        (
            HudDeathScreenButton::TitleScreen,
            title,
            screen.title_screen_label.as_str(),
        ),
    ] {
        push_button(
            vertices,
            commands,
            white_pixel,
            button_sprites,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            rect,
            label,
            screen.buttons_active,
            screen.hovered == Some(button) || screen.focused == button,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_quit_confirmation<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDeathQuitConfirmationScreen,
) {
    let layout = quit_confirmation_layout(screen, glyphs, surface_size);
    push_confirmation_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &layout,
    );
    for (button, rect, label) in [
        (
            HudDeathScreenButton::QuitTitleScreen,
            layout.buttons[0],
            screen.title_screen_label.as_str(),
        ),
        (
            HudDeathScreenButton::QuitRespawn,
            layout.buttons[1],
            screen.respawn_label.as_str(),
        ),
    ] {
        push_button(
            vertices,
            commands,
            white_pixel,
            button_sprites,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            rect,
            label,
            screen.buttons_active,
            screen.hovered == Some(button) || screen.focused == button,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_link_confirmation<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudDeathLinkConfirmationScreen,
) {
    let layout = link_confirmation_layout(screen, glyphs, surface_size);
    push_confirmation_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &layout,
    );
    for (index, (button, label)) in [
        (HudDeathScreenButton::LinkOpen, screen.open_label.as_str()),
        (HudDeathScreenButton::LinkCopy, screen.copy_label.as_str()),
        (
            HudDeathScreenButton::LinkCancel,
            screen.cancel_label.as_str(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        push_button(
            vertices,
            commands,
            white_pixel,
            button_sprites,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            layout.buttons[index],
            label,
            true,
            screen.hovered == Some(button) || screen.focused == button,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_confirmation_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    layout: &ConfirmationLayout,
) {
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
        y,
        &layout.title_lines,
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
        y,
        &layout.message_lines,
    );
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
    y: i32,
    lines: &[Vec<HudStyledTextRun>],
) {
    for (index, line) in lines.iter().enumerate() {
        push_centered_runs(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            line,
            y.saturating_add(index as i32 * FONT_LINE_HEIGHT) as f32,
            1.0,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_centered_runs<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    y: f32,
    scale: f32,
) {
    if runs.is_empty() {
        return;
    }
    let width = hud_font_runs_width(runs, glyphs).unwrap_or(0) as f32 * scale;
    let x = surface_size.width.max(1) as f32 * 0.5 - width * 0.5;
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
            runs,
            origin: (x, y),
            scale,
            tint: HUD_TINT_WHITE,
            backdrop_opacity: 0.0,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn push_centered_component_runs<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudComponentTextRun],
    y: f32,
    scale: f32,
) {
    if runs.is_empty() {
        return;
    }
    let width = hud_component_runs_width(runs, fonts).unwrap_or(0) as f32 * scale;
    let x = surface_size.width.max(1) as f32 * 0.5 - width * 0.5;
    push_hud_component_screen_text_draw(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        fonts,
        frame_index,
        surface_size,
        &HudComponentScreenTextDraw {
            runs,
            origin: (x, y),
            scale,
            tint: HUD_TINT_WHITE,
            backdrop_opacity: 0.0,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn push_button<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    rect: ButtonRect,
    label: &str,
    active: bool,
    highlighted: bool,
) {
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
        active,
        active && highlighted,
        if active {
            HUD_TINT_WHITE
        } else {
            DISABLED_TEXT_TINT
        },
        button_sprites,
    );
}

fn main_button_rects(surface_size: PhysicalSize<u32>) -> (ButtonRect, ButtonRect) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let x = width / 2 - MAIN_BUTTON_WIDTH / 2;
    let rect = |y_offset| ButtonRect {
        x,
        y: height / 4 + y_offset,
        width: MAIN_BUTTON_WIDTH,
        height: BUTTON_HEIGHT,
    };
    (rect(MAIN_RESPAWN_Y_OFFSET), rect(MAIN_TITLE_Y_OFFSET))
}

fn quit_confirmation_layout(
    screen: &HudDeathQuitConfirmationScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> ConfirmationLayout {
    confirmation_layout(
        &screen.title,
        "",
        QUIT_BUTTON_WIDTH,
        2,
        glyphs,
        surface_size,
    )
}

fn link_confirmation_layout(
    screen: &HudDeathLinkConfirmationScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> ConfirmationLayout {
    confirmation_layout(
        &screen.title,
        &screen.message,
        LINK_BUTTON_WIDTH,
        3,
        glyphs,
        surface_size,
    )
}

fn confirmation_layout(
    title: &str,
    message: &str,
    button_width: i32,
    button_count: i32,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> ConfirmationLayout {
    let max_width = surface_size.width.saturating_sub(50).max(1);
    let title_lines = wrap_text(title, max_width, glyphs, 1);
    let mut message_lines = wrap_text(message, max_width, glyphs, 15);
    message_lines.truncate(15);
    let content_height = lines_height(&title_lines)
        .saturating_add(CONFIRM_CHILD_SPACING)
        .saturating_add(lines_height(&message_lines))
        .saturating_add(CONFIRM_CHILD_SPACING)
        .saturating_add(CONFIRM_BUTTON_TOP_PADDING)
        .saturating_add(BUTTON_HEIGHT);
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let content_y = surface_height / 2 - content_height / 2;
    let buttons_width = button_width
        .saturating_mul(button_count)
        .saturating_add(CONFIRM_BUTTON_GAP.saturating_mul(button_count.saturating_sub(1)));
    let button_x = surface_width / 2 - buttons_width / 2;
    let button_y = content_y
        .saturating_add(content_height)
        .saturating_sub(BUTTON_HEIGHT);
    let buttons = (0..button_count)
        .map(|index| ButtonRect {
            x: button_x.saturating_add(
                index.saturating_mul(button_width.saturating_add(CONFIRM_BUTTON_GAP)),
            ),
            y: button_y,
            width: button_width,
            height: BUTTON_HEIGHT,
        })
        .collect();
    ConfirmationLayout {
        title_lines,
        message_lines,
        content_y,
        buttons,
    }
}

fn wrap_text(
    text: &str,
    max_width: u32,
    glyphs: &HudFontGlyphMap,
    max_rows: usize,
) -> Vec<Vec<HudStyledTextRun>> {
    if text.is_empty() {
        return Vec::new();
    }
    let runs = [HudStyledTextRun {
        text: text.to_string(),
        style: HudTextStyle::default(),
        color: None,
    }];
    let mut lines = hud_wrap_chat_runs_with_interactions(&runs, &[], max_width.max(1), glyphs)
        .into_iter()
        .map(|line| line.runs)
        .collect::<Vec<_>>();
    lines.truncate(max_rows);
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

    fn main_screen(buttons_active: bool) -> HudDeathScreen {
        HudDeathScreen::Main(HudDeathMainScreen {
            generation: 7,
            title: "You Died!".to_string(),
            cause_runs: vec![
                HudComponentTextRun::plain("fell "),
                HudComponentTextRun::plain("here"),
            ],
            cause_link_run_indices: vec![None, Some(4)],
            score_runs: vec![HudComponentTextRun::plain("Score: 9")],
            respawn_label: "Respawn".to_string(),
            title_screen_label: "Title Screen".to_string(),
            buttons_active,
            focused: HudDeathScreenButton::Respawn,
            hovered: None,
        })
    }

    #[test]
    fn hud_death_screen_main_uses_vanilla_button_rects_and_activation_delay() {
        let size = PhysicalSize::new(320, 240);
        let fonts = HudFontGlyphBook::from(glyphs_for("fell here"));
        let inactive = main_screen(false);
        assert_eq!(
            hud_death_screen_hit_target_at(&inactive, &fonts, size, (160.0, 137.0)),
            None
        );
        let active = main_screen(true);
        assert_eq!(
            hud_death_screen_hit_target_at(&active, &fonts, size, (160.0, 137.0)),
            Some(HudDeathScreenHitTarget {
                generation: 7,
                kind: HudDeathScreenHitTargetKind::Button(HudDeathScreenButton::Respawn),
            })
        );
        assert_eq!(
            hud_death_screen_hit_target_at(&active, &fonts, size, (160.0, 161.0)),
            Some(HudDeathScreenHitTarget {
                generation: 7,
                kind: HudDeathScreenHitTargetKind::Button(HudDeathScreenButton::TitleScreen),
            })
        );
    }

    #[test]
    fn hud_death_screen_cause_link_uses_centered_visual_run_and_canonical_index() {
        let size = PhysicalSize::new(320, 240);
        let default = glyphs_for("fell here");
        let mut wide = HudFontGlyphMap::new();
        for ch in "here".chars() {
            wide.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    width: 9,
                    height: 8,
                    advance: 10,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            std::collections::BTreeMap::from([
                ("example:wide".to_string(), wide),
                ("minecraft:default".to_string(), default),
            ]),
        );
        let mut screen = main_screen(false);
        let HudDeathScreen::Main(main) = &mut screen else {
            unreachable!()
        };
        main.cause_runs[1].font = Some("example:wide".to_string());
        // Total width 70, first run 30: the custom-font linked run starts at x=155.
        assert_eq!(
            hud_death_screen_hit_target_at(&screen, &fonts, size, (158.0, 88.0)),
            Some(HudDeathScreenHitTarget {
                generation: 7,
                kind: HudDeathScreenHitTargetKind::CauseLink { run_index: 4 },
            })
        );
        assert_eq!(
            hud_death_screen_hit_target_at(&screen, &fonts, size, (150.0, 88.0)),
            None
        );
    }

    #[test]
    fn hud_death_screen_confirmation_layouts_use_vanilla_button_widths() {
        let size = PhysicalSize::new(400, 300);
        let glyphs = glyphs_for("Are you sure? open copy cancel");
        let quit = HudDeathQuitConfirmationScreen {
            generation: 1,
            title: "Are you sure?".to_string(),
            title_screen_label: "Title Screen".to_string(),
            respawn_label: "Respawn".to_string(),
            buttons_active: true,
            focused: HudDeathScreenButton::QuitTitleScreen,
            hovered: None,
        };
        let quit_layout = quit_confirmation_layout(&quit, &glyphs, size);
        assert_eq!(quit_layout.buttons.len(), 2);
        assert!(quit_layout.buttons.iter().all(|rect| rect.width == 150));
        assert_eq!(quit_layout.buttons[1].x - quit_layout.buttons[0].x, 154);

        let link = HudDeathLinkConfirmationScreen {
            generation: 2,
            title: "Open?".to_string(),
            message: "url".to_string(),
            open_label: "open".to_string(),
            copy_label: "copy".to_string(),
            cancel_label: "cancel".to_string(),
            focused: HudDeathScreenButton::LinkOpen,
            hovered: None,
        };
        let link_layout = link_confirmation_layout(&link, &glyphs, size);
        assert_eq!(link_layout.buttons.len(), 3);
        assert!(link_layout.buttons.iter().all(|rect| rect.width == 100));
        assert_eq!(link_layout.buttons[1].x - link_layout.buttons[0].x, 104);
    }
}
