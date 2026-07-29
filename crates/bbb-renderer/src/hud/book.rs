use std::collections::HashMap;

use winit::dpi::PhysicalSize;

use super::{
    absolute_hud_rect, hud_component_text_pass_geometry, hud_font_glyph, hud_font_runs_width,
    hud_logical_font_runs_width, hud_logical_styled_text_pass_geometry, hud_rich_tooltip_lines,
    hud_styled_text_pass_geometry, hud_wrap_chat_runs_with_interactions,
    hud_wrap_component_runs_with_interactions, push_hud_component_tooltip,
    push_hud_debug_tinted_rect, push_hud_draw_with_uv_and_tint, push_hud_pause_button,
    push_hud_styled_text_pass, sanitize_hud_component_runs, sanitize_hud_rich_tooltip,
    sanitize_hud_text_preserving_empty, HudChatInteraction, HudComponentTextRun, HudDrawCommand,
    HudFontGlyphBook, HudFontGlyphMap, HudNineSliceSprite, HudObfuscatedGlyphPool, HudRichTooltip,
    HudSpriteGpu, HudStyledTextRun, HudTooltipStyleSprites, HudUvRect, HudVertex, HUD_TINT_WHITE,
};

pub const HUD_BOOK_WIDTH: i32 = 192;
pub const HUD_BOOK_HEIGHT: i32 = 192;
pub const HUD_BOOK_TOP: i32 = 2;
pub const HUD_BOOK_TEXT_X: i32 = 36;
pub const HUD_BOOK_TEXT_Y: i32 = 30;
pub const HUD_BOOK_TEXT_WIDTH: u32 = 114;
pub const HUD_BOOK_LINE_HEIGHT: i32 = 9;
pub const HUD_BOOK_MAX_LINES: usize = 14;
pub const HUD_BOOK_PAGE_INDICATOR_RIGHT: i32 = 148;
pub const HUD_BOOK_PAGE_INDICATOR_Y: i32 = 16;
pub const HUD_BOOK_PAGE_BUTTON_Y: i32 = 157;
pub const HUD_BOOK_PAGE_BACK_X: i32 = 43;
pub const HUD_BOOK_PAGE_FORWARD_X: i32 = 116;
pub const HUD_BOOK_PAGE_BUTTON_WIDTH: i32 = 23;
pub const HUD_BOOK_PAGE_BUTTON_HEIGHT: i32 = 13;

