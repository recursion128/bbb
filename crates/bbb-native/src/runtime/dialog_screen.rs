use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::CustomClickAction;
use bbb_renderer::{
    HudInventoryBackgroundLayer, HudInventoryBackgroundTexture, HudInventoryScreen,
    HudInventoryTextLabel, HudInventoryTooltip, HudInventoryTooltipLine, HudStyledTextRun,
    HudTextStyle, HudUvRect,
};
use bbb_world::{
    CommonDialogDataState, DialogActionState, DialogBodyState, DialogButtonState, DialogDataState,
    DialogState, DialogTextState, WorldStore,
};
use tokio::sync::mpsc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::input::DebugClipboard;

use super::hud_ascii_approx_char_width;

// Vanilla `HeaderAndFooterLayout` uses 33-pixel header/footer frames and places
// content 30 pixels below the header. `DialogScreen` stacks body elements at a
// 10-pixel gap; vanilla font lines are 9 pixels high.
const HEADER_HEIGHT: i32 = 33;
const FOOTER_HEIGHT: i32 = 33;
const CONTENT_MARGIN_TOP: i32 = 30;
const BODY_SPACING: i32 = 10;
const SIMPLE_BUTTON_SPACING: i32 = 8;
const BUTTON_HEIGHT: u32 = 20;
const BUTTON_TEXT_Y_OFFSET: i32 = 6;
const TEXT_LINE_HEIGHT: i32 = 9;
const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const INACTIVE_TEXT_COLOR: [f32; 4] = [160.0 / 255.0, 160.0 / 255.0, 160.0 / 255.0, 1.0];
const WAITING_BUTTON_WIDTH: u32 = 200;
const WAITING_BUTTON_VISIBLE_TICKS: u32 = 21;
const WAITING_BUTTON_ACTIVE_TICKS: u32 = 101;
const WAITING_TITLE: &str = "Waiting for Server";

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogTextLine {
    text: String,
    width: u32,
    runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogButtonLayout {
    x: i32,
    y: i32,
    width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SimpleDialogLayout {
    screen_width: u32,
    screen_height: u32,
    buttons: Vec<DialogButtonLayout>,
}

impl SimpleDialogLayout {
    fn new(action_widths: &[i32], surface_size: PhysicalSize<u32>) -> Option<Self> {
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        let screen_width_i32 = i32::try_from(screen_width).ok()?;
        let screen_height_i32 = i32::try_from(screen_height).ok()?;
        let button_widths = action_widths
            .iter()
            .map(|width| u32::try_from(*width).ok().map(|width| width.clamp(1, 512)))
            .collect::<Option<Vec<_>>>()?;
        let spacing_width = SIMPLE_BUTTON_SPACING.saturating_mul(
            i32::try_from(button_widths.len().saturating_sub(1)).unwrap_or(i32::MAX),
        );
        let total_width = button_widths.iter().fold(spacing_width, |total, width| {
            total.saturating_add(i32::try_from(*width).unwrap_or(i32::MAX))
        });
        let button_y =
            screen_height_i32 - FOOTER_HEIGHT + (FOOTER_HEIGHT - BUTTON_HEIGHT as i32) / 2;
        // Notice and Confirmation have at most two codec-bounded buttons, so this
        // subtraction is bounded while preserving vanilla's negative centered x
        // when the declared button row is wider than the screen.
        let mut button_x = (screen_width_i32 - total_width) / 2;
        let buttons = button_widths
            .into_iter()
            .map(|width| {
                let button = DialogButtonLayout {
                    x: button_x,
                    y: button_y,
                    width,
                };
                button_x = button_x
                    .saturating_add(i32::try_from(width).unwrap_or(i32::MAX))
                    .saturating_add(SIMPLE_BUTTON_SPACING);
                button
            })
            .collect();
        Some(Self {
            screen_width,
            screen_height,
            buttons,
        })
    }

    fn button_at(
        &self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> Option<usize> {
        self.buttons.iter().position(|button| {
            screen_button_contains(
                self.screen_width,
                self.screen_height,
                button.x,
                button.y,
                button.width,
                cursor_position,
                surface_size,
            )
        })
    }
}

struct SimpleDialogControls<'a> {
    common: &'a CommonDialogDataState,
    actions: Vec<&'a DialogButtonState>,
    cancel: &'a DialogButtonState,
}

fn simple_dialog_controls(data: &DialogDataState) -> Option<SimpleDialogControls<'_>> {
    match data {
        DialogDataState::Notice { common, action } => Some(SimpleDialogControls {
            common,
            actions: vec![action],
            cancel: action,
        }),
        DialogDataState::Confirmation { common, yes, no } => Some(SimpleDialogControls {
            common,
            actions: vec![yes, no],
            cancel: no,
        }),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WaitingLayout {
    screen_width: u32,
    screen_height: u32,
    button_x: i32,
    button_y: i32,
}

impl WaitingLayout {
    fn new(surface_size: PhysicalSize<u32>) -> Self {
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        let screen_width_i32 = i32::try_from(screen_width).unwrap_or(i32::MAX);
        let screen_height_i32 = i32::try_from(screen_height).unwrap_or(i32::MAX);
        Self {
            screen_width,
            screen_height,
            button_x: (screen_width_i32 - WAITING_BUTTON_WIDTH as i32) / 2,
            button_y: (HEADER_HEIGHT + CONTENT_MARGIN_TOP)
                .min(screen_height_i32 - BUTTON_HEIGHT as i32),
        }
    }

    fn button_contains(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        screen_button_contains(
            self.screen_width,
            self.screen_height,
            self.button_x,
            self.button_y,
            WAITING_BUTTON_WIDTH,
            cursor_position,
            surface_size,
        )
    }
}

pub(super) fn hud_dialog_screen(
    dialog: &DialogState,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> Option<HudInventoryScreen> {
    let controls = simple_dialog_controls(dialog.data.as_ref()?)?;
    let layout = SimpleDialogLayout::new(
        &controls
            .actions
            .iter()
            .map(|action| action.width)
            .collect::<Vec<_>>(),
        surface_size,
    )?;
    let screen_width_i32 = i32::try_from(layout.screen_width).ok()?;
    let screen_height_i32 = i32::try_from(layout.screen_height).ok()?;
    let cursor_screen_position = cursor_position;
    let cursor_position =
        cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted_button = layout.button_at(cursor_position, surface_size);

    let mut text_labels = Vec::new();
    push_centered_text_label(
        &mut text_labels,
        &controls.common.title,
        screen_width_i32 / 2,
        (HEADER_HEIGHT - TEXT_LINE_HEIGHT) / 2,
    );

    let body_blocks = controls
        .common
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

    for (action, button) in controls.actions.iter().zip(&layout.buttons) {
        push_centered_text_label(
            &mut text_labels,
            &action.label,
            button.x + i32::try_from(button.width).unwrap_or(i32::MAX) / 2,
            button.y + BUTTON_TEXT_Y_OFFSET,
        );
    }

    let background_layers = layout
        .buttons
        .iter()
        .enumerate()
        .map(|(index, button)| HudInventoryBackgroundLayer {
            texture: if highlighted_button == Some(index) {
                HudInventoryBackgroundTexture::WidgetButtonHighlighted
            } else {
                HudInventoryBackgroundTexture::WidgetButton
            },
            x: button.x,
            y: button.y,
            width: button.width,
            height: BUTTON_HEIGHT,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        })
        .collect();
    let tooltip = highlighted_button.and_then(|index| {
        let tooltip = controls.actions.get(index)?.tooltip.as_ref()?;
        let (cursor_x, cursor_y) = cursor_screen_position?;
        let (origin_x, origin_y) =
            centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
        Some(HudInventoryTooltip {
            slot_id: u16::try_from(index).unwrap_or(u16::MAX),
            x: i32::try_from(i64::from(cursor_x) - origin_x).unwrap_or_else(|_| {
                if i64::from(cursor_x) < origin_x {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }),
            y: i32::try_from(i64::from(cursor_y) - origin_y).unwrap_or_else(|_| {
                if i64::from(cursor_y) < origin_y {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }),
            lines: wrapped_text_lines(tooltip, 170)
                .into_iter()
                .map(|line| HudInventoryTooltipLine {
                    text: line.text,
                    tint: TEXT_COLOR,
                    runs: line.runs,
                })
                .collect(),
        })
    });

    Some(HudInventoryScreen {
        width: layout.screen_width,
        height: layout.screen_height,
        background_layers,
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        text_labels,
        hovered_slot_id: None,
        tooltip,
    })
}

pub(super) fn hud_waiting_response_screen(
    waiting_ticks: u32,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> HudInventoryScreen {
    let layout = WaitingLayout::new(surface_size);
    let screen_width_i32 = i32::try_from(layout.screen_width).unwrap_or(i32::MAX);
    let visible = waiting_ticks >= WAITING_BUTTON_VISIBLE_TICKS;
    let active = waiting_ticks >= WAITING_BUTTON_ACTIVE_TICKS;
    let cursor_position =
        cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted = active && layout.button_contains(cursor_position, surface_size);
    let mut text_labels = Vec::new();
    push_centered_plain_text_label(
        &mut text_labels,
        WAITING_TITLE,
        screen_width_i32 / 2,
        (HEADER_HEIGHT - TEXT_LINE_HEIGHT) / 2,
        TEXT_COLOR,
    );
    let mut background_layers = Vec::new();
    if visible {
        let seconds_visible = waiting_ticks.saturating_sub(1) / 20;
        let button_text = if active {
            "Back".to_string()
        } else {
            format!("Back ({}s)", 5u32.saturating_sub(seconds_visible))
        };
        background_layers.push(HudInventoryBackgroundLayer {
            texture: if highlighted {
                HudInventoryBackgroundTexture::WidgetButtonHighlighted
            } else if active {
                HudInventoryBackgroundTexture::WidgetButton
            } else {
                HudInventoryBackgroundTexture::WidgetButtonDisabled
            },
            x: layout.button_x,
            y: layout.button_y,
            width: WAITING_BUTTON_WIDTH,
            height: BUTTON_HEIGHT,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        });
        push_centered_plain_text_label(
            &mut text_labels,
            &button_text,
            layout.button_x + WAITING_BUTTON_WIDTH as i32 / 2,
            layout.button_y + BUTTON_TEXT_Y_OFFSET,
            if active {
                TEXT_COLOR
            } else {
                INACTIVE_TEXT_COLOR
            },
        );
    }

    HudInventoryScreen {
        width: layout.screen_width,
        height: layout.screen_height,
        background_layers,
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
    }
}

pub(crate) fn handle_dialog_key_input(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
) -> bool {
    if world.dialog_waiting_ticks().is_some() {
        if state == ElementState::Pressed
            && physical_key == PhysicalKey::Code(KeyCode::Escape)
            && world.dialog_waiting_can_close()
        {
            world.close_dialog_waiting_response();
        }
        return true;
    }
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    if state != ElementState::Pressed || physical_key != PhysicalKey::Code(KeyCode::Escape) {
        return true;
    }
    let Some(controls) = dialog.data.as_ref().and_then(simple_dialog_controls) else {
        return true;
    };
    if !controls.common.can_close_with_escape {
        return true;
    }
    let action = controls.cancel.action_data.clone();
    run_dialog_action(
        counters,
        world,
        net_commands,
        clipboard,
        action.as_ref(),
        "close",
    );
    true
}

pub(crate) fn handle_dialog_mouse_input(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if world.dialog_waiting_ticks().is_some() {
        if button == MouseButton::Left
            && state == ElementState::Pressed
            && world.dialog_waiting_can_close()
            && WaitingLayout::new(surface_size).button_contains(cursor_position, surface_size)
        {
            world.close_dialog_waiting_response();
        }
        return true;
    }
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    if button != MouseButton::Left || state != ElementState::Pressed {
        return true;
    }
    let Some(controls) = dialog.data.as_ref().and_then(simple_dialog_controls) else {
        return true;
    };
    let Some(action_index) = SimpleDialogLayout::new(
        &controls
            .actions
            .iter()
            .map(|action| action.width)
            .collect::<Vec<_>>(),
        surface_size,
    )
    .and_then(|layout| layout.button_at(cursor_position, surface_size)) else {
        return true;
    };
    let action = (
        controls.actions[action_index].action_data.clone(),
        controls.common.after_action.clone(),
    );
    run_dialog_action(
        counters,
        world,
        net_commands,
        clipboard,
        action.0.as_ref(),
        &action.1,
    );
    true
}

fn run_dialog_action(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    action: Option<&DialogActionState>,
    after_action: &str,
) -> bool {
    match action {
        None => {}
        Some(DialogActionState::Custom { id, payload }) => {
            if let Some(tx) = net_commands {
                if tx
                    .try_send(NetCommand::CustomClickAction(CustomClickAction {
                        id: id.clone(),
                        payload: payload.clone(),
                    }))
                    .is_ok()
                {
                    counters.custom_click_action_commands_queued += 1;
                }
            }
        }
        Some(DialogActionState::ShowDialog { dialog }) => {
            // Vanilla computes an active screen from `after_action`, then
            // `showDialog` unwraps an active Dialog/Waiting screen back to its
            // original previous screen. All three after-action variants
            // therefore replace the current dialog while retaining that owner.
            return world.show_dialog_from_action((**dialog).clone());
        }
        Some(DialogActionState::CopyToClipboard { value }) => {
            // KeyboardHandler.setClipboard intentionally ignores empty text.
            if !value.is_empty() {
                if let Some(clipboard) = clipboard {
                    clipboard.set_debug_clipboard_text(value);
                }
            }
        }
        Some(DialogActionState::ChangePage { .. }) => {
            // DialogScreen delegates this event to Screen.defaultHandleClickEvent,
            // which has no ChangePage arm. The BookViewScreen override is not
            // consulted, but the computed after-action screen is still applied.
        }
        Some(_) => return false,
    }
    match after_action {
        "close" => world.close_dialog(),
        "none" => true,
        "wait_for_response" => world.wait_for_dialog_response(),
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn screen_button_contains(
    screen_width: u32,
    screen_height: u32,
    button_x: i32,
    button_y: i32,
    button_width: u32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let button_left = origin_x + i64::from(button_x);
    let button_top = origin_y + i64::from(button_y);
    cursor.x >= button_left as f64
        && cursor.x < (button_left + i64::from(button_width)) as f64
        && cursor.y >= button_top as f64
        && cursor.y < (button_top + i64::from(BUTTON_HEIGHT)) as f64
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

fn push_centered_plain_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: &str,
    center_x: i32,
    y: i32,
    tint: [f32; 4],
) {
    let width = text.chars().fold(0u32, |width, ch| {
        width.saturating_add(char_approx_width(ch, HudTextStyle::default()))
    });
    labels.push(HudInventoryTextLabel {
        x: center_x - i32::try_from(width).unwrap_or(i32::MAX) / 2,
        y,
        width: width.max(1),
        text: text.to_string(),
        tint,
        background: None,
        input: None,
        shadow: false,
        runs: Vec::new(),
    });
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

    #[derive(Default)]
    struct MockDialogClipboard {
        text: Option<String>,
        rejects_write: bool,
    }

    impl DebugClipboard for MockDialogClipboard {
        fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
            if self.rejects_write {
                return false;
            }
            self.text = Some(text.to_string());
            true
        }
    }

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
        let mut counters = NetCounters::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
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
        let mut counters = NetCounters::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
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
        let mut counters = NetCounters::default();
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut default_notice,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(default_notice.current_dialog().is_none());

        let mut locked_notice = world_with_notice(vec![entry(
            "can_close_with_escape",
            NbtSummaryValue::Byte(0),
        )]);
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut locked_notice,
            &None,
            None,
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
                            NbtSummaryValue::String("minecraft:run_command".to_string()),
                        ),
                        entry(
                            "command",
                            NbtSummaryValue::String("say deferred".to_string()),
                        ),
                    ]),
                ),
            ]),
        )]);
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut action_notice,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(action_notice.current_dialog().is_some());
    }

    #[test]
    fn confirmation_projects_two_buttons_and_escape_runs_no_button_action() {
        let mut world = world_with_confirmation(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "yes",
                custom_button("Yes", Some("Proceed"), 80, "example:yes", None),
            ),
            entry("no", custom_button("No", None, 100, "example:no", None)),
        ]);
        let screen = hud_dialog_screen(
            world.current_dialog().unwrap(),
            Some((100, 220)),
            PhysicalSize::new(320, 240),
        )
        .unwrap();

        assert_eq!(screen.background_layers.len(), 2);
        assert_eq!(screen.background_layers[0].x, 66);
        assert_eq!(screen.background_layers[0].width, 80);
        assert_eq!(
            screen.background_layers[0].texture,
            HudInventoryBackgroundTexture::WidgetButtonHighlighted
        );
        assert_eq!(screen.background_layers[1].x, 154);
        assert_eq!(screen.background_layers[1].width, 100);
        assert_eq!(screen.text_labels[1].text, "Yes");
        assert_eq!(screen.text_labels[2].text, "No");
        let tooltip = screen.tooltip.as_ref().unwrap();
        assert_eq!((tooltip.x, tooltip.y), (100, 220));
        assert_eq!(tooltip.lines.len(), 1);
        assert_eq!(tooltip.lines[0].text, "Proceed");

        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(2);
        let net_commands = Some(tx);
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(100.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));
        assert!(world.current_dialog().is_some());
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::CustomClickAction(CustomClickAction { id, .. }) if id == "example:yes"
        ));

        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(world.current_dialog().is_none());
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::CustomClickAction(CustomClickAction { id, .. }) if id == "example:no"
        ));
        assert_eq!(counters.custom_click_action_commands_queued, 2);
    }

    #[test]
    fn show_dialog_action_replaces_outer_dialog_and_preserves_previous_screen() {
        let mut world = WorldStore::new();
        assert!(world.open_advancements_screen());
        apply_notice(
            &mut world,
            vec![
                entry(
                    "after_action",
                    NbtSummaryValue::String("wait_for_response".to_string()),
                ),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry("label", NbtSummaryValue::String("Open".to_string())),
                        entry(
                            "action",
                            NbtSummaryValue::Compound(vec![
                                entry(
                                    "type",
                                    NbtSummaryValue::String("minecraft:show_dialog".to_string()),
                                ),
                                entry(
                                    "dialog",
                                    NbtSummaryValue::Compound(vec![
                                        entry(
                                            "type",
                                            NbtSummaryValue::String("minecraft:notice".to_string()),
                                        ),
                                        entry(
                                            "title",
                                            NbtSummaryValue::String("Inner".to_string()),
                                        ),
                                    ]),
                                ),
                            ]),
                        ),
                    ]),
                ),
            ],
        );
        let mut counters = NetCounters::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));

        let Some(DialogDataState::Notice { common, .. }) = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
        else {
            panic!("expected inner Notice dialog");
        };
        assert_eq!(common.title.text, "Inner");
        assert_eq!(world.dialog_waiting_ticks(), None);
        assert!(!world.advancements_screen_is_open());
        assert_eq!(world.counters().show_dialog_packets, 1);
        assert!(world.close_dialog());
        assert!(world.advancements_screen_is_open());
    }

    #[test]
    fn copy_to_clipboard_action_writes_non_empty_text_before_transition() {
        let mut world = world_with_notice(copy_action_fields("copied text"));
        let mut counters = NetCounters::default();
        let mut clipboard = MockDialogClipboard::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            Some(&mut clipboard),
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));

        assert_eq!(clipboard.text.as_deref(), Some("copied text"));
        assert!(world.current_dialog().is_some());

        let mut empty_world = world_with_notice(copy_action_fields(""));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut empty_world,
            &None,
            Some(&mut clipboard),
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));
        assert_eq!(clipboard.text.as_deref(), Some("copied text"));
        assert!(empty_world.current_dialog().is_some());

        let mut closing_fields = copy_action_fields("not copied");
        closing_fields[1] = entry("after_action", NbtSummaryValue::String("close".to_string()));
        let mut closing_world = world_with_notice(closing_fields);
        let mut rejecting_clipboard = MockDialogClipboard {
            text: Some("unchanged".to_string()),
            rejects_write: true,
        };
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut closing_world,
            &None,
            Some(&mut rejecting_clipboard),
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));
        assert_eq!(rejecting_clipboard.text.as_deref(), Some("unchanged"));
        assert!(closing_world.current_dialog().is_none());
    }

    #[test]
    fn change_page_dialog_action_only_applies_after_action() {
        let mut world = world_with_notice(vec![
            entry(
                "after_action",
                NbtSummaryValue::String("wait_for_response".to_string()),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Page".to_string())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:change_page".to_string()),
                            ),
                            entry("page", NbtSummaryValue::Int(7)),
                        ]),
                    ),
                ]),
            ),
        ]);
        let mut counters = NetCounters::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));

        assert_eq!(world.current_dialog(), None);
        assert_eq!(world.dialog_waiting_ticks(), Some(0));
    }

    #[test]
    fn static_custom_notice_action_queues_exact_payload_then_closes() {
        let mut world = world_with_notice(custom_action_fields());
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(1);

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &Some(tx),
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));

        assert!(world.current_dialog().is_none());
        assert_eq!(counters.custom_click_action_commands_queued, 1);
        let command = rx.try_recv().unwrap();
        assert!(matches!(
            command,
            NetCommand::CustomClickAction(CustomClickAction {
                id,
                payload: Some(NbtSummaryValue::Int(7)),
            }) if id == "example:test"
        ));
    }

    #[test]
    fn static_custom_notice_cancel_queues_action_before_forced_close() {
        let mut world = world_with_notice(custom_action_fields());
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(1);

        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &Some(tx),
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));

        assert!(world.current_dialog().is_none());
        assert_eq!(counters.custom_click_action_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::CustomClickAction(_)
        ));
    }

    fn custom_action_fields() -> Vec<NbtSummaryEntry> {
        vec![entry(
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
                        entry("payload", NbtSummaryValue::Int(7)),
                    ]),
                ),
            ]),
        )]
    }

    fn copy_action_fields(value: &str) -> Vec<NbtSummaryEntry> {
        vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Copy".to_string())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:copy_to_clipboard".to_string()),
                            ),
                            entry("value", NbtSummaryValue::String(value.to_string())),
                        ]),
                    ),
                ]),
            ),
        ]
    }

    fn custom_button(
        label: &str,
        tooltip: Option<&str>,
        width: i32,
        id: &str,
        payload: Option<NbtSummaryValue>,
    ) -> NbtSummaryValue {
        let mut action = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:custom".to_string()),
            ),
            entry("id", NbtSummaryValue::String(id.to_string())),
        ];
        if let Some(payload) = payload {
            action.push(entry("payload", payload));
        }
        let mut button = vec![
            entry("label", NbtSummaryValue::String(label.to_string())),
            entry("width", NbtSummaryValue::Int(width)),
            entry("action", NbtSummaryValue::Compound(action)),
        ];
        if let Some(tooltip) = tooltip {
            button.push(entry(
                "tooltip",
                NbtSummaryValue::String(tooltip.to_string()),
            ));
        }
        NbtSummaryValue::Compound(button)
    }

    #[test]
    fn wait_for_response_countdown_enables_back_after_five_seconds() {
        let mut world = world_with_notice(vec![entry(
            "after_action",
            NbtSummaryValue::String("wait_for_response".to_string()),
        )]);
        let mut counters = NetCounters::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            PhysicalSize::new(320, 240),
        ));
        assert_eq!(world.dialog_waiting_ticks(), Some(0));

        let hidden = hud_waiting_response_screen(20, None, PhysicalSize::new(320, 240));
        assert!(hidden.background_layers.is_empty());
        assert_eq!(hidden.text_labels.len(), 1);

        let first_countdown = hud_waiting_response_screen(21, None, PhysicalSize::new(320, 240));
        assert_eq!(first_countdown.text_labels[1].text, "Back (4s)");
        assert_eq!(
            first_countdown.background_layers[0].texture,
            HudInventoryBackgroundTexture::WidgetButtonDisabled
        );
        let last_countdown = hud_waiting_response_screen(100, None, PhysicalSize::new(320, 240));
        assert_eq!(last_countdown.text_labels[1].text, "Back (1s)");

        world.advance_dialog_waiting_ticks(100);
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert_eq!(world.dialog_waiting_ticks(), Some(100));

        world.advance_dialog_waiting_ticks(1);
        let active = hud_waiting_response_screen(101, Some((160, 64)), PhysicalSize::new(320, 240));
        assert_eq!(active.text_labels[1].text, "Back");
        assert_eq!(
            active.background_layers[0].texture,
            HudInventoryBackgroundTexture::WidgetButtonHighlighted
        );
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(!world.dialog_screen_is_open());

        let mut mouse_world = world_with_notice(vec![entry(
            "after_action",
            NbtSummaryValue::String("wait_for_response".to_string()),
        )]);
        assert!(mouse_world.wait_for_dialog_response());
        mouse_world.advance_dialog_waiting_ticks(101);
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut mouse_world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 64.0)),
            PhysicalSize::new(320, 240),
        ));
        assert!(!mouse_world.dialog_screen_is_open());
    }

    fn world_with_notice(fields: Vec<NbtSummaryEntry>) -> WorldStore {
        let mut world = WorldStore::new();
        apply_notice(&mut world, fields);
        world
    }

    fn apply_notice(world: &mut WorldStore, mut fields: Vec<NbtSummaryEntry>) {
        let mut dialog = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:notice".to_string()),
            ),
            entry("title", NbtSummaryValue::String("Notice".to_string())),
        ];
        dialog.append(&mut fields);
        world.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(dialog),
            },
        });
    }

    fn world_with_confirmation(mut fields: Vec<NbtSummaryEntry>) -> WorldStore {
        let mut dialog = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:confirmation".to_string()),
            ),
            entry("title", NbtSummaryValue::String("Confirm".to_string())),
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
