//! Native Code of Conduct warning-screen values and drawing.

use winit::dpi::PhysicalSize;

use super::{
    absolute_hud_rect, hud_argb_to_tint, hud_raw_string_runs,
    push_hud_debug_options_centered_text_in_width, push_hud_draw_with_uv_and_tint,
    push_hud_inventory_scissor_reset, push_hud_rect_scissor, push_hud_screen_text_draw,
    push_hud_scrollbar, sanitize_hud_chars_with_count, sanitize_hud_text_preserving_empty,
    GuiSprites, HudDebugOptionsButtonSprites, HudDrawCommand, HudFontGlyphMap,
    HudObfuscatedGlyphPool, HudScreenTextDraw, HudSpriteGpu, HudStyledTextRun, HudTextStyle,
    HudUvRect, HudVertex, HUD_TINT_WHITE,
};

const MENU_BACKGROUND_TILE_SIZE: f32 = 32.0;
const MAX_MESSAGE_LINES: usize = 8_192;
const MAX_LINE_CHARS: usize = 65_536;
const MAX_LABEL_CHARS: usize = 256;
const MAX_COORDINATE: i32 = 1 << 20;
const MAX_DIMENSION: i32 = 1 << 16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudCodeOfConductRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudCodeOfConductScreen {
    pub text_hash: i32,
    pub title: String,
    pub message_lines: Vec<Vec<HudStyledTextRun>>,
    pub message_content_height: i32,
    pub title_rect: HudCodeOfConductRect,
    pub message_rect: HudCodeOfConductRect,
    pub message_scissor: HudCodeOfConductRect,
    pub scroll_amount: i32,
    pub max_scroll_amount: i32,
    pub scrollbar_track: Option<HudCodeOfConductRect>,
    pub scrollbar_thumb: Option<HudCodeOfConductRect>,
    pub message_focused: bool,
    pub scrollbar_hovered: bool,
    pub scrollbar_dragging: bool,
    pub checkbox_rect: HudCodeOfConductRect,
    pub checkbox_label: String,
    pub checkbox_label_lines: Vec<String>,
    pub checkbox_selected: bool,
    pub checkbox_focused: bool,
    pub checkbox_hovered: bool,
    pub acknowledge_rect: HudCodeOfConductRect,
    pub acknowledge_label: String,
    pub acknowledge_highlighted: bool,
    pub disconnect_rect: HudCodeOfConductRect,
    pub disconnect_label: String,
    pub disconnect_highlighted: bool,
}

pub(super) fn sanitize_hud_code_of_conduct_screen(
    mut screen: HudCodeOfConductScreen,
) -> Option<HudCodeOfConductScreen> {
    screen.title = sanitize_hud_text_preserving_empty(screen.title, MAX_LABEL_CHARS);
    screen.checkbox_label =
        sanitize_hud_text_preserving_empty(screen.checkbox_label, MAX_LABEL_CHARS);
    screen.acknowledge_label =
        sanitize_hud_text_preserving_empty(screen.acknowledge_label, MAX_LABEL_CHARS);
    screen.disconnect_label =
        sanitize_hud_text_preserving_empty(screen.disconnect_label, MAX_LABEL_CHARS);
    screen.message_lines.truncate(MAX_MESSAGE_LINES);
    for runs in &mut screen.message_lines {
        sanitize_message_line_in_place(runs);
    }
    if screen.message_lines.is_empty() {
        screen
            .message_lines
            .push(vec![HudStyledTextRun::plain(String::new())]);
    }
    screen.checkbox_label_lines.truncate(2);
    for line in &mut screen.checkbox_label_lines {
        *line = sanitize_hud_text_preserving_empty(std::mem::take(line), MAX_LABEL_CHARS);
    }
    if screen.checkbox_label_lines.is_empty() {
        screen.checkbox_label_lines.push(String::new());
    }
    screen.title_rect = sanitize_rect(screen.title_rect);
    screen.message_rect = sanitize_rect(screen.message_rect);
    screen.message_scissor = sanitize_rect(screen.message_scissor);
    screen.checkbox_rect = sanitize_rect(screen.checkbox_rect);
    screen.acknowledge_rect = sanitize_rect(screen.acknowledge_rect);
    screen.disconnect_rect = sanitize_rect(screen.disconnect_rect);
    screen.scrollbar_track = screen.scrollbar_track.map(sanitize_rect);
    screen.scrollbar_thumb = screen.scrollbar_thumb.map(sanitize_rect);
    screen.message_content_height = screen.message_content_height.clamp(0, MAX_DIMENSION);
    screen.max_scroll_amount = screen.max_scroll_amount.clamp(0, MAX_DIMENSION);
    screen.scroll_amount = screen.scroll_amount.clamp(0, screen.max_scroll_amount);
    Some(screen)
}

