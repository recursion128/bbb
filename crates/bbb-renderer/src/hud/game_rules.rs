use winit::dpi::PhysicalSize;

use super::{
    absolute_hud_rect, hud_argb_to_tint, push_hud_debug_options_centered_text,
    push_hud_debug_options_centered_text_in_width, push_hud_debug_options_tooltip,
    push_hud_debug_tinted_rect, push_hud_draw_with_uv_and_tint, push_hud_inventory_scissor_reset,
    push_hud_inventory_text_input_label, push_hud_pause_button, push_hud_plain_text,
    push_hud_rect_scissor, sanitize_hud_text_line, sanitize_hud_text_preserving_empty,
    HudDebugOptionsButtonSprites, HudDebugOptionsTooltip, HudDrawCommand, HudFontGlyphMap,
    HudInventoryTextInputDecoration, HudInventoryTextLabel, HudObfuscatedGlyphPool, HudRect,
    HudSpriteGpu, HudUvRect, HudVertex, HUD_TINT_WHITE,
};

pub const HUD_GAME_RULES_HEADER_HEIGHT: i32 = 36;
pub const HUD_GAME_RULES_FOOTER_HEIGHT: i32 = 33;
pub const HUD_GAME_RULES_ROW_HEIGHT: i32 = 24;
pub const HUD_GAME_RULES_LIST_WIDTH: i32 = 310;
pub const HUD_GAME_RULES_SEARCH_WIDTH: i32 = 200;
pub const HUD_GAME_RULES_SEARCH_HEIGHT: i32 = 15;
pub const HUD_GAME_RULES_VALUE_WIDTH: i32 = 44;
pub const HUD_GAME_RULES_VALUE_HEIGHT: i32 = 20;

