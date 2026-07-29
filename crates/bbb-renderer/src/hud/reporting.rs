//! Player-reporting screen values, shared layout, drawing, and hit testing.

use winit::dpi::PhysicalSize;

use crate::Renderer;

use super::{
    absolute_hud_rect, hud_argb_to_tint, hud_wrap_plain_text, push_hud_debug_options_button,
    push_hud_debug_options_centered_text, push_hud_debug_options_text,
    push_hud_draw_with_uv_and_tint, push_hud_rect_scissor, HudDebugOptionsButtonSprites,
    HudDrawCommand, HudFontGlyphMap, HudObfuscatedGlyphPool, HudSpriteGpu, HudUvRect, HudVertex,
    HUD_TINT_WHITE,
};

const PANEL_WIDTH: i32 = 320;
const PANEL_MARGIN: i32 = 12;
const TITLE_Y: i32 = 12;
const CONTENT_TOP: i32 = 36;
const CONTENT_BOTTOM_MARGIN: i32 = 42;
const CONTROL_GAP: i32 = 8;
const BUTTON_HEIGHT: i32 = 20;
const TEXT_INPUT_HEIGHT: i32 = 72;
const ROW_HEIGHT: i32 = 24;
const MAX_ROWS: usize = 1_024;
const MAX_TEXT_CHARS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPlayerReportingScreenKind {
    TypeChooser,
    ChatEditor,
    SkinEditor,
    UsernameEditor,
    ReasonSelection,
    ChatSelection,
    DraftChoice,
    DiscardWarning,
    Sending,
    Success,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPlayerReportingFocus {
    ReportType(u8),
    SelectMessages,
    SelectReason,
    Comments,
    Attestation,
    Reason(u8),
    Message(u64),
    Back,
    Send,
    Done,
    Cancel,
    ReadInfo,
    DraftEdit,
    DraftDiscard,
    WarningReturn,
    WarningSaveDraft,
    WarningDiscard,
    Retry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HudPlayerReportingControlKind {
    Button,
    Checkbox {
        selected: bool,
    },
    TextInput {
        value: String,
        placeholder: String,
        cursor: usize,
        selection: usize,
        cursor_visible: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPlayerReportingControlPlacement {
    Body,
    Footer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudPlayerReportingControl {
    pub focus: HudPlayerReportingFocus,
    pub label: String,
    pub enabled: bool,
    pub placement: HudPlayerReportingControlPlacement,
    pub kind: HudPlayerReportingControlKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudPlayerReportingRow {
    pub focus: HudPlayerReportingFocus,
    pub title: String,
    pub detail: Option<String>,
    pub selected: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudPlayerReportingScreen {
    pub generation: u64,
    pub kind: HudPlayerReportingScreenKind,
    pub title: String,
    pub message: Option<String>,
    pub status: Option<String>,
    pub rows: Vec<HudPlayerReportingRow>,
    pub controls: Vec<HudPlayerReportingControl>,
    pub scroll_amount: i32,
    pub focus: HudPlayerReportingFocus,
    pub hovered: Option<HudPlayerReportingFocus>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudPlayerReportingRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl HudPlayerReportingRect {
    fn contains(self, position: (f64, f64)) -> bool {
        position.0 >= f64::from(self.x)
            && position.0 < f64::from(self.x.saturating_add(self.width))
            && position.1 >= f64::from(self.y)
            && position.1 < f64::from(self.y.saturating_add(self.height))
    }

    fn intersects(self, other: Self) -> bool {
        self.x < other.x.saturating_add(other.width)
            && self.x.saturating_add(self.width) > other.x
            && self.y < other.y.saturating_add(other.height)
            && self.y.saturating_add(self.height) > other.y
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudPlayerReportingLayout {
    pub panel: HudPlayerReportingRect,
    pub list: HudPlayerReportingRect,
    pub rows: Vec<(HudPlayerReportingFocus, HudPlayerReportingRect)>,
    pub controls: Vec<(HudPlayerReportingFocus, HudPlayerReportingRect)>,
    pub max_scroll_amount: i32,
}

pub fn hud_player_reporting_layout(
    screen: &HudPlayerReportingScreen,
    surface_size: PhysicalSize<u32>,
) -> HudPlayerReportingLayout {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let panel_width = PANEL_WIDTH
        .min(width.saturating_sub(PANEL_MARGIN * 2))
        .max(120);
    let panel = HudPlayerReportingRect {
        x: width / 2 - panel_width / 2,
        y: PANEL_MARGIN,
        width: panel_width,
        height: height.saturating_sub(PANEL_MARGIN * 2).max(80),
    };
    let footer_controls = screen
        .controls
        .iter()
        .filter(|control| control.placement == HudPlayerReportingControlPlacement::Footer)
        .count();
    let list_y = if !screen.rows.is_empty() && screen.message.is_some() {
        CONTENT_TOP + 22
    } else {
        CONTENT_TOP
    };
    let list = HudPlayerReportingRect {
        x: panel.x,
        y: list_y,
        width: panel.width,
        height: height.saturating_sub(list_y + CONTENT_BOTTOM_MARGIN).max(0),
    };
    let content_height = i32::try_from(screen.rows.len())
        .unwrap_or(i32::MAX / ROW_HEIGHT)
        .saturating_mul(ROW_HEIGHT);
    let max_scroll_amount = content_height.saturating_sub(list.height).max(0);
    let scroll_amount = screen.scroll_amount.clamp(0, max_scroll_amount);
    let rows = screen
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            (
                row.focus,
                HudPlayerReportingRect {
                    x: list.x,
                    y: list
                        .y
                        .saturating_add(
                            i32::try_from(index)
                                .unwrap_or(i32::MAX / ROW_HEIGHT)
                                .saturating_mul(ROW_HEIGHT),
                        )
                        .saturating_sub(scroll_amount),
                    width: list.width,
                    height: ROW_HEIGHT,
                },
            )
        })
        .collect();

    let body_controls = screen
        .controls
        .iter()
        .filter(|control| control.placement == HudPlayerReportingControlPlacement::Body)
        .collect::<Vec<_>>();
    let total_body_height = body_controls
        .iter()
        .map(|control| control_height(control))
        .sum::<i32>()
        .saturating_add(
            i32::try_from(body_controls.len().saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(CONTROL_GAP),
        );
    let mut body_y = if screen.rows.is_empty() {
        (height - total_body_height) / 2
    } else {
        list_y
    };
    let mut controls = Vec::with_capacity(screen.controls.len());
    for control in &body_controls {
        let control_height = control_height(control);
        controls.push((
            control.focus,
            HudPlayerReportingRect {
                x: panel.x + 20,
                y: body_y,
                width: panel.width - 40,
                height: control_height,
            },
        ));
        body_y = body_y.saturating_add(control_height + CONTROL_GAP);
    }
    if footer_controls > 0 {
        let total_gap = i32::try_from(footer_controls.saturating_sub(1))
            .unwrap_or(i32::MAX)
            .saturating_mul(CONTROL_GAP);
        let width_each = (panel.width - total_gap)
            .checked_div(i32::try_from(footer_controls).unwrap_or(1).max(1))
            .unwrap_or(panel.width);
        let mut footer_x = panel.x;
        for control in screen
            .controls
            .iter()
            .filter(|control| control.placement == HudPlayerReportingControlPlacement::Footer)
        {
            controls.push((
                control.focus,
                HudPlayerReportingRect {
                    x: footer_x,
                    y: height.saturating_sub(32),
                    width: width_each,
                    height: BUTTON_HEIGHT,
                },
            ));
            footer_x = footer_x.saturating_add(width_each + CONTROL_GAP);
        }
    }
    HudPlayerReportingLayout {
        panel,
        list,
        rows,
        controls,
        max_scroll_amount,
    }
}

fn control_height(control: &HudPlayerReportingControl) -> i32 {
    match control.kind {
        HudPlayerReportingControlKind::TextInput { .. } => TEXT_INPUT_HEIGHT,
        HudPlayerReportingControlKind::Button | HudPlayerReportingControlKind::Checkbox { .. } => {
            BUTTON_HEIGHT
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudPlayerReportingHitTarget {
    pub generation: u64,
    pub focus: HudPlayerReportingFocus,
}

pub(super) fn sanitize_hud_player_reporting_screen(
    mut screen: HudPlayerReportingScreen,
) -> HudPlayerReportingScreen {
    sanitize(&mut screen.title);
    if let Some(message) = &mut screen.message {
        sanitize(message);
    }
    if let Some(status) = &mut screen.status {
        sanitize(status);
    }
    screen.rows.truncate(MAX_ROWS);
    for row in &mut screen.rows {
        sanitize(&mut row.title);
        if let Some(detail) = &mut row.detail {
            sanitize(detail);
        }
    }
    for control in &mut screen.controls {
        sanitize(&mut control.label);
        if let HudPlayerReportingControlKind::TextInput {
            value,
            placeholder,
            cursor,
            selection,
            ..
        } = &mut control.kind
        {
            sanitize(value);
            sanitize(placeholder);
            let len = value.chars().count();
            *cursor = (*cursor).min(len);
            *selection = (*selection).min(len);
        }
    }
    screen.scroll_amount = screen.scroll_amount.max(0);
    screen
}

fn sanitize(value: &mut String) {
    if value.chars().count() > MAX_TEXT_CHARS {
        *value = value.chars().take(MAX_TEXT_CHARS).collect();
    }
}

pub(super) fn hud_player_reporting_hit_target_at(
    screen: &HudPlayerReportingScreen,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudPlayerReportingHitTarget> {
    let layout = hud_player_reporting_layout(screen, surface_size);
    let focus = layout
        .controls
        .iter()
        .find(|(focus, rect)| {
            rect.contains(position)
                && screen
                    .controls
                    .iter()
                    .find(|control| control.focus == *focus)
                    .is_some_and(|control| control.enabled)
        })
        .map(|(focus, _)| *focus)
        .or_else(|| {
            layout
                .rows
                .iter()
                .find(|(focus, rect)| {
                    rect.intersects(layout.list)
                        && layout.list.contains(position)
                        && rect.contains(position)
                        && screen
                            .rows
                            .iter()
                            .find(|row| row.focus == *focus)
                            .is_some_and(|row| row.enabled)
                })
                .map(|(focus, _)| *focus)
        })?;
    Some(HudPlayerReportingHitTarget {
        generation: screen.generation,
        focus,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_player_reporting_screen<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    background: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudPlayerReportingScreen,
) {
    let background = background.unwrap_or(white_pixel);
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
            max: [1.0, 1.0],
        },
        if std::ptr::eq(background, white_pixel) {
            [0.08, 0.08, 0.08, 0.96]
        } else {
            HUD_TINT_WHITE
        },
    );
    let Some(font_atlas) = font_atlas else {
        return;
    };
    let layout = hud_player_reporting_layout(screen, surface_size);
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
        TITLE_Y,
        HUD_TINT_WHITE,
    );
    if let Some(message) = &screen.message {
        for (line, text) in hud_wrap_plain_text(message, layout.panel.width.max(1) as u32, glyphs)
            .into_iter()
            .take(3)
            .enumerate()
        {
            push_hud_debug_options_centered_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &text,
                TITLE_Y + 12 + i32::try_from(line).unwrap_or(0) * 10,
                hud_argb_to_tint(0xFF_AA_AA_AA),
            );
        }
    }
    if !screen.rows.is_empty()
        && push_hud_rect_scissor(
            commands,
            surface_size,
            absolute_hud_rect(
                layout.list.x as f32,
                layout.list.y as f32,
                layout.list.width.max(0) as u32,
                layout.list.height.max(0) as u32,
            ),
        )
    {
        for (row, (_, rect)) in screen.rows.iter().zip(&layout.rows) {
            if !rect.intersects(layout.list) {
                continue;
            }
            let highlighted = screen.focus == row.focus || screen.hovered == Some(row.focus);
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
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
                if row.selected {
                    [0.16, 0.28, 0.58, 0.95]
                } else if highlighted {
                    [0.34, 0.34, 0.34, 0.95]
                } else {
                    [0.18, 0.18, 0.18, 0.92]
                },
            );
            push_hud_debug_options_text(
                vertices,
                commands,
                white_pixel,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                &row.title,
                (
                    (rect.x + 5) as f32,
                    (rect.y + if row.detail.is_some() { 3 } else { 7 }) as f32,
                ),
                if row.enabled {
                    HUD_TINT_WHITE
                } else {
                    [0.5, 0.5, 0.5, 1.0]
                },
            );
            if let Some(detail) = row.detail.as_ref().and_then(|detail| {
                hud_wrap_plain_text(detail, rect.width.saturating_sub(28).max(1) as u32, glyphs)
                    .into_iter()
                    .next()
            }) {
                push_hud_debug_options_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    &detail,
                    ((rect.x + 5) as f32, (rect.y + 13) as f32),
                    if row.enabled {
                        [0.65, 0.65, 0.65, 1.0]
                    } else {
                        [0.4, 0.4, 0.4, 1.0]
                    },
                );
            }
            if row.selected {
                push_hud_debug_options_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    "✓",
                    ((rect.x + rect.width - 14) as f32, (rect.y + 7) as f32),
                    HUD_TINT_WHITE,
                );
            }
        }
        commands.push(HudDrawCommand::Scissor {
            x: 0,
            y: 0,
            width: surface_size.width.max(1),
            height: surface_size.height.max(1),
        });
    }
    for (control, (_, rect)) in screen.controls.iter().zip(&layout.controls) {
        let highlighted = screen.focus == control.focus || screen.hovered == Some(control.focus);
        match &control.kind {
            HudPlayerReportingControlKind::Button => push_hud_debug_options_button(
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
                &control.label,
                control.enabled,
                highlighted,
                HUD_TINT_WHITE,
                button_sprites,
            ),
            HudPlayerReportingControlKind::Checkbox { selected } => {
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
                    &format!(
                        "{} {}",
                        if *selected { "[x]" } else { "[ ]" },
                        control.label
                    ),
                    control.enabled,
                    highlighted,
                    HUD_TINT_WHITE,
                    button_sprites,
                );
            }
            HudPlayerReportingControlKind::TextInput {
                value,
                placeholder,
                cursor_visible,
                ..
            } => {
                push_hud_debug_options_text(
                    vertices,
                    commands,
                    white_pixel,
                    font_atlas,
                    glyphs,
                    obfuscated_pool,
                    frame_index,
                    surface_size,
                    &control.label,
                    ((rect.x + 4) as f32, (rect.y + 2) as f32),
                    HUD_TINT_WHITE,
                );
                let input_y = rect.y + 12;
                let input_height = rect.height.saturating_sub(12);
                push_hud_draw_with_uv_and_tint(
                    vertices,
                    commands,
                    white_pixel,
                    surface_size,
                    absolute_hud_rect(
                        rect.x as f32,
                        input_y as f32,
                        rect.width.max(0) as u32,
                        input_height.max(0) as u32,
                    ),
                    HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    if highlighted {
                        [0.03, 0.03, 0.03, 1.0]
                    } else {
                        [0.08, 0.08, 0.08, 1.0]
                    },
                );
                let text = if value.is_empty() { placeholder } else { value };
                for (line, text) in
                    hud_wrap_plain_text(text, rect.width.saturating_sub(8).max(1) as u32, glyphs)
                        .into_iter()
                        .take(5)
                        .enumerate()
                {
                    push_hud_debug_options_text(
                        vertices,
                        commands,
                        white_pixel,
                        font_atlas,
                        glyphs,
                        obfuscated_pool,
                        frame_index,
                        surface_size,
                        &text,
                        (
                            (rect.x + 4) as f32,
                            (input_y + 4 + i32::try_from(line).unwrap_or(0) * 9) as f32,
                        ),
                        if value.is_empty() {
                            [0.55, 0.55, 0.55, 1.0]
                        } else {
                            HUD_TINT_WHITE
                        },
                    );
                }
                if highlighted && *cursor_visible {
                    push_hud_debug_options_text(
                        vertices,
                        commands,
                        white_pixel,
                        font_atlas,
                        glyphs,
                        obfuscated_pool,
                        frame_index,
                        surface_size,
                        "_",
                        ((rect.x + 4) as f32, (input_y + input_height - 12) as f32),
                        HUD_TINT_WHITE,
                    );
                }
            }
        }
    }
    if let Some(status) = &screen.status {
        let tint = match screen.kind {
            HudPlayerReportingScreenKind::Failure => hud_argb_to_tint(0xFF_FF_55_55),
            HudPlayerReportingScreenKind::Success => hud_argb_to_tint(0xFF_55_FF_55),
            _ => hud_argb_to_tint(0xFF_AA_AA_AA),
        };
        push_hud_debug_options_centered_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            status,
            i32::try_from(surface_size.height)
                .unwrap_or(i32::MAX)
                .saturating_sub(48),
            tint,
        );
    }
}

impl Renderer {
    pub fn set_hud_player_reporting_screen(&mut self, screen: Option<HudPlayerReportingScreen>) {
        self.hud_player_reporting_screen = screen.map(sanitize_hud_player_reporting_screen);
    }

    pub fn hud_player_reporting_hit_target_at(
        &self,
        position: (f64, f64),
    ) -> Option<HudPlayerReportingHitTarget> {
        hud_player_reporting_hit_target_at(
            self.hud_player_reporting_screen.as_ref()?,
            self.surface_size(),
            position,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> HudPlayerReportingScreen {
        HudPlayerReportingScreen {
            generation: 7,
            kind: HudPlayerReportingScreenKind::ChatSelection,
            title: "Select Chat Messages".to_string(),
            message: None,
            status: None,
            rows: vec![HudPlayerReportingRow {
                focus: HudPlayerReportingFocus::Message(3),
                title: "signed message".to_string(),
                detail: None,
                selected: false,
                enabled: true,
            }],
            controls: vec![HudPlayerReportingControl {
                focus: HudPlayerReportingFocus::Done,
                label: "Done".to_string(),
                enabled: true,
                placement: HudPlayerReportingControlPlacement::Footer,
                kind: HudPlayerReportingControlKind::Button,
            }],
            scroll_amount: 0,
            focus: HudPlayerReportingFocus::Message(3),
            hovered: None,
        }
    }

    #[test]
    fn hud_player_reporting_hit_test_tags_current_generation_and_visible_row() {
        let screen = screen();
        let size = PhysicalSize::new(640, 360);
        let layout = hud_player_reporting_layout(&screen, size);
        let row = layout.rows[0].1;
        assert_eq!(
            hud_player_reporting_hit_target_at(
                &screen,
                size,
                (f64::from(row.x + 1), f64::from(row.y + 1)),
            ),
            Some(HudPlayerReportingHitTarget {
                generation: 7,
                focus: HudPlayerReportingFocus::Message(3),
            })
        );
    }

    #[test]
    fn hud_player_reporting_sanitization_bounds_rows_text_and_cursor() {
        let mut screen = screen();
        screen.title = "x".repeat(MAX_TEXT_CHARS + 1);
        screen.rows = vec![screen.rows[0].clone(); MAX_ROWS + 1];
        screen.controls[0].kind = HudPlayerReportingControlKind::TextInput {
            value: "abc".to_string(),
            placeholder: String::new(),
            cursor: 99,
            selection: 99,
            cursor_visible: true,
        };
        let screen = sanitize_hud_player_reporting_screen(screen);
        assert_eq!(screen.title.chars().count(), MAX_TEXT_CHARS);
        assert_eq!(screen.rows.len(), MAX_ROWS);
        let HudPlayerReportingControlKind::TextInput {
            cursor, selection, ..
        } = screen.controls[0].kind
        else {
            unreachable!()
        };
        assert_eq!((cursor, selection), (3, 3));
    }
}