fn sanitize_message_line_in_place(runs: &mut Vec<HudStyledTextRun>) {
    let mut budget = MAX_LINE_CHARS;
    runs.retain_mut(|run| {
        if budget == 0 {
            return false;
        }
        let (text, count) =
            sanitize_hud_chars_with_count(std::mem::take(&mut run.text), budget, false);
        budget = budget.saturating_sub(count);
        if text.is_empty() {
            return false;
        }
        run.text = text;
        run.color = run.color.map(|color| color & 0xFF_FF_FF);
        true
    });
    if runs.is_empty() {
        runs.push(HudStyledTextRun::plain(String::new()));
    }
}

fn sanitize_rect(rect: HudCodeOfConductRect) -> HudCodeOfConductRect {
    HudCodeOfConductRect {
        x: rect.x.clamp(-MAX_COORDINATE, MAX_COORDINATE),
        y: rect.y.clamp(-MAX_COORDINATE, MAX_COORDINATE),
        width: rect.width.clamp(0, MAX_DIMENSION),
        height: rect.height.clamp(0, MAX_DIMENSION),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_code_of_conduct_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    menu_background: Option<&'a HudSpriteGpu>,
    text_field: Option<&'a HudSpriteGpu>,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    checkbox: Option<&'a HudSpriteGpu>,
    checkbox_highlighted: Option<&'a HudSpriteGpu>,
    checkbox_selected: Option<&'a HudSpriteGpu>,
    checkbox_selected_highlighted: Option<&'a HudSpriteGpu>,
    scrollbar_sprites: &'a GuiSprites,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudCodeOfConductScreen,
) {
    push_menu_background(
        vertices,
        commands,
        white_pixel,
        menu_background,
        surface_size,
    );
    let Some(font_atlas) = font_atlas else {
        return;
    };

    push_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &screen.title,
        screen.title_rect.x,
        screen.title_rect.y,
        true,
        HUD_TINT_WHITE,
    );

    let message_background = if screen.message_focused {
        text_field_highlighted.or(text_field)
    } else {
        text_field
    };
    if let Some(background) = message_background {
        push_sprite(
            vertices,
            commands,
            background,
            surface_size,
            screen.message_rect,
        );
    }
    if push_hud_rect_scissor(commands, surface_size, rect_to_hud(screen.message_scissor)) {
        for (index, line) in screen.message_lines.iter().enumerate() {
            let y = screen
                .message_rect
                .y
                .saturating_add(4)
                .saturating_add(i32::try_from(index).unwrap_or(i32::MAX).saturating_mul(9))
                .saturating_sub(screen.scroll_amount);
            push_runs(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                line,
                screen.message_rect.x.saturating_add(4),
                y,
                HUD_TINT_WHITE,
            );
        }
        push_hud_inventory_scissor_reset(commands, surface_size);
    }
    if let (Some(track), Some(thumb)) = (screen.scrollbar_track, screen.scrollbar_thumb) {
        push_hud_scrollbar(
            vertices,
            commands,
            white_pixel,
            scrollbar_sprites,
            surface_size,
            rect_to_hud(track),
            rect_to_hud(thumb),
        );
    }
    push_checkbox(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen,
        checkbox,
        checkbox_highlighted,
        checkbox_selected,
        checkbox_selected_highlighted,
    );
    push_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen.acknowledge_rect,
        &hud_raw_string_runs(&screen.acknowledge_label),
        screen.acknowledge_highlighted,
        button_sprites,
    );
    push_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen.disconnect_rect,
        &hud_raw_string_runs(&screen.disconnect_label),
        screen.disconnect_highlighted,
        button_sprites,
    );
}

fn push_menu_background<'a>(
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

