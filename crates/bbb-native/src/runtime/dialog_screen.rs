use bbb_renderer::{
    HudInventoryBackgroundLayer, HudInventoryBackgroundTexture, HudInventoryScreen,
    HudInventoryTextLabel, HudStyledTextRun, HudTextStyle, HudUvRect,
};
use bbb_world::{DialogBodyState, DialogDataState, DialogState, DialogTextState, WorldStore};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

use super::hud_ascii_approx_char_width;

// Vanilla `HeaderAndFooterLayout` uses 33-pixel header/footer frames and places
// content 30 pixels below the header. `DialogScreen` stacks body elements at a
// 10-pixel gap; vanilla font lines are 9 pixels high.
const HEADER_HEIGHT: i32 = 33;
const FOOTER_HEIGHT: i32 = 33;
const CONTENT_MARGIN_TOP: i32 = 30;
const BODY_SPACING: i32 = 10;
const BUTTON_HEIGHT: u32 = 20;
const BUTTON_TEXT_Y_OFFSET: i32 = 6;
const TEXT_LINE_HEIGHT: i32 = 9;
const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogTextLine {
    text: String,
    width: u32,
    runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NoticeLayout {
    screen_width: u32,
    screen_height: u32,
    button_x: i32,
    button_y: i32,
    button_width: u32,
}

impl NoticeLayout {
    fn new(action_width: i32, surface_size: PhysicalSize<u32>) -> Option<Self> {
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        let screen_width_i32 = i32::try_from(screen_width).ok()?;
        let screen_height_i32 = i32::try_from(screen_height).ok()?;
        let button_width = u32::try_from(action_width).ok()?.clamp(1, 512);
        let button_width_i32 = i32::try_from(button_width).ok()?;
        Some(Self {
            screen_width,
            screen_height,
            button_x: (screen_width_i32 - button_width_i32) / 2,
            button_y: screen_height_i32 - FOOTER_HEIGHT
                + (FOOTER_HEIGHT - BUTTON_HEIGHT as i32) / 2,
            button_width,
        })
    }

    fn button_contains(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(cursor) = cursor_position else {
            return false;
        };
        let (origin_x, origin_y) =
            centered_screen_origin(surface_size, self.screen_width, self.screen_height);
        let button_left = origin_x + i64::from(self.button_x);
        let button_top = origin_y + i64::from(self.button_y);
        cursor.x >= button_left as f64
            && cursor.x < (button_left + i64::from(self.button_width)) as f64
            && cursor.y >= button_top as f64
            && cursor.y < (button_top + i64::from(BUTTON_HEIGHT)) as f64
    }
}

pub(super) fn hud_dialog_screen(
    dialog: &DialogState,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> Option<HudInventoryScreen> {
    let DialogDataState::Notice { common, action } = dialog.data.as_ref()? else {
        return None;
    };
    let layout = NoticeLayout::new(action.width, surface_size)?;
    let screen_width_i32 = i32::try_from(layout.screen_width).ok()?;
    let screen_height_i32 = i32::try_from(layout.screen_height).ok()?;
    let button_width_i32 = i32::try_from(layout.button_width).ok()?;
    let button_highlighted = layout.button_contains(
        cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y))),
        surface_size,
    );

    let mut text_labels = Vec::new();
    push_centered_text_label(
        &mut text_labels,
        &common.title,
        screen_width_i32 / 2,
        (HEADER_HEIGHT - TEXT_LINE_HEIGHT) / 2,
    );

    let body_blocks = common
        .body_data
        .iter()
        .filter_map(|body| match body {
            DialogBodyState::PlainMessage { message } => {
                let max_width = u32::try_from(message.width)
                    .ok()?
                    .clamp(1, layout.screen_width);
                Some(wrapped_text_lines(&message.contents, max_width))
            }
            DialogBodyState::Item { .. } => None,
        })
        .filter(|lines| !lines.is_empty())
        .collect::<Vec<_>>();
    let body_height = body_blocks.iter().fold(0i32, |height, lines| {
        height.saturating_add(
            i32::try_from(lines.len())
                .unwrap_or(i32::MAX)
                .saturating_mul(TEXT_LINE_HEIGHT),
        )
    });
    let body_height = body_height.saturating_add(
        i32::try_from(body_blocks.len().saturating_sub(1))
            .unwrap_or(i32::MAX)
            .saturating_mul(BODY_SPACING),
    );
    let mut body_y = (HEADER_HEIGHT + CONTENT_MARGIN_TOP).min(
        screen_height_i32
            .saturating_sub(FOOTER_HEIGHT)
            .saturating_sub(body_height),
    );
    for (block_index, lines) in body_blocks.into_iter().enumerate() {
        if block_index > 0 {
            body_y = body_y.saturating_add(BODY_SPACING);
        }
        for line in lines {
            let line_width_i32 = i32::try_from(line.width).unwrap_or(i32::MAX);
            text_labels.push(HudInventoryTextLabel {
                x: screen_width_i32 / 2 - line_width_i32 / 2,
                y: body_y,
                width: line.width.max(1),
                text: line.text,
                tint: TEXT_COLOR,
                background: None,
                input: None,
                shadow: false,
                runs: line.runs,
            });
            body_y = body_y.saturating_add(TEXT_LINE_HEIGHT);
        }
    }

    push_centered_text_label(
        &mut text_labels,
        &action.label,
        layout.button_x + button_width_i32 / 2,
        layout.button_y + BUTTON_TEXT_Y_OFFSET,
    );

    Some(HudInventoryScreen {
        width: layout.screen_width,
        height: layout.screen_height,
        background_layers: vec![HudInventoryBackgroundLayer {
            texture: if button_highlighted {
                HudInventoryBackgroundTexture::WidgetButtonHighlighted
            } else {
                HudInventoryBackgroundTexture::WidgetButton
            },
            x: layout.button_x,
            y: layout.button_y,
            width: layout.button_width,
            height: BUTTON_HEIGHT,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        }],
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        text_labels,
        hovered_slot_id: None,
        tooltip: None,
    })
}

