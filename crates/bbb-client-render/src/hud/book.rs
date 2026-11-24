use winit::dpi::{PhysicalPosition, PhysicalSize};

use super::{
    absolute_hud_rect, hud_active_area_hits, hud_coded_string_runs, hud_component_text_geometry,
    hud_component_text_hit_geometry, hud_font_runs_width, hud_logical_styled_text_geometry,
    hud_prepared_bounds_contain, hud_rich_tooltip_lines, hud_styled_text_geometry,
    hud_wrap_chat_runs_with_interactions, hud_wrap_formatted_component_runs_with_interactions,
    multiline_line_visible, multiline_selection_rect, push_hud_component_tooltip,
    push_hud_draw_with_uv_and_tint, push_hud_fill_rect, push_hud_pause_button,
    push_hud_rect_scissor, push_hud_styled_text_geometry, push_hud_text_highlight,
    sanitize_hud_component_runs, sanitize_hud_ime_preedit, sanitize_hud_rich_tooltip,
    sanitize_hud_text_preserving_empty, GuiSprites, HudActiveAreaHits, HudChatInteraction,
    HudComponentTextRun, HudDebugOptionsButtonSprites, HudDrawCommand, HudFontGlyphBook,
    HudFontGlyphMap, HudImePreedit, HudImePreeditProjection, HudObfuscatedGlyphPool, HudRect,
    HudRichTooltip, HudSpriteGpu, HudStyledTextRun, HudUvRect, HudVertex, HUD_TINT_WHITE,
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
const HUD_BOOK_EDIT_TEXT_Y: i32 = 32;
const HUD_BOOK_EDIT_VIEWPORT_Y: (f32, f32) = (28.0, 162.0);
const HUD_BOOK_EDIT_CONTINUATION_WIDTH: i32 = 118;
const HUD_BOOK_SIGN_TITLE_Y: i32 = 50;
const HUD_BOOK_TEXT_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudBookMode {
    View,
    Lectern,
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
    pub text_filtering_enabled: bool,
    pub advanced_item_tooltips: bool,
    pub mode: HudBookMode,
    pub runs: Vec<HudComponentTextRun>,
    pub editor_text: Vec<u16>,
    pub editor_display_start: usize,
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
    pub take_book: String,
    pub may_build: bool,
    pub sign: String,
    pub finalize: String,
    pub cancel: String,
    pub finalize_enabled: bool,
    pub preedit: Option<HudImePreedit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudBookInteraction {
    pub start_char: usize,
    pub end_char: usize,
    pub source_run_index: usize,
    pub clickable: bool,
    pub tooltip: Option<HudRichTooltip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudBookHitTarget {
    pub mode: HudBookMode,
    pub generation: u64,
    pub page: usize,
    pub resource_generation: u64,
    pub resources_loaded: bool,
    pub text_filtering_enabled: bool,
    pub source_run_index: usize,
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
    screen.runs.truncate(1_024);
    let had_non_empty_run = screen.runs.iter().any(|run| !run.visual.text.is_empty());
    let mut budget = 4_096usize;
    screen.runs = sanitize_hud_component_runs(screen.runs, "", &mut budget, true);
    if screen.runs.is_empty() && had_non_empty_run && budget > 0 {
        screen.runs.push(HudComponentTextRun::plain(String::new()));
    }
    if !matches!(screen.mode, HudBookMode::View | HudBookMode::Lectern) {
        for run in &mut screen.runs {
            run.font = None;
        }
    }
    let char_count = screen
        .runs
        .iter()
        .map(|run| run.visual.text.chars().count())
        .sum();
    screen.interactions.retain_mut(|interaction| {
        if interaction.start_char >= interaction.end_char || interaction.end_char > char_count {
            return false;
        }
        interaction.tooltip = interaction
            .tooltip
            .take()
            .and_then(sanitize_hud_rich_tooltip);
        true
    });
    screen.editor_text.truncate(4096);
    screen.editor_display_start = screen.editor_display_start.min(screen.editor_text.len());
    let cursor_count = if matches!(screen.mode, HudBookMode::Edit | HudBookMode::Sign) {
        screen.editor_text.len()
    } else {
        char_count
    };
    screen.cursor = screen.cursor.min(cursor_count);
    screen.selection = screen.selection.min(cursor_count);
    screen.page_indicator = sanitize_hud_text_preserving_empty(screen.page_indicator, 128);
    screen.title = sanitize_hud_text_preserving_empty(screen.title, 128);
    screen.author = sanitize_hud_text_preserving_empty(screen.author, 128);
    screen.warning = sanitize_hud_text_preserving_empty(screen.warning, 256);
    screen.done = sanitize_hud_text_preserving_empty(screen.done, 128);
    screen.take_book = sanitize_hud_text_preserving_empty(screen.take_book, 128);
    screen.sign = sanitize_hud_text_preserving_empty(screen.sign, 128);
    screen.finalize = sanitize_hud_text_preserving_empty(screen.finalize, 128);
    screen.cancel = sanitize_hud_text_preserving_empty(screen.cancel, 128);
    screen.preedit = if matches!(screen.mode, HudBookMode::Edit | HudBookMode::Sign) {
        screen.preedit.and_then(sanitize_hud_ime_preedit)
    } else {
        None
    };
    Some(screen)
}

pub(super) fn caret_origin(
    screen: &HudBookScreen,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<(i32, i32)> {
    match screen.mode {
        HudBookMode::Edit => {
            let x = i32::try_from(surface_size.width)
                .unwrap_or(i32::MAX)
                .saturating_sub(HUD_BOOK_TEXT_WIDTH as i32)
                / 2
                - 4;
            let text = &screen.editor_text;
            let lines = editor_utf16_lines(text, glyphs);
            let insert_cursor = screen.cursor < text.len();
            let Some(index) = editor_cursor_line(&lines, screen.cursor, insert_cursor) else {
                return Some((x, editor_line_y(0)));
            };
            let line = lines[index];
            let cursor_end = if insert_cursor {
                screen.cursor.clamp(line.start, line.end)
            } else {
                line.end
            };
            let cursor_x = x.saturating_add(super::hud_formatted_utf16_width(
                &text[line.start..cursor_end],
                glyphs,
            ));
            Some((cursor_x, editor_line_y(index)))
        }
        HudBookMode::Sign => {
            let layout = hud_book_layout(surface_size);
            let text = &screen.editor_text;
            let start = screen.editor_display_start.min(text.len());
            let visible_len = glyphs
                .plain_utf16_head_len(&text[start..], HUD_BOOK_TEXT_WIDTH as i32)
                .unwrap_or(0);
            let width = super::hud_formatted_utf16_width(&text[start..start + visible_len], glyphs);
            let widget_x = layout.origin_x + HUD_BOOK_TEXT_X;
            let x = widget_x + (HUD_BOOK_TEXT_WIDTH as i32 - width) / 2;
            let view = super::HudChatInputView {
                value: text.clone(),
                cursor: screen.cursor,
                selection: screen.selection,
                display_start: start,
            };
            let input = super::chat_input::edit_box_layout(
                &view,
                &[],
                glyphs,
                HUD_BOOK_TEXT_WIDTH,
                15,
                (widget_x + HUD_BOOK_TEXT_WIDTH as i32 - x) as f32,
            );
            Some((x + input.cursor_x as i32, HUD_BOOK_SIGN_TITLE_Y))
        }
        HudBookMode::View | HudBookMode::Lectern => None,
    }
}

pub(super) fn preedit_projection(
    screen: &HudBookScreen,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<HudImePreeditProjection> {
    let preedit = screen.preedit.as_ref()?;
    let origin = caret_origin(screen, surface_size, glyphs)?;
    Some(super::ime_preedit::project(
        preedit,
        origin,
        HUD_BOOK_LINE_HEIGHT + 1,
        surface_size,
        glyphs,
    ))
}

pub(super) fn ime_cursor_area(
    screen: &HudBookScreen,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    if let Some(projection) = preedit_projection(screen, surface_size, glyphs) {
        return Some((projection.ime_position, projection.ime_size));
    }
    let (x, y) = caret_origin(screen, surface_size, glyphs)?;
    Some((
        PhysicalPosition::new(x, y),
        PhysicalSize::new(1, (HUD_BOOK_LINE_HEIGHT + 1) as u32),
    ))
}

pub(super) fn hud_book_hit_target_at(
    screen: &HudBookScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudBookHitTarget> {
    // `BookViewScreen.mouseClicked` never enables insertions, so only a style
    // with a click event answers a page click.
    let (_, source_run_index) =
        book_active_area_hits(screen, fonts, surface_size, position)?.clickable?;
    Some(HudBookHitTarget {
        mode: screen.mode,
        generation: screen.generation,
        page: screen.page,
        resource_generation: screen.resource_generation,
        resources_loaded: screen.resources_loaded,
        text_filtering_enabled: screen.text_filtering_enabled,
        source_run_index,
    })
}

/// `BookViewScreen.visitText` accepts the page indicator only for drawing, then
/// up to fourteen page lines top to bottom; the click finder keeps the last
/// accepted style, so a lower line overrides an overlapping higher one.
fn book_active_area_hits(
    screen: &HudBookScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudActiveAreaHits<(usize, usize)>> {
    if !matches!(screen.mode, HudBookMode::View | HudBookMode::Lectern)
        || !position.0.is_finite()
        || !position.1.is_finite()
    {
        return None;
    }
    let click = (position.0 as i32, position.1 as i32);
    let layout = hud_book_layout(surface_size);
    let chat_interactions = book_chat_interactions(screen);
    let lines = hud_wrap_formatted_component_runs_with_interactions(
        &screen.runs,
        &chat_interactions,
        HUD_BOOK_TEXT_WIDTH,
        fonts,
    );
    let mut hits = HudActiveAreaHits::default();
    for (line_index, line) in lines.iter().take(HUD_BOOK_MAX_LINES).enumerate() {
        let origin = (
            (layout.origin_x + HUD_BOOK_TEXT_X) as f32,
            (layout.origin_y
                + HUD_BOOK_TEXT_Y
                + i32::try_from(line_index).unwrap_or(i32::MAX) * HUD_BOOK_LINE_HEIGHT)
                as f32,
        );
        let geometry = hud_component_text_hit_geometry(&line.runs, &line.targets, fonts, true);
        if !hud_prepared_bounds_contain(geometry.text_bounds, origin, click) {
            continue;
        }
        let local = (click.0 as f32 - origin.0, click.1 as f32 - origin.1);
        hud_active_area_hits(
            &geometry,
            local,
            |source_run_index| {
                screen
                    .interactions
                    .iter()
                    .find(|interaction| interaction.source_run_index == source_run_index)
                    .map(|interaction| {
                        ((line_index, source_run_index), interaction.clickable, false)
                    })
            },
            &mut hits,
        );
    }
    Some(hits)
}

fn book_interaction_at<'a>(
    screen: &'a HudBookScreen,
    fonts: &HudFontGlyphBook,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<&'a HudBookInteraction> {
    let (_, source_run_index) =
        book_active_area_hits(screen, fonts, surface_size, position)?.hovered?;
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
            clickable: interaction.clickable,
            insertion: false,
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
    tooltip_background: Option<&'a HudSpriteGpu>,
    tooltip_frame: Option<&'a HudSpriteGpu>,
    gui_sprites: &'a GuiSprites,
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
        HudBookMode::View | HudBookMode::Lectern => push_view_page(
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
        gui_sprites,
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
            gui_sprites,
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
    let lines = hud_wrap_formatted_component_runs_with_interactions(
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
    let x = i32::try_from(surface_size.width)
        .unwrap_or(i32::MAX)
        .saturating_sub(HUD_BOOK_TEXT_WIDTH as i32)
        / 2
        - 4;
    // AbstractTextAreaWidget clips text, cursor, and selection together.
    push_hud_rect_scissor(commands, surface_size, editor_scissor_rect(x));
    let text = &screen.editor_text;
    let lines = editor_utf16_lines(text, glyphs);
    let insert_cursor = screen.cursor < text.len();
    let cursor_line = screen
        .cursor_visible
        .then(|| editor_cursor_line(&lines, screen.cursor, insert_cursor))
        .flatten();
    for (index, line) in lines.iter().take(HUD_BOOK_MAX_LINES).enumerate() {
        let y = editor_line_y(index);
        if !multiline_line_visible(y as f32, HUD_BOOK_EDIT_VIEWPORT_Y) {
            continue;
        }
        let draw_insert_cursor = insert_cursor && cursor_line == Some(index);
        let before_end = if draw_insert_cursor {
            screen.cursor
        } else {
            line.end
        };
        let before = &text[line.start..before_end];
        push_resolved_runs(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &super::hud_resolve_formatted_utf16_text(before),
            (x, y),
        );
        if draw_insert_cursor {
            let cursor_x = x.saturating_add(super::hud_formatted_utf16_width(before, glyphs));
            push_resolved_runs(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &super::hud_resolve_formatted_utf16_text(&text[screen.cursor..line.end]),
                (cursor_x, y),
            );
            push_hud_fill_rect(
                vertices,
                commands,
                surface_size,
                cursor_x,
                y - 1,
                1,
                (HUD_BOOK_LINE_HEIGHT + 2) as u32,
                HUD_BOOK_TEXT_TINT,
            );
        }
    }
    if let Some(index) = cursor_line.filter(|_| !insert_cursor) {
        let line = lines[index];
        let width = super::hud_formatted_utf16_width(&text[line.start..line.end], glyphs);
        push_resolved_runs(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &[HudStyledTextRun::plain("_")],
            (x.saturating_add(width), editor_line_y(index)),
        );
    }
    // MultiLineEditBox highlights after text and cursor extraction.
    for (index, line) in lines.iter().take(HUD_BOOK_MAX_LINES).enumerate() {
        if let Some(rect) = editor_utf16_selection_rect(
            text,
            *line,
            glyphs,
            screen.cursor,
            screen.selection,
            x,
            editor_line_y(index),
        ) {
            push_hud_text_highlight(vertices, commands, surface_size, rect);
        }
    }
    commands.push(HudDrawCommand::Scissor {
        x: 0,
        y: 0,
        width: surface_size.width.max(1),
        height: surface_size.height.max(1),
    });
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
    let warning_runs = hud_coded_string_runs(&screen.warning);
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
    let text = &screen.editor_text;
    let start = screen.editor_display_start.min(text.len());
    let visible_len = glyphs
        .plain_utf16_head_len(&text[start..], HUD_BOOK_TEXT_WIDTH as i32)
        .unwrap_or(0);
    let width = super::hud_formatted_utf16_width(&text[start..start + visible_len], glyphs);
    let widget_x = layout.origin_x + HUD_BOOK_TEXT_X;
    let x = widget_x + (HUD_BOOK_TEXT_WIDTH as i32 - width) / 2;
    let view = super::HudChatInputView {
        value: text.clone(),
        cursor: screen.cursor,
        selection: screen.selection,
        display_start: start,
    };
    let input = super::chat_input::edit_box_layout(
        &view,
        &[],
        glyphs,
        HUD_BOOK_TEXT_WIDTH,
        15,
        (widget_x + HUD_BOOK_TEXT_WIDTH as i32 - x) as f32,
    );
    for (runs, offset) in [(&input.head, 0.0), (&input.tail, input.tail_x)] {
        push_logical_runs(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            runs,
            (x + offset as i32, HUD_BOOK_SIGN_TITLE_Y),
        );
    }
    if let Some((left, right)) = input.selection {
        push_hud_text_highlight(
            vertices,
            commands,
            surface_size,
            absolute_hud_rect(
                x as f32 + left,
                (HUD_BOOK_SIGN_TITLE_Y - 1) as f32,
                (right - left) as u32,
                11,
            ),
        );
    }
    if screen.cursor_visible && input.cursor_on_screen {
        let cursor_x = x + input.cursor_x as i32;
        if input.insert {
            push_hud_fill_rect(
                vertices,
                commands,
                surface_size,
                cursor_x,
                HUD_BOOK_SIGN_TITLE_Y - 1,
                1,
                11,
                HUD_BOOK_TEXT_TINT,
            );
        } else {
            push_logical_runs(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &[HudStyledTextRun::plain("_")],
                (cursor_x, HUD_BOOK_SIGN_TITLE_Y),
            );
        }
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
    let runs = hud_coded_string_runs(&screen.page_indicator);
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or_default();
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
        shadow_color: None,
        text: text.to_string(),
        style: Default::default(),
        color,
    }];
    let width = hud_font_runs_width(&runs, glyphs).unwrap_or_default();
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
    push_resolved_runs(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &resolved_book_runs(runs),
        origin,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_resolved_runs<'a>(
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
    let geometry = hud_styled_text_geometry(
        runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        (origin.0 as f32, origin.1 as f32),
        false,
        HUD_BOOK_TEXT_TINT,
        None,
        1.0,
    );
    push_hud_styled_text_geometry(
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
    let geometry = hud_logical_styled_text_geometry(
        runs,
        glyphs,
        obfuscated_pool,
        frame_index,
        (origin.0 as f32, origin.1 as f32),
        false,
        HUD_BOOK_TEXT_TINT,
        None,
        1.0,
    );
    push_hud_styled_text_geometry(
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
    let geometry = hud_component_text_geometry(
        runs,
        fonts,
        frame_index,
        (origin.0 as f32, origin.1 as f32),
        false,
        HUD_BOOK_TEXT_TINT,
        None,
        1.0,
    );
    push_hud_styled_text_geometry(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        surface_size,
        &geometry,
    );
}

#[allow(clippy::too_many_arguments)]
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
    let (show_back, show_forward) = page_button_visibility(screen);
    if show_back {
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

fn page_button_visibility(screen: &HudBookScreen) -> (bool, bool) {
    (
        screen.page > 0,
        matches!(screen.mode, HudBookMode::Edit | HudBookMode::Sign)
            || screen.page.saturating_add(1) < screen.page_count,
    )
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
    gui_sprites: &'a GuiSprites,
) {
    let cursor = screen.cursor_position;
    let y = layout.origin_y + HUD_BOOK_MENU_Y;
    let buttons = book_menu_buttons(screen, layout);
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
            &hud_coded_string_runs(label),
            hovered,
            enabled,
            HudDebugOptionsButtonSprites::widget(gui_sprites),
        );
    }
}

fn book_menu_buttons(screen: &HudBookScreen, layout: HudBookLayout) -> Vec<(i32, i32, &str, bool)> {
    match screen.mode {
        HudBookMode::View => vec![(
            layout.origin_x + HUD_BOOK_MENU_LEFT_X,
            HUD_BOOK_MENU_FULL_WIDTH,
            screen.done.as_str(),
            true,
        )],
        HudBookMode::Lectern if screen.may_build => vec![
            (
                layout.origin_x + HUD_BOOK_MENU_LEFT_X,
                HUD_BOOK_MENU_HALF_WIDTH,
                screen.done.as_str(),
                true,
            ),
            (
                layout.origin_x + HUD_BOOK_MENU_RIGHT_X,
                HUD_BOOK_MENU_HALF_WIDTH,
                screen.take_book.as_str(),
                true,
            ),
        ],
        HudBookMode::Lectern => vec![(
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
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EditorLine {
    start: usize,
    end: usize,
}

fn editor_utf16_lines(text: &[u16], glyphs: &HudFontGlyphMap) -> Vec<EditorLine> {
    super::hud_formatted_utf16_line_ranges(text, HUD_BOOK_TEXT_WIDTH, glyphs)
        .into_iter()
        .map(|range| EditorLine {
            start: range.start,
            end: range.end,
        })
        .collect()
}

#[cfg(test)]
fn editor_lines(text: &str, glyphs: &HudFontGlyphMap) -> Vec<EditorLine> {
    editor_utf16_lines(&text.encode_utf16().collect::<Vec<_>>(), glyphs)
}

fn editor_line_y(index: usize) -> i32 {
    HUD_BOOK_EDIT_TEXT_Y + index as i32 * HUD_BOOK_LINE_HEIGHT
}

fn editor_scissor_rect(text_x: i32) -> HudRect {
    absolute_hud_rect((text_x - 3) as f32, 29.0, 120, 132)
}

fn editor_cursor_line(lines: &[EditorLine], cursor: usize, insert: bool) -> Option<usize> {
    let mut visible = lines
        .iter()
        .take(HUD_BOOK_MAX_LINES)
        .enumerate()
        .filter(|(index, _)| {
            multiline_line_visible(editor_line_y(*index) as f32, HUD_BOOK_EDIT_VIEWPORT_Y)
        });
    if insert {
        visible
            .find(|(_, line)| (line.start..=line.end).contains(&cursor))
            .map(|(index, _)| index)
    } else {
        visible.next_back().map(|(index, _)| index)
    }
}

#[cfg(test)]
fn editor_selection_rect(
    text: &str,
    line: EditorLine,
    glyphs: &HudFontGlyphMap,
    cursor: usize,
    selection: usize,
    x: i32,
    y: i32,
) -> Option<HudRect> {
    editor_utf16_selection_rect(
        &text.encode_utf16().collect::<Vec<_>>(),
        line,
        glyphs,
        cursor,
        selection,
        x,
        y,
    )
}

fn editor_utf16_selection_rect(
    text: &[u16],
    line: EditorLine,
    glyphs: &HudFontGlyphMap,
    cursor: usize,
    selection: usize,
    x: i32,
    y: i32,
) -> Option<HudRect> {
    multiline_selection_rect(
        line.start..line.end,
        cursor.min(selection)..cursor.max(selection),
        (x as f32, y as f32),
        HUD_BOOK_EDIT_CONTINUATION_WIDTH,
        HUD_BOOK_EDIT_VIEWPORT_Y,
        |start, end| super::hud_formatted_utf16_width(&text[start..end], glyphs),
    )
}

fn resolved_book_runs(runs: &[HudStyledTextRun]) -> Vec<HudStyledTextRun> {
    let components = runs
        .iter()
        .cloned()
        .map(|visual| HudComponentTextRun { visual, font: None })
        .collect::<Vec<_>>();
    super::hud_resolve_formatted_component_text(&components)
        .into_iter()
        .map(|run| run.visual)
        .collect()
}

#[cfg(test)]
fn formatted_width(text: &str, glyphs: &HudFontGlyphMap) -> i32 {
    super::hud_formatted_text_width(text, glyphs)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod multiline_tests;