#[allow(clippy::too_many_arguments)]
fn push_checkbox<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudCodeOfConductScreen,
    checkbox: Option<&'a HudSpriteGpu>,
    checkbox_highlighted: Option<&'a HudSpriteGpu>,
    checkbox_selected: Option<&'a HudSpriteGpu>,
    checkbox_selected_highlighted: Option<&'a HudSpriteGpu>,
) {
    let sprite = match (screen.checkbox_selected, screen.checkbox_focused) {
        (false, false) => checkbox,
        (false, true) => checkbox_highlighted.or(checkbox),
        (true, false) => checkbox_selected,
        (true, true) => checkbox_selected_highlighted.or(checkbox_selected),
    };
    if let Some(sprite) = sprite {
        push_sprite(
            vertices,
            commands,
            sprite,
            surface_size,
            HudCodeOfConductRect {
                x: screen.checkbox_rect.x,
                y: screen.checkbox_rect.y,
                width: 17,
                height: 17,
            },
        );
    }
    let lines_height = i32::try_from(screen.checkbox_label_lines.len())
        .unwrap_or(i32::MAX)
        .saturating_mul(9);
    let label_y = screen
        .checkbox_rect
        .y
        .saturating_add(17 / 2)
        .saturating_sub(lines_height / 2);
    for (index, line) in screen.checkbox_label_lines.iter().enumerate() {
        push_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            line,
            screen.checkbox_rect.x.saturating_add(21),
            label_y.saturating_add(i32::try_from(index).unwrap_or(i32::MAX) * 9),
            false,
            HUD_TINT_WHITE,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_button<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    rect: HudCodeOfConductRect,
    label: &[HudStyledTextRun],
    highlighted: bool,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
) {
    if let Some(sprite) = button_sprites.get(true, highlighted) {
        push_sprite(vertices, commands, sprite, surface_size, rect);
    }
    push_hud_debug_options_centered_text_in_width(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        label,
        rect.x,
        rect.width,
        rect.y + (rect.height - 8).max(0) / 2,
        hud_argb_to_tint(if highlighted { 0xFFFFFFA0 } else { 0xFFFFFFFF }),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    x: i32,
    y: i32,
    bold: bool,
    tint: [f32; 4],
) {
    let runs = [HudStyledTextRun {
        shadow_color: None,
        text: text.to_string(),
        style: HudTextStyle {
            bold,
            ..HudTextStyle::default()
        },
        color: None,
    }];
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
        x,
        y,
        tint,
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
    x: i32,
    y: i32,
    tint: [f32; 4],
) {
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
            origin: (x as f32, y as f32),
            scale: 1.0,
            tint,
            backdrop_opacity: 0.0,
        },
    );
}

fn push_sprite<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    sprite: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudCodeOfConductRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        sprite,
        surface_size,
        rect_to_hud(rect),
        HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
        HUD_TINT_WHITE,
    );
}

fn rect_to_hud(rect: HudCodeOfConductRect) -> super::HudRect {
    absolute_hud_rect(
        rect.x as f32,
        rect.y as f32,
        rect.width.max(0) as u32,
        rect.height.max(0) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> HudCodeOfConductScreen {
        HudCodeOfConductScreen {
            text_hash: 7,
            title: "Title".to_string(),
            message_lines: vec![vec![HudStyledTextRun::plain("line")]],
            message_content_height: 17,
            title_rect: HudCodeOfConductRect::default(),
            message_rect: HudCodeOfConductRect::default(),
            message_scissor: HudCodeOfConductRect::default(),
            scroll_amount: 0,
            max_scroll_amount: 0,
            scrollbar_track: None,
            scrollbar_thumb: None,
            message_focused: true,
            scrollbar_hovered: false,
            scrollbar_dragging: false,
            checkbox_rect: HudCodeOfConductRect::default(),
            checkbox_label: "check".to_string(),
            checkbox_label_lines: vec!["check".to_string()],
            checkbox_selected: false,
            checkbox_focused: false,
            checkbox_hovered: false,
            acknowledge_rect: HudCodeOfConductRect::default(),
            acknowledge_label: "yes".to_string(),
            acknowledge_highlighted: false,
            disconnect_rect: HudCodeOfConductRect::default(),
            disconnect_label: "no".to_string(),
            disconnect_highlighted: false,
        }
    }

    #[test]
    fn sanitization_bounds_text_geometry_and_scroll() {
        let mut screen = screen();
        screen.message_lines = vec![
            vec![HudStyledTextRun::plain("x".repeat(MAX_LINE_CHARS + 10))];
            MAX_MESSAGE_LINES + 1
        ];
        screen.message_rect = HudCodeOfConductRect {
            x: i32::MAX,
            y: i32::MIN,
            width: i32::MAX,
            height: i32::MIN,
        };
        screen.max_scroll_amount = 30;
        screen.scroll_amount = 80;

        let sanitized = sanitize_hud_code_of_conduct_screen(screen).unwrap();

        assert_eq!(sanitized.message_lines.len(), MAX_MESSAGE_LINES);
        assert_eq!(
            sanitized.message_lines[0][0].text.chars().count(),
            MAX_LINE_CHARS
        );
        assert_eq!(sanitized.message_rect.x, MAX_COORDINATE);
        assert_eq!(sanitized.message_rect.y, -MAX_COORDINATE);
        assert_eq!(sanitized.message_rect.width, MAX_DIMENSION);
        assert_eq!(sanitized.message_rect.height, 0);
        assert_eq!(sanitized.scroll_amount, 30);
    }
}