pub(crate) fn handle_dialog_key_input(
    world: &mut WorldStore,
    physical_key: PhysicalKey,
    state: ElementState,
) -> bool {
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    if state != ElementState::Pressed || physical_key != PhysicalKey::Code(KeyCode::Escape) {
        return true;
    }
    let should_close = matches!(
        dialog.data.as_ref(),
        Some(DialogDataState::Notice { common, action })
            if common.can_close_with_escape && action.action_data.is_none()
    );
    if should_close {
        world.close_dialog();
    }
    true
}

pub(crate) fn handle_dialog_mouse_input(
    world: &mut WorldStore,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    if button != MouseButton::Left || state != ElementState::Pressed {
        return true;
    }
    let should_close = match dialog.data.as_ref() {
        Some(DialogDataState::Notice { common, action }) => {
            NoticeLayout::new(action.width, surface_size)
                .is_some_and(|layout| layout.button_contains(cursor_position, surface_size))
                && action.action_data.is_none()
                && common.after_action == "close"
        }
        _ => false,
    };
    if should_close {
        world.close_dialog();
    }
    true
}

fn centered_screen_origin(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> (i64, i64) {
    (
        (i64::from(surface_size.width.max(1)) - i64::from(screen_width)) / 2,
        (i64::from(surface_size.height.max(1)) - i64::from(screen_height)) / 2,
    )
}

fn push_centered_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: &DialogTextState,
    center_x: i32,
    y: i32,
) {
    let runs = dialog_text_runs(text);
    let width = runs_approx_width(&runs).max(1);
    let width_i32 = i32::try_from(width).unwrap_or(i32::MAX);
    labels.push(HudInventoryTextLabel {
        x: center_x - width_i32 / 2,
        y,
        width,
        text: text.text.clone(),
        tint: TEXT_COLOR,
        background: None,
        input: None,
        shadow: false,
        runs,
    });
}