const HUD_GAME_RULES_BUTTON_WIDTH: i32 = 150;
const HUD_GAME_RULES_BUTTON_HEIGHT: i32 = 20;
const HUD_GAME_RULES_BUTTON_SPACING: i32 = 8;
const HUD_GAME_RULES_SCROLLBAR_WIDTH: i32 = 6;
const HUD_GAME_RULES_SCROLLBAR_GAP: i32 = 4;
const HUD_GAME_RULES_MIN_SCROLLER_HEIGHT: i32 = 32;
const HUD_GAME_RULES_SELECTION_TINT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudGameRulesRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl HudGameRulesRect {
    pub fn contains(self, position: Option<(i32, i32)>) -> bool {
        let Some((x, y)) = position else {
            return false;
        };
        x >= self.x
            && x < self.x.saturating_add(self.width)
            && y >= self.y
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudGameRulesLayout {
    pub search: HudGameRulesRect,
    pub list: HudGameRulesRect,
    pub scrollbar: HudGameRulesRect,
    pub done: HudGameRulesRect,
    pub cancel: HudGameRulesRect,
}

pub fn hud_game_rules_layout(surface_size: PhysicalSize<u32>) -> HudGameRulesLayout {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let list_height = height
        .saturating_sub(HUD_GAME_RULES_HEADER_HEIGHT + HUD_GAME_RULES_FOOTER_HEIGHT)
        .max(0);
    let list_x = width / 2 - HUD_GAME_RULES_LIST_WIDTH / 2;
    let button_group_width = HUD_GAME_RULES_BUTTON_WIDTH * 2 + HUD_GAME_RULES_BUTTON_SPACING;
    let button_x = width / 2 - button_group_width / 2;
    let button_y = height
        .saturating_sub(HUD_GAME_RULES_FOOTER_HEIGHT)
        .saturating_add(6);
    HudGameRulesLayout {
        search: HudGameRulesRect {
            x: width / 2 - HUD_GAME_RULES_SEARCH_WIDTH / 2,
            y: HUD_GAME_RULES_HEADER_HEIGHT - HUD_GAME_RULES_SEARCH_HEIGHT - 4,
            width: HUD_GAME_RULES_SEARCH_WIDTH,
            height: HUD_GAME_RULES_SEARCH_HEIGHT,
        },
        list: HudGameRulesRect {
            x: list_x,
            y: HUD_GAME_RULES_HEADER_HEIGHT,
            width: HUD_GAME_RULES_LIST_WIDTH,
            height: list_height,
        },
        scrollbar: HudGameRulesRect {
            x: list_x + HUD_GAME_RULES_LIST_WIDTH + HUD_GAME_RULES_SCROLLBAR_GAP,
            y: HUD_GAME_RULES_HEADER_HEIGHT,
            width: HUD_GAME_RULES_SCROLLBAR_WIDTH,
            height: list_height,
        },
        done: HudGameRulesRect {
            x: button_x,
            y: button_y,
            width: HUD_GAME_RULES_BUTTON_WIDTH,
            height: HUD_GAME_RULES_BUTTON_HEIGHT,
        },
        cancel: HudGameRulesRect {
            x: button_x + HUD_GAME_RULES_BUTTON_WIDTH + HUD_GAME_RULES_BUTTON_SPACING,
            y: button_y,
            width: HUD_GAME_RULES_BUTTON_WIDTH,
            height: HUD_GAME_RULES_BUTTON_HEIGHT,
        },
    }
}

pub fn hud_game_rules_confirmation_buttons(
    surface_size: PhysicalSize<u32>,
) -> (HudGameRulesRect, HudGameRulesRect) {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let group_width = HUD_GAME_RULES_BUTTON_WIDTH * 2 + HUD_GAME_RULES_BUTTON_SPACING;
    let start_x = width / 2 - group_width / 2;
    (
        HudGameRulesRect {
            x: start_x,
            y: height / 2 + 4,
            width: HUD_GAME_RULES_BUTTON_WIDTH,
            height: HUD_GAME_RULES_BUTTON_HEIGHT,
        },
        HudGameRulesRect {
            x: start_x + HUD_GAME_RULES_BUTTON_WIDTH + HUD_GAME_RULES_BUTTON_SPACING,
            y: height / 2 + 4,
            width: HUD_GAME_RULES_BUTTON_WIDTH,
            height: HUD_GAME_RULES_BUTTON_HEIGHT,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HudGameRulesRow {
    Category {
        category: String,
        label: String,
    },
    Boolean {
        rule: String,
        label: String,
        value: bool,
        value_label: String,
        hovered: bool,
        focused: bool,
    },
    Integer {
        rule: String,
        label: String,
        input: String,
        valid: bool,
        cursor: usize,
        selection: usize,
        hovered: bool,
        focused: bool,
    },
}

impl HudGameRulesRow {
    pub fn rule(&self) -> Option<&str> {
        match self {
            Self::Category { .. } => None,
            Self::Boolean { rule, .. } | Self::Integer { rule, .. } => Some(rule),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudGameRulesDiscardConfirmation {
    pub title: String,
    pub message: String,
    pub discard: String,
    pub keep_editing: String,
    pub discard_hovered: bool,
    pub keep_editing_hovered: bool,
    pub discard_focused: bool,
    pub keep_editing_focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudGameRulesScreen {
    pub title: String,
    pub search_hint: String,
    pub search_text: String,
    pub search_cursor: usize,
    pub search_selection: usize,
    pub search_focused: bool,
    pub loading_text: String,
    pub is_loading: bool,
    pub rows: Vec<HudGameRulesRow>,
    pub scroll_amount: i32,
    pub max_scroll_amount: i32,
    pub scroller_height: i32,
    pub done: String,
    pub cancel: String,
    pub done_enabled: bool,
    pub done_hovered: bool,
    pub cancel_hovered: bool,
    pub done_focused: bool,
    pub cancel_focused: bool,
    pub tooltip: Option<HudDebugOptionsTooltip>,
    pub discard_confirmation: Option<HudGameRulesDiscardConfirmation>,
}

pub(super) fn sanitize_hud_game_rules_screen(
    screen: HudGameRulesScreen,
) -> Option<HudGameRulesScreen> {
    let title = sanitize_hud_text_line(screen.title)?;
    let search_hint = sanitize_hud_text_line(screen.search_hint)?;
    let search_text = sanitize_hud_text_preserving_empty(screen.search_text, 256);
    let loading_text = sanitize_hud_text_line(screen.loading_text)?;
    let done = sanitize_hud_text_line(screen.done)?;
    let cancel = sanitize_hud_text_line(screen.cancel)?;
    let rows = screen
        .rows
        .into_iter()
        .filter_map(sanitize_hud_game_rules_row)
        .take(128)
        .collect();
    let max_scroll_amount = screen.max_scroll_amount.clamp(0, 1_000_000);
    let scroll_amount = screen.scroll_amount.clamp(0, max_scroll_amount);
    let scroller_height = if max_scroll_amount > 0 {
        screen.scroller_height.clamp(1, 4096)
    } else {
        0
    };
    Some(HudGameRulesScreen {
        title,
        search_hint,
        search_text,
        search_cursor: screen.search_cursor.min(256),
        search_selection: screen.search_selection.min(256),
        search_focused: screen.search_focused,
        loading_text,
        is_loading: screen.is_loading,
        rows,
        scroll_amount,
        max_scroll_amount,
        scroller_height,
        done,
        cancel,
        done_enabled: screen.done_enabled,
        done_hovered: screen.done_hovered,
        cancel_hovered: screen.cancel_hovered,
        done_focused: screen.done_focused,
        cancel_focused: screen.cancel_focused,
        tooltip: screen
            .tooltip
            .and_then(super::sanitize_hud_debug_options_tooltip),
        discard_confirmation: screen
            .discard_confirmation
            .and_then(sanitize_hud_game_rules_discard_confirmation),
    })
}

fn sanitize_hud_game_rules_row(row: HudGameRulesRow) -> Option<HudGameRulesRow> {
    Some(match row {
        HudGameRulesRow::Category { category, label } => HudGameRulesRow::Category {
            category: sanitize_hud_text_line(category)?,
            label: sanitize_hud_text_line(label)?,
        },
        HudGameRulesRow::Boolean {
            rule,
            label,
            value,
            value_label,
            hovered,
            focused,
        } => HudGameRulesRow::Boolean {
            rule: sanitize_hud_text_line(rule)?,
            label: sanitize_hud_text_line(label)?,
            value,
            value_label: sanitize_hud_text_line(value_label)?,
            hovered,
            focused,
        },
        HudGameRulesRow::Integer {
            rule,
            label,
            input,
            valid,
            cursor,
            selection,
            hovered,
            focused,
        } => {
            let input = sanitize_hud_text_preserving_empty(input, 64);
            let length = input.chars().count();
            HudGameRulesRow::Integer {
                rule: sanitize_hud_text_line(rule)?,
                label: sanitize_hud_text_line(label)?,
                input,
                valid,
                cursor: cursor.min(length),
                selection: selection.min(length),
                hovered,
                focused,
            }
        }
    })
}

fn sanitize_hud_game_rules_discard_confirmation(
    confirmation: HudGameRulesDiscardConfirmation,
) -> Option<HudGameRulesDiscardConfirmation> {
    Some(HudGameRulesDiscardConfirmation {
        title: sanitize_hud_text_line(confirmation.title)?,
        message: sanitize_hud_text_line(confirmation.message)?,
        discard: sanitize_hud_text_line(confirmation.discard)?,
        keep_editing: sanitize_hud_text_line(confirmation.keep_editing)?,
        discard_hovered: confirmation.discard_hovered,
        keep_editing_hovered: confirmation.keep_editing_hovered,
        discard_focused: confirmation.discard_focused,
        keep_editing_focused: confirmation.keep_editing_focused,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_game_rules_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    tooltip_background: Option<&'a super::HudNineSliceSprite>,
    tooltip_frame: Option<&'a super::HudNineSliceSprite>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudGameRulesScreen,
) {
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = hud_game_rules_layout(surface_size);
    push_hud_debug_tinted_rect(
        vertices,
        commands,
        white_pixel,
        surface_size,
        0,
        0,
        surface_size.width,
        surface_size.height,
        [0.0, 0.0, 0.0, 0.45],
    );
    push_hud_debug_options_centered_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &screen.title,
        5,
        HUD_TINT_WHITE,
    );
    push_text_field(
        vertices,
        commands,
        white_pixel,
        text_field_highlighted,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        layout.search,
        if screen.search_text.is_empty() && !screen.search_focused {
            &screen.search_hint
        } else {
            &screen.search_text
        },
        screen.search_cursor,
        screen.search_selection,
        screen.search_focused,
        true,
    );

    if screen.is_loading {
        push_hud_debug_options_centered_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            &screen.loading_text,
            i32::try_from(surface_size.height).unwrap_or(i32::MAX) / 2 - 4,
            HUD_TINT_WHITE,
        );
    } else {
        push_rows(
            vertices,
            commands,
            white_pixel,
            text_field_highlighted,
            button_sprites,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            screen,
            layout,
        );
        push_scrollbar(
            vertices,
            commands,
            white_pixel,
            surface_size,
            screen,
            layout,
        );
    }

    for (rect, label, hovered, enabled) in [
        (
            layout.done,
            screen.done.as_str(),
            screen.done_hovered || screen.done_focused,
            screen.done_enabled,
        ),
        (
            layout.cancel,
            screen.cancel.as_str(),
            screen.cancel_hovered || screen.cancel_focused,
            true,
        ),
    ] {
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            (rect.x, rect.y, rect.width, rect.height),
            label,
            hovered,
            enabled,
        );
    }
    push_hud_debug_options_tooltip(
        vertices,
        commands,
        white_pixel,
        tooltip_background,
        tooltip_frame,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen.tooltip.as_ref(),
    );
    if let Some(confirmation) = &screen.discard_confirmation {
        push_discard_confirmation(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            confirmation,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_rows<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudGameRulesScreen,
    layout: HudGameRulesLayout,
) {
    if !push_hud_rect_scissor(
        commands,
        surface_size,
        HudRect {
            x: layout.list.x as f32,
            y: layout.list.y as f32,
            width: layout.list.width.max(0) as u32,
            height: layout.list.height.max(0) as u32,
        },
    ) {
        return;
    }
    for (index, row) in screen.rows.iter().enumerate() {
        let y = layout.list.y
            + i32::try_from(index).unwrap_or(i32::MAX) * HUD_GAME_RULES_ROW_HEIGHT
            - screen.scroll_amount;
        if y.saturating_add(HUD_GAME_RULES_ROW_HEIGHT) <= layout.list.y
            || y >= layout.list.y.saturating_add(layout.list.height)
        {
            continue;
        }
        match row {
            HudGameRulesRow::Category { label, .. } => {
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
                    layout.list.x,
                    layout.list.width,
                    y + 8,
                    hud_argb_to_tint(0xFFA0A0A0),
                );
            }
            HudGameRulesRow::Boolean {
                label,
                value_label,
                hovered,
                focused,
                ..
            } => {
                push_hud_plain_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    label,
                    ((layout.list.x + 5) as f32, (y + 8) as f32),
                    HUD_TINT_WHITE,
                    1.0,
                    true,
                );
                let value_x = layout.list.x + layout.list.width - HUD_GAME_RULES_VALUE_WIDTH - 5;
                super::push_hud_debug_options_button(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    value_x,
                    y + 2,
                    HUD_GAME_RULES_VALUE_WIDTH,
                    HUD_GAME_RULES_VALUE_HEIGHT,
                    value_label,
                    true,
                    *hovered || *focused,
                    HUD_TINT_WHITE,
                    button_sprites,
                );
            }
            HudGameRulesRow::Integer {
                label,
                input,
                valid,
                cursor,
                selection,
                hovered,
                focused,
                ..
            } => {
                push_hud_plain_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    label,
                    ((layout.list.x + 5) as f32, (y + 8) as f32),
                    HUD_TINT_WHITE,
                    1.0,
                    true,
                );
                push_text_field(
                    vertices,
                    commands,
                    white_pixel,
                    text_field_highlighted,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    HudGameRulesRect {
                        x: layout.list.x + layout.list.width - HUD_GAME_RULES_VALUE_WIDTH - 5,
                        y: y + 2,
                        width: HUD_GAME_RULES_VALUE_WIDTH,
                        height: HUD_GAME_RULES_VALUE_HEIGHT,
                    },
                    input,
                    *cursor,
                    *selection,
                    *focused,
                    *valid,
                );
                if *hovered && !*focused {
                    push_hud_debug_tinted_rect(
                        vertices,
                        commands,
                        white_pixel,
                        surface_size,
                        layout.list.x + layout.list.width - HUD_GAME_RULES_VALUE_WIDTH - 5,
                        y + 2,
                        HUD_GAME_RULES_VALUE_WIDTH as u32,
                        HUD_GAME_RULES_VALUE_HEIGHT as u32,
                        [1.0, 1.0, 1.0, 0.08],
                    );
                }
            }
        }
    }
    push_hud_inventory_scissor_reset(commands, surface_size);
}

fn push_scrollbar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    screen: &HudGameRulesScreen,
    layout: HudGameRulesLayout,
) {
    if screen.max_scroll_amount <= 0 || layout.scrollbar.height <= 0 {
        return;
    }
    push_hud_debug_tinted_rect(
        vertices,
        commands,
        white_pixel,
        surface_size,
        layout.scrollbar.x,
        layout.scrollbar.y,
        layout.scrollbar.width.max(0) as u32,
        layout.scrollbar.height.max(0) as u32,
        [0.0, 0.0, 0.0, 0.45],
    );
    let scroller_height = screen
        .scroller_height
        .clamp(
            HUD_GAME_RULES_MIN_SCROLLER_HEIGHT.min(layout.scrollbar.height),
            layout.scrollbar.height,
        )
        .max(1);
    let travel = (layout.scrollbar.height - scroller_height).max(0);
    let y = layout.scrollbar.y
        + screen
            .scroll_amount
            .saturating_mul(travel)
            .checked_div(screen.max_scroll_amount.max(1))
            .unwrap_or_default();
    push_hud_debug_tinted_rect(
        vertices,
        commands,
        white_pixel,
        surface_size,
        layout.scrollbar.x,
        y,
        layout.scrollbar.width.max(0) as u32,
        scroller_height as u32,
        [0.55, 0.55, 0.55, 0.92],
    );
}

#[allow(clippy::too_many_arguments)]
fn push_text_field<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    rect: HudGameRulesRect,
    text: &str,
    cursor: usize,
    selection: usize,
    focused: bool,
    valid: bool,
) {
    if let Some(text_field_highlighted) = text_field_highlighted {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            text_field_highlighted,
            surface_size,
            absolute_hud_rect(
                rect.x as f32,
                rect.y as f32,
                rect.width.max(0) as u32,
                rect.height.max(0) as u32,
            ),
            HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            if valid {
                HUD_TINT_WHITE
            } else {
                hud_argb_to_tint(0xFFFF5555)
            },
        );
    } else {
        push_hud_debug_tinted_rect(
            vertices,
            commands,
            white_pixel,
            surface_size,
            rect.x,
            rect.y,
            rect.width.max(0) as u32,
            rect.height.max(0) as u32,
            if valid {
                [0.95, 0.95, 0.95, 1.0]
            } else {
                [1.0, 0.25, 0.25, 1.0]
            },
        );
        push_hud_debug_tinted_rect(
            vertices,
            commands,
            white_pixel,
            surface_size,
            rect.x + 1,
            rect.y + 1,
            rect.width.saturating_sub(2).max(0) as u32,
            rect.height.saturating_sub(2).max(0) as u32,
            [0.0, 0.0, 0.0, 0.75],
        );
    }
    let input = HudInventoryTextInputDecoration {
        cursor,
        selection,
        scroll_to: if cursor == selection {
            cursor
        } else {
            selection
        },
        max_length: 256,
        cursor_visible: focused,
        cursor_tint: if valid {
            HUD_TINT_WHITE
        } else {
            hud_argb_to_tint(0xFFFF5555)
        },
        selection_tint: HUD_GAME_RULES_SELECTION_TINT,
    };
    let label = HudInventoryTextLabel {
        x: 0,
        y: 0,
        width: rect.width.saturating_sub(8).max(0) as u32,
        text: text.to_string(),
        tint: if valid {
            HUD_TINT_WHITE
        } else {
            hud_argb_to_tint(0xFFFF5555)
        },
        background: None,
        input: Some(input),
        scrolling: None,
        shadow: true,
        runs: Vec::new(),
    };
    push_hud_inventory_text_input_label(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &label,
        input,
        ((rect.x + 4) as f32, (rect.y + (rect.height - 8) / 2) as f32),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_discard_confirmation<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    confirmation: &HudGameRulesDiscardConfirmation,
) {
    push_hud_debug_tinted_rect(
        vertices,
        commands,
        white_pixel,
        surface_size,
        0,
        0,
        surface_size.width,
        surface_size.height,
        [0.0, 0.0, 0.0, 0.72],
    );
    let center_y = i32::try_from(surface_size.height).unwrap_or(i32::MAX) / 2;
    push_hud_debug_options_centered_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &confirmation.title,
        center_y - 42,
        HUD_TINT_WHITE,
    );
    push_hud_debug_options_centered_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &confirmation.message,
        center_y - 20,
        HUD_TINT_WHITE,
    );
    let group_width = HUD_GAME_RULES_BUTTON_WIDTH * 2 + HUD_GAME_RULES_BUTTON_SPACING;
    let start_x = i32::try_from(surface_size.width).unwrap_or(i32::MAX) / 2 - group_width / 2;
    for (index, (label, hovered, focused)) in [
        (
            confirmation.discard.as_str(),
            confirmation.discard_hovered,
            confirmation.discard_focused,
        ),
        (
            confirmation.keep_editing.as_str(),
            confirmation.keep_editing_hovered,
            confirmation.keep_editing_focused,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        push_hud_pause_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            (
                start_x
                    + i32::try_from(index).unwrap_or(i32::MAX)
                        * (HUD_GAME_RULES_BUTTON_WIDTH + HUD_GAME_RULES_BUTTON_SPACING),
                center_y + 4,
                HUD_GAME_RULES_BUTTON_WIDTH,
                HUD_GAME_RULES_BUTTON_HEIGHT,
            ),
            label,
            hovered || focused,
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_preserves_vanilla_header_footer_and_widget_sizes() {
        let layout = hud_game_rules_layout(PhysicalSize::new(854, 480));

        assert_eq!(layout.search.width, 200);
        assert_eq!(layout.search.height, 15);
        assert_eq!(layout.list.y, 36);
        assert_eq!(layout.list.height, 411);
        assert_eq!(layout.done.width, 150);
        assert_eq!(layout.done.height, 20);
        assert_eq!(layout.done.y, 453);
        assert_eq!(layout.cancel.x - layout.done.x, 158);
    }

    #[test]
    fn sanitizer_clamps_scroll_and_rejects_empty_required_labels() {
        let screen = HudGameRulesScreen {
            title: "Game Rules".to_string(),
            search_hint: "Search".to_string(),
            search_text: "\nkeep".to_string(),
            search_cursor: 99,
            search_selection: 99,
            search_focused: true,
            loading_text: "Downloading".to_string(),
            is_loading: false,
            rows: vec![HudGameRulesRow::Integer {
                rule: "minecraft:random_tick_speed".to_string(),
                label: "Random Tick Speed".to_string(),
                input: "3".to_string(),
                valid: true,
                cursor: 4,
                selection: 4,
                hovered: false,
                focused: true,
            }],
            scroll_amount: 90,
            max_scroll_amount: 12,
            scroller_height: 18,
            done: "Done".to_string(),
            cancel: "Cancel".to_string(),
            done_enabled: true,
            done_hovered: false,
            cancel_hovered: false,
            done_focused: false,
            cancel_focused: false,
            tooltip: None,
            discard_confirmation: None,
        };

        let sanitized = sanitize_hud_game_rules_screen(screen).unwrap();
        assert_eq!(sanitized.search_text, "keep");
        assert_eq!(sanitized.scroll_amount, 12);
        let HudGameRulesRow::Integer {
            cursor, selection, ..
        } = &sanitized.rows[0]
        else {
            panic!("integer row");
        };
        assert_eq!((*cursor, *selection), (1, 1));

        let mut invalid = sanitized;
        invalid.title.clear();
        assert!(sanitize_hud_game_rules_screen(invalid).is_none());
    }
}