const HUD_BOOK_MENU_Y: i32 = 194;
const HUD_BOOK_MENU_LEFT_X: i32 = -4;
const HUD_BOOK_MENU_RIGHT_X: i32 = 98;
const HUD_BOOK_MENU_FULL_WIDTH: i32 = 200;
const HUD_BOOK_MENU_HALF_WIDTH: i32 = 98;
const HUD_BOOK_MENU_HEIGHT: i32 = 20;
const HUD_BOOK_EDIT_TEXT_X: i32 = 35;
const HUD_BOOK_SIGN_TITLE_Y: i32 = 50;
const HUD_BOOK_TEXT_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const HUD_BOOK_SELECTION_TINT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudBookMode {
    View,
    Edit,
    Sign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudBookScreen {
    pub generation: u64,
    pub page: usize,
    pub page_count: usize,
    pub resource_generation: u64,
    pub resources_loaded: bool,
    pub advanced_item_tooltips: bool,
    pub mode: HudBookMode,
    pub runs: Vec<HudComponentTextRun>,
    pub interactions: Vec<HudBookInteraction>,
    pub cursor: usize,
    pub selection: usize,
    pub cursor_visible: bool,
    pub cursor_position: Option<(i32, i32)>,
    pub page_indicator: String,
    pub title: String,
    pub author: String,
    pub warning: String,
    pub done: String,
    pub sign: String,
    pub finalize: String,
    pub cancel: String,
    pub finalize_enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudBookInteraction {
    pub start_char: usize,
    pub end_char: usize,
    pub source_run_index: u16,
    pub clickable: bool,
    pub tooltip: Option<HudRichTooltip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudBookHitTarget {
    pub generation: u64,
    pub page: usize,
    pub resource_generation: u64,
    pub resources_loaded: bool,
    pub source_run_index: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudBookLayout {
    pub origin_x: i32,
    pub origin_y: i32,
}

pub fn hud_book_layout(surface_size: PhysicalSize<u32>) -> HudBookLayout {
    HudBookLayout {
        origin_x: i32::try_from(surface_size.width)
            .unwrap_or(i32::MAX)
            .saturating_sub(HUD_BOOK_WIDTH)
            / 2,
        origin_y: HUD_BOOK_TOP,
    }
}

pub(super) fn sanitize_hud_book_screen(mut screen: HudBookScreen) -> Option<HudBookScreen> {
    screen.page_count = screen.page_count.clamp(1, 100);
    screen.page = screen.page.min(screen.page_count.saturating_sub(1));
    let source_runs = screen.runs.into_iter().take(1_024).collect::<Vec<_>>();
    let fallback = source_runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    let mut budget = 4_096usize;
    screen.runs = sanitize_hud_component_runs(source_runs, &fallback, &mut budget, true);
    if screen.mode != HudBookMode::View {
        for run in &mut screen.runs {
            run.font = None;
        }
    }
    let char_count = screen
        .runs
        .iter()
        .map(|run| run.visual.text.chars().count())
        .sum();
    screen.interactions = screen
        .interactions
        .into_iter()
        .filter(|interaction| {
            interaction.start_char < interaction.end_char && interaction.end_char <= char_count
        })
        .map(|mut interaction| {
            interaction.tooltip = interaction
                .tooltip
                .take()
                .and_then(sanitize_hud_rich_tooltip);
            interaction
        })
        .collect();
    screen.cursor = screen.cursor.min(char_count);
    screen.selection = screen.selection.min(char_count);
    screen.page_indicator = sanitize_hud_text_preserving_empty(screen.page_indicator, 128);
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 128);
    screen.author = sanitize_hud_text_preserving_empty(screen.author, 128);
    screen.warning = sanitize_hud_text_preserving_empty(screen.warning, 256);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 128);
    screen.sign = sanitize_hud_text_preserving_empty(screen.sign, 128);
    screen.finalize = sanitize_hud_text_preserving_empty(screen.finalize, 128);
    screen.cancel = sanitize_hud_text_preserving_empty(screen.cancel, 128);
    Some(screen)
}

pub(super) fn hud_book_hit_target_at(
    screen: &HudBookScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudBookHitTarget> {
    let interaction = book_interaction_at(screen, fonts, surface_size, position)?;
    if !interaction.clickable {
        return None;
    }
    Some(HudBookHitTarget {
        generation: screen.generation,
        page: screen.page,
        resource_generation: screen.resource_generation,
        resources_loaded: screen.resources_loaded,
        source_run_index: interaction.source_run_index,
    })
}

fn book_interaction_at<'a>(
    screen: &'a HudBookScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<&'a HudBookInteraction> {
    if screen.mode != HudBookMode::View || !position.0.is_finite() || !position.1.is_finite() {
        return None;
    }
    let layout = hud_book_layout(surface_size);
    let x = position.0.floor() as i32 - layout.origin_x - HUD_BOOK_TEXT_X;
    let y = position.1.floor() as i32 - layout.origin_y - HUD_BOOK_TEXT_Y;
    let line_index = usize::try_from(y.checked_div(HUD_BOOK_LINE_HEIGHT)?).ok()?;
    if x < 0
        || y < 0
        || line_index >= HUD_BOOK_MAX_LINES
        || y % HUD_BOOK_LINE_HEIGHT >= HUD_BOOK_LINE_HEIGHT
    {
        return None;
    }
    let chat_interactions = book_chat_interactions(screen);
    let lines = hud_wrap_component_runs_with_interactions(
        &screen.runs,
        &chat_interactions,
        HUD_BOOK_TEXT_WIDTH,
        fonts,
    );
    let line = lines.get(line_index)?;
    let x = u32::try_from(x).ok()?;
    let source_run_index = line
        .interactions
        .iter()
        .find(|interaction| (interaction.start_x..interaction.end_x).contains(&x))
        .map(|interaction| interaction.target_index)?;
    screen
        .interactions
        .iter()
        .find(|interaction| interaction.source_run_index == source_run_index)
}

fn book_chat_interactions(screen: &HudBookScreen) -> Vec<HudChatInteraction> {
    screen
        .interactions
        .iter()
        .map(|interaction| HudChatInteraction {
            start_char: interaction.start_char,
            end_char: interaction.end_char,
            target_index: interaction.source_run_index,
            tooltip: interaction.tooltip.clone(),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_book_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    book_background: Option<&'a HudSpriteGpu>,
    page_backward: Option<&'a HudSpriteGpu>,
    page_backward_highlighted: Option<&'a HudSpriteGpu>,
    page_forward: Option<&'a HudSpriteGpu>,
    page_forward_highlighted: Option<&'a HudSpriteGpu>,
    tooltip_background: Option<&'a HudNineSliceSprite>,
    tooltip_frame: Option<&'a HudNineSliceSprite>,
    tooltip_styles: &'a HashMap<String, HudTooltipStyleSprites>,
    font_atlas: Option<&'a HudSpriteGpu>,
    fonts: &HudFontGlyphBook,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudBookScreen,
) {
    let (Some(font_atlas), Some(book_background)) = (font_atlas, book_background) else {
        return;
    };
    let glyphs = fonts.glyphs(None);
    let obfuscated_pool = fonts.obfuscated_pool(None);
    let layout = hud_book_layout(surface_size);
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        book_background,
        surface_size,
        absolute_hud_rect(
            layout.origin_x as f32,
            layout.origin_y as f32,
            HUD_BOOK_WIDTH as u32,
            HUD_BOOK_HEIGHT as u32,
        ),
        HudUvRect {
            min: [0.0, 0.0],
            max: [192.0 / 256.0, 192.0 / 256.0],
        },
        HUD_TINT_WHITE,
    );

    push_page_buttons(
        vertices,
        commands,
        surface_size,
        screen,
        layout,
        page_backward,
        page_backward_highlighted,
        page_forward,
        page_forward_highlighted,
    );
    match screen.mode {
        HudBookMode::View => push_view_page(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            fonts,
            frame_index,
            surface_size,
            screen,
            layout,
        ),
        HudBookMode::Edit => push_edit_page(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            screen,
            layout,
        ),
        HudBookMode::Sign => push_sign_page(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            screen,
            layout,
        ),
    }
    push_buttons(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen,
        layout,
    );

    let tooltip = book_interaction_at(
        screen,
        fonts,
        surface_size,
        screen
            .cursor_position
            .map_or((f64::NAN, f64::NAN), |(x, y)| (f64::from(x), f64::from(y))),
    )
    .filter(|interaction| {
        interaction.tooltip.as_ref().is_some_and(|tooltip| {
            !hud_rich_tooltip_lines(tooltip, screen.advanced_item_tooltips).is_empty()
        })
    });
    if let (Some(interaction), Some((x, y))) = (tooltip, screen.cursor_position) {
        push_hud_component_tooltip(
            vertices,
            commands,
            white_pixel,
            tooltip_background,
            tooltip_frame,
            tooltip_styles,
            font_atlas,
            fonts,
            frame_index,
            surface_size,
            interaction.tooltip.as_ref().expect("filtered tooltip"),
            screen.advanced_item_tooltips,
            x,
            y,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_view_page<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudBookScreen,
    layout: HudBookLayout,
) {
    push_page_indicator(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        fonts.glyphs(None),
        fonts.obfuscated_pool(None),
        frame_index,
        surface_size,
        screen,
        layout,
    );
    let interactions = book_chat_interactions(screen);
    let lines = hud_wrap_component_runs_with_interactions(
        &screen.runs,
        &interactions,
        HUD_BOOK_TEXT_WIDTH,
        fonts,
    );
    for (index, line) in lines.iter().take(HUD_BOOK_MAX_LINES).enumerate() {
        push_component_runs(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            fonts,
            frame_index,
            surface_size,
            &line.runs,
            (
                layout.origin_x + HUD_BOOK_TEXT_X,
                layout.origin_y
                    + HUD_BOOK_TEXT_Y
                    + i32::try_from(index).unwrap_or(i32::MAX) * HUD_BOOK_LINE_HEIGHT,
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_edit_page<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudBookScreen,
    layout: HudBookLayout,
) {
    push_page_indicator(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen,
        layout,
    );
    let text = screen
        .runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    let lines = editor_lines(&text, glyphs);
    for (index, line) in lines.iter().take(HUD_BOOK_MAX_LINES).enumerate() {
        let y = layout.origin_y
            + HUD_BOOK_TEXT_Y
            + i32::try_from(index).unwrap_or(i32::MAX) * HUD_BOOK_LINE_HEIGHT;
        push_editor_selection(
            vertices,
            commands,
            white_pixel,
            surface_size,
            &text,
            *line,
            glyphs,
            screen.cursor,
            screen.selection,
            layout.origin_x + HUD_BOOK_EDIT_TEXT_X,
            y,
        );
        let line_text = char_slice(&text, line.start, line.end);
        push_runs(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &[HudStyledTextRun::plain(line_text)],
            (layout.origin_x + HUD_BOOK_EDIT_TEXT_X, y),
        );
        if screen.cursor_visible && (line.start..=line.end).contains(&screen.cursor) {
            push_editor_cursor(
                vertices,
                commands,
                white_pixel,
                surface_size,
                &text,
                *line,
                glyphs,
                screen.cursor,
                layout.origin_x + HUD_BOOK_EDIT_TEXT_X,
                y,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_sign_page<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudBookScreen,
    layout: HudBookLayout,
) {
    for (text, y, color) in [
        (&screen.title, 34, None),
        (&screen.author, 60, Some(0x55_55_55)),
    ] {
        push_centered_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            text,
            layout.origin_x + HUD_BOOK_WIDTH / 2,
            y,
            color,
        );
    }
    let warning_runs = [HudStyledTextRun::plain(&screen.warning)];
    let warning_lines =
        hud_wrap_chat_runs_with_interactions(&warning_runs, &[], HUD_BOOK_TEXT_WIDTH, glyphs);
    for (index, line) in warning_lines.iter().enumerate() {
        push_runs(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &line.runs,
            (
                layout.origin_x + HUD_BOOK_TEXT_X,
                82 + i32::try_from(index).unwrap_or(i32::MAX) * HUD_BOOK_LINE_HEIGHT,
            ),
        );
    }
    let text = screen
        .runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    let visual_runs = screen
        .runs
        .iter()
        .map(|run| run.visual.clone())
        .collect::<Vec<_>>();
    let width = hud_logical_font_runs_width(&visual_runs, glyphs).unwrap_or_default() as i32;
    let x = layout.origin_x + HUD_BOOK_WIDTH / 2 - width / 2;
    push_editor_selection(
        vertices,
        commands,
        white_pixel,
        surface_size,
        &text,
        EditorLine {
            start: 0,
            end: text.chars().count(),
        },
        glyphs,
        screen.cursor,
        screen.selection,
        x,
        HUD_BOOK_SIGN_TITLE_Y,
    );
    push_logical_runs(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &visual_runs,
        (x, HUD_BOOK_SIGN_TITLE_Y),
    );
    if screen.cursor_visible {
        push_editor_cursor(
            vertices,
            commands,
            white_pixel,
            surface_size,
            &text,
            EditorLine {
                start: 0,
                end: text.chars().count(),
            },
            glyphs,
            screen.cursor,
            x,
            HUD_BOOK_SIGN_TITLE_Y,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_page_indicator<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudBookScreen,
    layout: HudBookLayout,
) {
    let runs = [HudStyledTextRun::plain(&screen.page_indicator)];
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or_default() as i32;
    push_runs(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &runs,
        (
            layout.origin_x + HUD_BOOK_PAGE_INDICATOR_RIGHT - width,
            layout.origin_y + HUD_BOOK_PAGE_INDICATOR_Y,
        ),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_centered_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    center_x: i32,
    y: i32,
    color: Option<u32>,
) {
    let runs = [HudStyledTextRun {
        text: text.to_string(),
        style: Default::default(),
        color,
    }];
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or_default() as i32;
    push_runs(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &runs,
        (center_x - width / 2, y),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_runs<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    origin: (i32, i32),
) {
    let geometry = hud_styled_text_pass_geometry(
        runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        (origin.0 as f32, origin.1 as f32),
        0.0,
        false,
        HUD_BOOK_TEXT_TINT,
        None,
        1.0,
    );
    push_hud_styled_text_pass(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        &geometry,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_logical_runs<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudStyledTextRun],
    origin: (i32, i32),
) {
    let geometry = hud_logical_styled_text_pass_geometry(
        runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        (origin.0 as f32, origin.1 as f32),
        0.0,
        false,
        HUD_BOOK_TEXT_TINT,
        None,
        1.0,
    );
    push_hud_styled_text_pass(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        &geometry,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_component_runs<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    fonts: &HudFontGlyphBook,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    runs: &[HudComponentTextRun],
    origin: (i32, i32),
) {
    let geometry = hud_component_text_pass_geometry(
        runs,
        fonts,
        frame_index,
        (origin.0 as f32, origin.1 as f32),
        0.0,
        false,
        HUD_BOOK_TEXT_TINT,
        None,
        1.0,
    );
    push_hud_styled_text_pass(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        &geometry,
    );
}

fn push_page_buttons<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    surface_size: PhysicalSize<u32>,
    screen: &HudBookScreen,
    layout: HudBookLayout,
    page_backward: Option<&'a HudSpriteGpu>,
    page_backward_highlighted: Option<&'a HudSpriteGpu>,
    page_forward: Option<&'a HudSpriteGpu>,
    page_forward_highlighted: Option<&'a HudSpriteGpu>,
) {
    if screen.page > 0 {
        let hovered = page_button_hovered(screen.cursor_position, layout, HUD_BOOK_PAGE_BACK_X);
        if let Some(sprite) = hovered
            .then_some(page_backward_highlighted)
            .flatten()
            .or(page_backward)
        {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                sprite,
                surface_size,
                absolute_hud_rect(
                    (layout.origin_x + HUD_BOOK_PAGE_BACK_X) as f32,
                    (layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y) as f32,
                    HUD_BOOK_PAGE_BUTTON_WIDTH as u32,
                    HUD_BOOK_PAGE_BUTTON_HEIGHT as u32,
                ),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TINT_WHITE,
            );
        }
    }
    let show_forward =
        screen.mode == HudBookMode::Edit || screen.page.saturating_add(1) < screen.page_count;
    if show_forward {
        let hovered = page_button_hovered(screen.cursor_position, layout, HUD_BOOK_PAGE_FORWARD_X);
        if let Some(sprite) = hovered
            .then_some(page_forward_highlighted)
            .flatten()
            .or(page_forward)
        {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                sprite,
                surface_size,
                absolute_hud_rect(
                    (layout.origin_x + HUD_BOOK_PAGE_FORWARD_X) as f32,
                    (layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y) as f32,
                    HUD_BOOK_PAGE_BUTTON_WIDTH as u32,
                    HUD_BOOK_PAGE_BUTTON_HEIGHT as u32,
                ),
                HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
                HUD_TINT_WHITE,
            );
        }
    }
}

fn page_button_hovered(cursor: Option<(i32, i32)>, layout: HudBookLayout, relative_x: i32) -> bool {
    cursor.is_some_and(|(x, y)| {
        (layout.origin_x + relative_x..layout.origin_x + relative_x + HUD_BOOK_PAGE_BUTTON_WIDTH)
            .contains(&x)
            && (layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y
                ..layout.origin_y + HUD_BOOK_PAGE_BUTTON_Y + HUD_BOOK_PAGE_BUTTON_HEIGHT)
                .contains(&y)
    })
}

#[allow(clippy::too_many_arguments)]
fn push_buttons<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudBookScreen,
    layout: HudBookLayout,
) {
    let cursor = screen.cursor_position;
    let y = layout.origin_y + HUD_BOOK_MENU_Y;
    let buttons = match screen.mode {
        HudBookMode::View => vec![(
            layout.origin_x + HUD_BOOK_MENU_LEFT_X,
            HUD_BOOK_MENU_FULL_WIDTH,
            screen.done.as_str(),
            true,
        )],
        HudBookMode::Edit => vec![
            (
                layout.origin_x + HUD_BOOK_MENU_LEFT_X,
                HUD_BOOK_MENU_HALF_WIDTH,
                screen.sign.as_str(),
                true,
            ),
            (
                layout.origin_x + HUD_BOOK_MENU_RIGHT_X,
                HUD_BOOK_MENU_HALF_WIDTH,
                screen.done.as_str(),
                true,
            ),
        ],
        HudBookMode::Sign => vec![
            (
                layout.origin_x + HUD_BOOK_MENU_LEFT_X,
                HUD_BOOK_MENU_HALF_WIDTH,
                screen.finalize.as_str(),
                screen.finalize_enabled,
            ),
            (
                layout.origin_x + HUD_BOOK_MENU_RIGHT_X,
                HUD_BOOK_MENU_HALF_WIDTH,
                screen.cancel.as_str(),
                true,
            ),
        ],
    };
    for (x, width, label, enabled) in buttons {
        let hovered = cursor.is_some_and(|(cursor_x, cursor_y)| {
            (x..x + width).contains(&cursor_x) && (y..y + HUD_BOOK_MENU_HEIGHT).contains(&cursor_y)
        });
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            (x, y, width, HUD_BOOK_MENU_HEIGHT),
            label,
            hovered,
            enabled,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EditorLine {
    start: usize,
    end: usize,
}

fn editor_lines(text: &str, glyphs: &HudFontGlyphMap) -> Vec<EditorLine> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return vec![EditorLine { start: 0, end: 0 }];
    }
    let mut lines = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let mut cursor = start;
        let mut width = 0u32;
        let mut last_space = None;
        let mut split = chars.len();
        while cursor < chars.len() {
            let ch = chars[cursor];
            if ch == '\n' {
                split = cursor;
                break;
            }
            if ch == ' ' {
                last_space = Some(cursor);
            }
            let advance = hud_font_glyph(ch, glyphs).styled_advance(Default::default());
            if cursor > start && width.saturating_add(advance) > HUD_BOOK_TEXT_WIDTH {
                split = last_space.filter(|space| *space >= start).unwrap_or(cursor);
                break;
            }
            width = width.saturating_add(advance);
            cursor += 1;
        }
        lines.push(EditorLine { start, end: split });
        if split == chars.len() {
            break;
        }
        start = if matches!(chars[split], '\n' | ' ') {
            split + 1
        } else {
            split
        };
    }
    if text.ends_with('\n') {
        lines.push(EditorLine {
            start: chars.len(),
            end: chars.len(),
        });
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn push_editor_selection<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    text: &str,
    line: EditorLine,
    glyphs: &HudFontGlyphMap,
    cursor: usize,
    selection: usize,
    x: i32,
    y: i32,
) {
    let selection_start = cursor.min(selection).max(line.start).min(line.end);
    let selection_end = cursor.max(selection).max(line.start).min(line.end);
    if selection_start >= selection_end {
        return;
    }
    let start_width = plain_width(char_slice(text, line.start, selection_start), glyphs);
    let selection_width = plain_width(char_slice(text, selection_start, selection_end), glyphs);
    push_hud_debug_tinted_rect(
        vertices,
        commands,
        white_pixel,
        surface_size,
        x.saturating_add(start_width as i32),
        y,
        selection_width.max(1),
        HUD_BOOK_LINE_HEIGHT as u32,
        HUD_BOOK_SELECTION_TINT,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_editor_cursor<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    text: &str,
    line: EditorLine,
    glyphs: &HudFontGlyphMap,
    cursor: usize,
    x: i32,
    y: i32,
) {
    let cursor = cursor.max(line.start).min(line.end);
    let width = plain_width(char_slice(text, line.start, cursor), glyphs);
    push_hud_debug_tinted_rect(
        vertices,
        commands,
        white_pixel,
        surface_size,
        x.saturating_add(width as i32),
        y.saturating_sub(1),
        1,
        (HUD_BOOK_LINE_HEIGHT + 1) as u32,
        HUD_BOOK_TEXT_TINT,
    );
}

fn plain_width(text: &str, glyphs: &HudFontGlyphMap) -> u32 {
    hud_logical_font_runs_width(&[HudStyledTextRun::plain(text)], glyphs).unwrap_or_default()
}

fn char_slice(text: &str, start: usize, end: usize) -> &str {
    let start = text
        .char_indices()
        .nth(start)
        .map_or(text.len(), |(index, _)| index);
    let end = text
        .char_indices()
        .nth(end)
        .map_or(text.len(), |(index, _)| index);
    &text[start..end]
}

#[cfg(test)]
mod tests;