fn wrapped_text_lines(text: &DialogTextState, max_width: u32) -> Vec<DialogTextLine> {
    let mut lines = Vec::new();
    let mut current = empty_text_line();
    let mut ended_with_newline = false;
    for run in dialog_text_runs(text) {
        for ch in run.text.chars() {
            if ch == '\n' {
                lines.push(current);
                current = empty_text_line();
                ended_with_newline = true;
                continue;
            }
            ended_with_newline = false;
            let advance = char_approx_width(ch, run.style);
            if !current.text.is_empty() && current.width.saturating_add(advance) > max_width {
                lines.push(current);
                current = empty_text_line();
                if ch.is_whitespace() {
                    continue;
                }
            }
            current.text.push(ch);
            current.width = current.width.saturating_add(advance);
            if let Some(last) = current
                .runs
                .last_mut()
                .filter(|last| last.style == run.style && last.color == run.color)
            {
                last.text.push(ch);
            } else {
                current.runs.push(HudStyledTextRun {
                    text: ch.to_string(),
                    style: run.style,
                    color: run.color,
                });
            }
        }
    }
    if !current.text.is_empty() || lines.is_empty() || ended_with_newline {
        lines.push(current);
    }
    lines
}

fn empty_text_line() -> DialogTextLine {
    DialogTextLine {
        text: String::new(),
        width: 0,
        runs: Vec::new(),
    }
}

fn dialog_text_runs(text: &DialogTextState) -> Vec<HudStyledTextRun> {
    let runs = text
        .runs
        .iter()
        .map(|run| HudStyledTextRun {
            text: run.text.clone(),
            style: HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
        })
        .collect::<Vec<_>>();
    if runs.is_empty() {
        vec![HudStyledTextRun::plain(text.text.clone())]
    } else {
        runs
    }
}

fn runs_approx_width(runs: &[HudStyledTextRun]) -> u32 {
    runs.iter().fold(0u32, |width, run| {
        run.text.chars().fold(width, |width, ch| {
            width.saturating_add(char_approx_width(ch, run.style))
        })
    })
}

fn char_approx_width(ch: char, style: HudTextStyle) -> u32 {
    hud_ascii_approx_char_width(ch).saturating_add(u32::from(style.bold))
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        packets::{DialogHolder, ShowDialog},
        NbtSummaryEntry, NbtSummaryValue,
    };

    use super::*;

    #[test]
    fn wrapped_text_preserves_explicit_trailing_empty_line() {
        let text = DialogTextState {
            text: "Line\n".to_string(),
            runs: Vec::new(),
            component: NbtSummaryValue::String("Line\n".to_string()),
        };

        let lines = wrapped_text_lines(&text, 200);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Line");
        assert_eq!(lines[1].text, "");
    }

    #[test]
    fn actionless_notice_button_closes_without_counting_clear_packet() {
        let mut world = world_with_notice(Vec::new());

        assert!(handle_dialog_mouse_input(
            &mut world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));

        assert!(world.current_dialog().is_none());
        assert_eq!(world.counters().clear_dialog_packets, 0);
    }

    #[test]
    fn notice_button_honors_none_after_action() {
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
        ]);

        assert!(handle_dialog_mouse_input(
            &mut world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));

        assert!(world.current_dialog().is_some());
    }

    #[test]
    fn notice_escape_requires_permission_and_actionless_cancel() {
        let mut default_notice = world_with_notice(Vec::new());
        assert!(handle_dialog_key_input(
            &mut default_notice,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(default_notice.current_dialog().is_none());

        let mut locked_notice = world_with_notice(vec![entry(
            "can_close_with_escape",
            NbtSummaryValue::Byte(0),
        )]);
        assert!(handle_dialog_key_input(
            &mut locked_notice,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(locked_notice.current_dialog().is_some());

        let mut action_notice = world_with_notice(vec![entry(
            "action",
            NbtSummaryValue::Compound(vec![
                entry("label", NbtSummaryValue::String("Send".to_string())),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "type",
                            NbtSummaryValue::String("minecraft:custom".to_string()),
                        ),
                        entry("id", NbtSummaryValue::String("example:test".to_string())),
                    ]),
                ),
            ]),
        )]);
        assert!(handle_dialog_key_input(
            &mut action_notice,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(action_notice.current_dialog().is_some());
    }

    fn world_with_notice(mut fields: Vec<NbtSummaryEntry>) -> WorldStore {
        let mut dialog = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:notice".to_string()),
            ),
            entry("title", NbtSummaryValue::String("Notice".to_string())),
        ];
        dialog.append(&mut fields);
        let mut world = WorldStore::new();
        world.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(dialog),
            },
        });
        world
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }
}
