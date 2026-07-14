use std::collections::BTreeMap;

use bbb_control::NetCounters;
use bbb_item_model::NativeItemRuntime;
use bbb_net::NetCommand;
use bbb_protocol::{
    java_float_to_string,
    packets::{CustomClickAction, GameType, ItemStackSummary},
    NbtSummaryValue,
};
use bbb_renderer::{
    HudInventoryBackgroundLayer, HudInventoryBackgroundTexture, HudInventoryItem,
    HudInventoryScreen, HudInventoryTextInputDecoration, HudInventoryTextLabel,
    HudInventoryTooltip, HudInventoryTooltipLine, HudStyledTextRun, HudTextStyle, HudUvRect,
    Renderer,
};
use bbb_world::{
    CommonDialogDataState, DialogActionState, DialogBodyState, DialogButtonState,
    DialogCommandTemplateState, DialogDataState, DialogInputControlState, DialogInputValueState,
    DialogItemStackTemplateState, DialogNumberRangeState, DialogPlainMessageState, DialogState,
    DialogTextState, WorldStore,
};
use tokio::sync::mpsc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::input::{queue_chat_command, text_edit, DebugClipboard};
use crate::terrain_runtime::TerrainTextureState;

use super::{
    block_item_3d_model, hud_ascii_approx_char_width, hud_item_icon_for_stack,
    hud_item_tooltip_lines, wall_clock_millis, InventoryHudLocalState,
};

// Vanilla `HeaderAndFooterLayout` uses 33-pixel header/footer frames and places
// content 30 pixels below the header. `DialogScreen` stacks body elements at a
// 10-pixel gap; vanilla font lines are 9 pixels high.
const HEADER_HEIGHT: i32 = 33;
const FOOTER_HEIGHT: i32 = 33;
const CONTENT_MARGIN_TOP: i32 = 30;
const BODY_SPACING: i32 = 10;
const ITEM_DESCRIPTION_SPACING: i32 = 2;
const FOCUSABLE_TEXT_PADDING: i32 = 4;
const SIMPLE_BUTTON_SPACING: i32 = 8;
const BUTTON_HEIGHT: u32 = 20;
const BUTTON_TEXT_Y_OFFSET: i32 = 6;
const CHECKBOX_SIZE: u32 = 17;
const CHECKBOX_LABEL_SPACING: i32 = 4;
const SLIDER_HEIGHT: u32 = 20;
const SLIDER_HANDLE_WIDTH: u32 = 8;
const SLIDER_HANDLE_HALF_WIDTH: i32 = 4;
const WIDGET_TEXT_MARGIN: u32 = 2;
const WIDGET_SPRITE_WIDTH: u32 = 200;
const TEXT_LINE_HEIGHT: i32 = 9;
const TEXT_INPUT_HEIGHT: i32 = 20;
const TEXT_INPUT_INSET: i32 = 4;
const TEXT_INPUT_Y_OFFSET: i32 = 6;
const TEXT_INPUT_COLOR: [f32; 4] = [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0];
const TEXT_SELECTION_TINT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogInputLayout {
    index: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogInputMouseHit {
    Boolean(usize),
    NumberRange(DialogInputLayout),
    SingleOption(DialogInputLayout),
    Text(DialogInputLayout),
}

pub(crate) trait DialogTextMeasurer {
    fn display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize;

    fn cursor_for_width(&self, text: &str, display_start: usize, width: u32) -> usize;
}

impl DialogTextMeasurer for Renderer {
    fn display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        self.hud_plain_text_display_start_for_width(text, scroll_to, width)
            .unwrap_or_else(|| approximate_dialog_text_display_start(text, scroll_to, width))
    }

    fn cursor_for_width(&self, text: &str, display_start: usize, width: u32) -> usize {
        self.hud_plain_text_cursor_for_width_from(text, display_start, width)
            .unwrap_or_else(|| approximate_dialog_text_cursor(text, display_start, width))
    }
}

#[cfg(test)]
struct ApproximateDialogTextMeasurer;

#[cfg(test)]
impl DialogTextMeasurer for ApproximateDialogTextMeasurer {
    fn display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        approximate_dialog_text_display_start(text, scroll_to, width)
    }

    fn cursor_for_width(&self, text: &str, display_start: usize, width: u32) -> usize {
        approximate_dialog_text_cursor(text, display_start, width)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SimpleDialogLayout {
    screen_width: u32,
    screen_height: u32,
    buttons: Vec<DialogButtonLayout>,
}

#[derive(Debug, Clone, PartialEq)]
struct DialogTextBlock {
    width: u32,
    height: i32,
    lines: Vec<DialogTextLine>,
}

#[derive(Debug, Clone, PartialEq)]
struct DialogItemBlock {
    stack: ItemStackSummary,
    item: HudInventoryItem,
    widget_width: i32,
    widget_height: i32,
    description: Option<DialogTextBlock>,
    show_tooltip: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum DialogBodyBlock {
    Text(DialogTextBlock),
    Item(DialogItemBlock),
}

impl DialogBodyBlock {
    fn width(&self) -> i32 {
        match self {
            Self::Text(text) => i32::try_from(text.width).unwrap_or(i32::MAX),
            Self::Item(item) => {
                item.description
                    .as_ref()
                    .map_or(item.widget_width, |description| {
                        item.widget_width
                            .saturating_add(ITEM_DESCRIPTION_SPACING)
                            .saturating_add(i32::try_from(description.width).unwrap_or(i32::MAX))
                    })
            }
        }
    }

    fn height(&self) -> i32 {
        match self {
            Self::Text(text) => text.height,
            Self::Item(item) => item
                .description
                .as_ref()
                .map_or(item.widget_height, |description| {
                    item.widget_height.max(description.height)
                }),
        }
    }
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

fn dialog_body_declared_height(body: &DialogBodyState) -> Option<i32> {
    match body {
        DialogBodyState::PlainMessage { message } => Some(dialog_text_block(message)?.height),
        DialogBodyState::Item {
            description,
            height,
            ..
        } => Some(
            description
                .as_ref()
                .and_then(dialog_text_block)
                .map_or(*height, |text| (*height).max(text.height)),
        ),
    }
}

fn dialog_input_dimensions(control: &DialogInputControlState) -> Option<(i32, i32)> {
    match control {
        DialogInputControlState::Boolean { label, .. } => {
            let label_width = i32::try_from(runs_approx_width(&dialog_text_runs(label))).ok()?;
            Some((
                i32::try_from(CHECKBOX_SIZE)
                    .ok()?
                    .saturating_add(CHECKBOX_LABEL_SPACING)
                    .saturating_add(label_width),
                i32::try_from(CHECKBOX_SIZE).ok()?,
            ))
        }
        DialogInputControlState::NumberRange { width, .. } => {
            Some((*width, i32::try_from(SLIDER_HEIGHT).ok()?))
        }
        DialogInputControlState::SingleOption { width, .. } => {
            Some((*width, i32::try_from(BUTTON_HEIGHT).ok()?))
        }
        DialogInputControlState::Text {
            width,
            label_visible,
            multiline,
            ..
        } => {
            let control_height = multiline.as_ref().map_or(20, |multiline| {
                multiline.height.unwrap_or_else(|| {
                    multiline
                        .max_lines
                        .unwrap_or(4)
                        .saturating_mul(TEXT_LINE_HEIGHT)
                        .saturating_add(8)
                        .min(512)
                })
            });
            Some((
                *width,
                control_height.saturating_add(if *label_visible {
                    TEXT_LINE_HEIGHT + 4
                } else {
                    0
                }),
            ))
        }
    }
}

fn dialog_input_layouts(
    common: &CommonDialogDataState,
    surface_size: PhysicalSize<u32>,
) -> Option<Vec<DialogInputLayout>> {
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let screen_width_i32 = i32::try_from(screen_width).ok()?;
    let screen_height_i32 = i32::try_from(screen_height).ok()?;
    let body_heights = common
        .body_data
        .iter()
        .map(dialog_body_declared_height)
        .collect::<Option<Vec<_>>>()?;
    let input_dimensions = common
        .input_data
        .iter()
        .map(|input| dialog_input_dimensions(&input.control))
        .collect::<Option<Vec<_>>>()?;
    let element_count = body_heights.len().saturating_add(input_dimensions.len());
    let content_height = body_heights
        .iter()
        .chain(input_dimensions.iter().map(|(_, height)| height))
        .fold(0i32, |total, height| total.saturating_add(*height))
        .saturating_add(
            i32::try_from(element_count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BODY_SPACING),
        );
    let mut y = (HEADER_HEIGHT + CONTENT_MARGIN_TOP).min(
        screen_height_i32
            .saturating_sub(FOOTER_HEIGHT)
            .saturating_sub(content_height),
    );
    for height in body_heights {
        y = y.saturating_add(height).saturating_add(BODY_SPACING);
    }
    input_dimensions
        .into_iter()
        .enumerate()
        .map(|(index, (width, height))| {
            let layout = DialogInputLayout {
                index,
                x: screen_width_i32 / 2 - width / 2,
                y,
                width,
                height,
            };
            y = y.saturating_add(height).saturating_add(BODY_SPACING);
            Some(layout)
        })
        .collect()
}

fn dialog_single_line_text_box_layout(
    layout: DialogInputLayout,
    label_visible: bool,
) -> DialogInputLayout {
    DialogInputLayout {
        y: layout.y.saturating_add(if label_visible {
            TEXT_LINE_HEIGHT + 4
        } else {
            0
        }),
        height: TEXT_INPUT_HEIGHT,
        ..layout
    }
}

fn approximate_dialog_text_display_start(text: &str, scroll_to: usize, width: u32) -> usize {
    let scroll_to = scroll_to.min(text_edit::char_len(text));
    let prefix_width = text.chars().take(scroll_to).fold(0u32, |total, ch| {
        total.saturating_add(hud_ascii_approx_char_width(ch))
    });
    if prefix_width <= width {
        return 0;
    }
    let chars = text.chars().take(scroll_to).collect::<Vec<_>>();
    let mut start = chars.len();
    let mut used_width = 0u32;
    while start > 0 {
        let advance = hud_ascii_approx_char_width(chars[start - 1]);
        if used_width.saturating_add(advance) > width {
            break;
        }
        used_width = used_width.saturating_add(advance);
        start -= 1;
    }
    start
}

fn approximate_dialog_text_cursor(text: &str, display_start: usize, width: u32) -> usize {
    let display_start = display_start.min(text_edit::char_len(text));
    let mut cursor = display_start;
    let mut used_width = 0u32;
    for ch in text.chars().skip(display_start) {
        let advance = hud_ascii_approx_char_width(ch);
        if used_width.saturating_add(advance) > width {
            break;
        }
        used_width = used_width.saturating_add(advance);
        cursor += 1;
    }
    cursor
}

fn push_slider_track_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    layout: DialogInputLayout,
    highlighted: bool,
) {
    push_horizontal_nine_slice_layers(
        layers,
        layout,
        if highlighted {
            HudInventoryBackgroundTexture::WidgetSliderHighlighted
        } else {
            HudInventoryBackgroundTexture::WidgetSlider
        },
        WIDGET_SPRITE_WIDTH,
        1,
    );
}

fn push_horizontal_nine_slice_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    layout: DialogInputLayout,
    texture: HudInventoryBackgroundTexture,
    sprite_width: u32,
    declared_border: u32,
) {
    let width = u32::try_from(layout.width).unwrap_or(0);
    let height = u32::try_from(layout.height).unwrap_or(0);
    if width == 0 || height == 0 || sprite_width == 0 {
        return;
    }
    // These widget sprites keep their declared 20px target height, so vanilla's nine regions
    // collapse to three exact horizontal columns while retaining the clamped left/right borders.
    let border = declared_border.min(width / 2);
    let columns = [
        (0, border, 0, border),
        (
            border,
            width.saturating_sub(border.saturating_mul(2)),
            border,
            sprite_width.saturating_sub(border.saturating_mul(2)),
        ),
        (
            width.saturating_sub(border),
            border,
            sprite_width.saturating_sub(border),
            border,
        ),
    ];
    for (screen_x, screen_width, texture_x, texture_width) in columns {
        if screen_width == 0 || texture_width == 0 {
            continue;
        }
        layers.push(HudInventoryBackgroundLayer {
            texture,
            x: layout
                .x
                .saturating_add(i32::try_from(screen_x).unwrap_or(i32::MAX)),
            y: layout.y,
            width: screen_width,
            height,
            uv: HudUvRect {
                min: [texture_x as f32 / sprite_width as f32, 0.0],
                max: [
                    texture_x.saturating_add(texture_width) as f32 / sprite_width as f32,
                    1.0,
                ],
            },
        });
    }
}

fn dialog_number_range_scaled_value(range: &DialogNumberRangeState, slider_value: f64) -> f32 {
    let start = range.start();
    let end = range.end();
    let value_in_range = start + slider_value as f32 * (end - start);
    let Some(step) = range.step() else {
        return value_in_range;
    };
    let initial = range.initial().unwrap_or((start + end) / 2.0);
    let steps = ((value_in_range - initial) / step + 0.5).floor() as i32;
    let stepped_delta = steps as f32 * step;
    let result = initial + stepped_delta;
    let slider_position = if start == end {
        0.5
    } else {
        (result - start) / (end - start)
    };
    if slider_position < 0.0 || slider_position > 1.0 {
        let one_step_less = steps - steps.signum();
        initial + one_step_less as f32 * step
    } else {
        result
    }
}

fn dialog_number_range_value_string(value: f32) -> String {
    let integer = value as i32;
    if integer as f32 == value {
        integer.to_string()
    } else {
        java_float_to_string(value)
    }
}

fn dialog_number_range_label_runs(
    template: &str,
    label: &DialogTextState,
    value: &str,
) -> Vec<HudStyledTextRun> {
    dialog_translated_two_arg_runs(
        template,
        &dialog_text_runs(label),
        &[HudStyledTextRun::plain(value)],
    )
}

fn dialog_translated_two_arg_runs(
    template: &str,
    first: &[HudStyledTextRun],
    second: &[HudStyledTextRun],
) -> Vec<HudStyledTextRun> {
    let mut runs = Vec::new();
    let mut current = 0;
    let mut replacement_index = 0usize;
    while let Some(offset) = template[current..].find('%') {
        let percent = current + offset;
        append_plain_dialog_run(&mut runs, &template[current..percent]);
        let bytes = template.as_bytes();
        let mut next = percent + 1;
        if bytes.get(next) == Some(&b'%') {
            append_plain_dialog_run(&mut runs, "%");
            current = next + 1;
            continue;
        }

        let digit_start = next;
        while bytes.get(next).is_some_and(u8::is_ascii_digit) {
            next += 1;
        }
        let explicit_index = if next > digit_start {
            if bytes.get(next) != Some(&b'$') {
                return vec![HudStyledTextRun::plain(template)];
            }
            let Ok(index) = template[digit_start..next].parse::<usize>() else {
                return vec![HudStyledTextRun::plain(template)];
            };
            next += 1;
            let Some(index) = index.checked_sub(1) else {
                return vec![HudStyledTextRun::plain(template)];
            };
            Some(index)
        } else {
            None
        };
        if bytes.get(next) != Some(&b's') {
            return vec![HudStyledTextRun::plain(template)];
        }
        let index = explicit_index.unwrap_or_else(|| {
            let index = replacement_index;
            replacement_index = replacement_index.saturating_add(1);
            index
        });
        match index {
            0 => append_dialog_runs(&mut runs, first),
            1 => append_dialog_runs(&mut runs, second),
            _ => return vec![HudStyledTextRun::plain(template)],
        }
        current = next + 1;
    }
    append_plain_dialog_run(&mut runs, &template[current..]);
    runs
}

fn append_dialog_runs(output: &mut Vec<HudStyledTextRun>, runs: &[HudStyledTextRun]) {
    for run in runs {
        if let Some(previous) = output
            .last_mut()
            .filter(|previous| previous.style == run.style && previous.color == run.color)
        {
            previous.text.push_str(&run.text);
        } else {
            output.push(run.clone());
        }
    }
}

fn append_plain_dialog_run(output: &mut Vec<HudStyledTextRun>, text: &str) {
    if text.is_empty() {
        return;
    }
    let run = HudStyledTextRun::plain(text);
    append_dialog_runs(output, std::slice::from_ref(&run));
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
    world: &WorldStore,
    dialog: &DialogState,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_state: &InventoryHudLocalState,
    surface_size: PhysicalSize<u32>,
    partial_tick: f32,
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
    let cursor_screen_position = local_state.cursor_position;
    let cursor_position = local_state
        .cursor_position
        .map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
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
                dialog_text_block(message).map(DialogBodyBlock::Text)
            }
            DialogBodyState::Item {
                item,
                description,
                show_decorations,
                show_tooltip,
                width,
                height,
            } => dialog_item_block(
                world,
                item_runtime,
                terrain_textures,
                local_state,
                item,
                description.as_ref(),
                *show_decorations,
                *show_tooltip,
                *width,
                *height,
                partial_tick,
            )
            .map(DialogBodyBlock::Item),
        })
        .collect::<Vec<_>>();
    let body_height = body_blocks
        .iter()
        .fold(0i32, |height, block| height.saturating_add(block.height()));
    let input_dimensions = controls
        .common
        .input_data
        .iter()
        .map(|input| dialog_input_dimensions(&input.control))
        .collect::<Option<Vec<_>>>()?;
    let content_element_count = body_blocks.len().saturating_add(input_dimensions.len());
    let content_height = input_dimensions
        .iter()
        .fold(body_height, |height, (_, input_height)| {
            height.saturating_add(*input_height)
        })
        .saturating_add(
            i32::try_from(content_element_count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BODY_SPACING),
        );
    let mut body_y = (HEADER_HEIGHT + CONTENT_MARGIN_TOP).min(
        screen_height_i32
            .saturating_sub(FOOTER_HEIGHT)
            .saturating_sub(content_height),
    );
    let mut floating_items = Vec::new();
    let mut item_tooltip = None;
    for (block_index, block) in body_blocks.into_iter().enumerate() {
        if block_index > 0 {
            body_y = body_y.saturating_add(BODY_SPACING);
        }
        let block_x = screen_width_i32 / 2 - block.width() / 2;
        let block_height = block.height();
        match block {
            DialogBodyBlock::Text(text) => {
                push_dialog_text_block(&mut text_labels, text, block_x, body_y);
            }
            DialogBodyBlock::Item(mut item) => {
                let item_y =
                    body_y.saturating_add(vertical_center_offset(block_height, item.widget_height));
                item.item.x = block_x;
                item.item.y = item_y;
                if item.show_tooltip
                    && dialog_widget_contains(
                        block_x,
                        item_y,
                        item.widget_width,
                        item.widget_height,
                        cursor_position,
                        surface_size,
                        layout.screen_width,
                        layout.screen_height,
                    )
                {
                    item_tooltip = dialog_item_tooltip(
                        world,
                        item_runtime,
                        &item.stack,
                        local_state.advanced_item_tooltips,
                        cursor_screen_position,
                        surface_size,
                        layout.screen_width,
                        layout.screen_height,
                        block_index,
                    );
                }
                floating_items.push(item.item);
                if let Some(description) = item.description {
                    let description_y = body_y
                        .saturating_add(vertical_center_offset(block_height, description.height));
                    push_dialog_text_block(
                        &mut text_labels,
                        description,
                        block_x
                            .saturating_add(item.widget_width)
                            .saturating_add(ITEM_DESCRIPTION_SPACING),
                        description_y,
                    );
                }
            }
        }
        body_y = body_y.saturating_add(block_height);
    }

    let input_layouts = dialog_input_layouts(controls.common, surface_size)?;
    let mut input_layers = Vec::new();
    for input_layout in input_layouts {
        let Some(input) = controls.common.input_data.get(input_layout.index) else {
            continue;
        };
        let hovered = dialog_widget_contains(
            input_layout.x,
            input_layout.y,
            input_layout.width,
            input_layout.height,
            cursor_position,
            surface_size,
            layout.screen_width,
            layout.screen_height,
        );
        let focused = world.dialog_focused_input() == Some(input_layout.index);
        match (
            &input.control,
            world.current_dialog_input_values().get(input_layout.index),
        ) {
            (
                DialogInputControlState::Boolean { label, .. },
                Some(Some(DialogInputValueState::Boolean { selected })),
            ) => {
                input_layers.push(HudInventoryBackgroundLayer {
                    texture: match (*selected, hovered || focused) {
                        (false, false) => HudInventoryBackgroundTexture::WidgetCheckbox,
                        (false, true) => HudInventoryBackgroundTexture::WidgetCheckboxHighlighted,
                        (true, false) => HudInventoryBackgroundTexture::WidgetCheckboxSelected,
                        (true, true) => {
                            HudInventoryBackgroundTexture::WidgetCheckboxSelectedHighlighted
                        }
                    },
                    x: input_layout.x,
                    y: input_layout.y,
                    width: CHECKBOX_SIZE,
                    height: CHECKBOX_SIZE,
                    uv: HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                });
                push_text_label(
                    &mut text_labels,
                    label,
                    input_layout
                        .x
                        .saturating_add(i32::try_from(CHECKBOX_SIZE).unwrap_or(i32::MAX))
                        .saturating_add(CHECKBOX_LABEL_SPACING),
                    input_layout.y.saturating_add(
                        (i32::try_from(CHECKBOX_SIZE).unwrap_or(i32::MAX) - TEXT_LINE_HEIGHT) / 2,
                    ),
                );
            }
            (
                DialogInputControlState::NumberRange {
                    label,
                    label_format,
                    range,
                    ..
                },
                Some(Some(DialogInputValueState::NumberRange { slider_value_bits })),
            ) => {
                let keyboard_can_change =
                    world.dialog_number_range_keyboard_can_change(input_layout.index);
                push_slider_track_layers(
                    &mut input_layers,
                    input_layout,
                    focused && !keyboard_can_change,
                );
                let slider_value = f64::from_bits(*slider_value_bits);
                let handle_offset =
                    (slider_value
                        * f64::from(input_layout.width.saturating_sub(
                            i32::try_from(SLIDER_HANDLE_WIDTH).unwrap_or(i32::MAX),
                        ))) as i32;
                input_layers.push(HudInventoryBackgroundLayer {
                    texture: if hovered
                        || keyboard_can_change
                        || world.dialog_dragging_number_range_input() == Some(input_layout.index)
                    {
                        HudInventoryBackgroundTexture::WidgetSliderHandleHighlighted
                    } else {
                        HudInventoryBackgroundTexture::WidgetSliderHandle
                    },
                    x: input_layout.x.saturating_add(handle_offset),
                    y: input_layout.y,
                    width: SLIDER_HANDLE_WIDTH,
                    height: SLIDER_HEIGHT,
                    uv: HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                });
                let value = dialog_number_range_scaled_value(range, slider_value);
                let value = dialog_number_range_value_string(value);
                let template = match item_runtime {
                    Some(runtime) => runtime.localized_text(label_format),
                    None if label_format == "options.generic_value" => "%s: %s",
                    None => label_format,
                };
                let runs = dialog_number_range_label_runs(template, label, &value);
                let available_width = u32::try_from(input_layout.width)
                    .unwrap_or(1)
                    .saturating_sub(WIDGET_TEXT_MARGIN.saturating_mul(2))
                    .max(1);
                push_centered_runs_text_label(
                    &mut text_labels,
                    runs,
                    input_layout.x.saturating_add(input_layout.width / 2),
                    input_layout.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
                    Some(available_width),
                );
            }
            (
                DialogInputControlState::SingleOption {
                    options,
                    label,
                    label_visible,
                    ..
                },
                Some(Some(DialogInputValueState::SingleOption { selected_index })),
            ) => {
                let Some(selected) = options.get(*selected_index) else {
                    continue;
                };
                push_horizontal_nine_slice_layers(
                    &mut input_layers,
                    input_layout,
                    if hovered || focused {
                        HudInventoryBackgroundTexture::WidgetButtonHighlighted
                    } else {
                        HudInventoryBackgroundTexture::WidgetButton
                    },
                    WIDGET_SPRITE_WIDTH,
                    3,
                );
                let value_runs = selected.display.as_ref().map_or_else(
                    || vec![HudStyledTextRun::plain(&selected.id)],
                    dialog_text_runs,
                );
                let runs = if *label_visible {
                    let template = match item_runtime {
                        Some(runtime) => runtime.localized_text("options.generic_value"),
                        None => "%s: %s",
                    };
                    dialog_translated_two_arg_runs(template, &dialog_text_runs(label), &value_runs)
                } else {
                    value_runs
                };
                let available_width = u32::try_from(input_layout.width)
                    .unwrap_or(1)
                    .saturating_sub(WIDGET_TEXT_MARGIN.saturating_mul(2))
                    .max(1);
                push_centered_runs_text_label(
                    &mut text_labels,
                    runs,
                    input_layout.x.saturating_add(input_layout.width / 2),
                    input_layout.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
                    Some(available_width),
                );
            }
            (
                DialogInputControlState::Text {
                    label,
                    label_visible,
                    max_length,
                    multiline: None,
                    ..
                },
                Some(Some(DialogInputValueState::Text { value })),
            ) => {
                if *label_visible {
                    push_text_label(&mut text_labels, label, input_layout.x, input_layout.y);
                }
                let box_layout = dialog_single_line_text_box_layout(input_layout, *label_visible);
                let edit = world
                    .dialog_text_edit_state()
                    .filter(|edit| edit.input_index == input_layout.index);
                push_horizontal_nine_slice_layers(
                    &mut input_layers,
                    box_layout,
                    if edit.is_some() {
                        HudInventoryBackgroundTexture::WidgetTextFieldHighlighted
                    } else {
                        HudInventoryBackgroundTexture::WidgetTextField
                    },
                    WIDGET_SPRITE_WIDTH,
                    1,
                );
                let inner_width = u32::try_from(
                    box_layout
                        .width
                        .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                        .max(1),
                )
                .unwrap_or(1);
                let text_len = text_edit::char_len(value);
                let cursor = edit.map_or(text_len, |edit| edit.cursor);
                let selection = edit.map_or(text_len, |edit| edit.selection);
                text_labels.push(HudInventoryTextLabel {
                    x: box_layout.x.saturating_add(TEXT_INPUT_INSET),
                    y: box_layout.y.saturating_add(TEXT_INPUT_Y_OFFSET),
                    width: inner_width,
                    text: value.clone(),
                    tint: TEXT_INPUT_COLOR,
                    background: None,
                    input: Some(HudInventoryTextInputDecoration {
                        cursor,
                        selection,
                        scroll_to: if selection != cursor {
                            selection
                        } else {
                            cursor
                        },
                        max_length: usize::try_from(*max_length).unwrap_or(usize::MAX),
                        cursor_visible: edit.is_some() && (wall_clock_millis() / 300) % 2 == 0,
                        cursor_tint: TEXT_INPUT_COLOR,
                        selection_tint: TEXT_SELECTION_TINT,
                    }),
                    shadow: true,
                    runs: Vec::new(),
                });
            }
            _ => {}
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

    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    background_layers.extend(input_layers);
    background_layers.extend(layout.buttons.iter().enumerate().map(|(index, button)| {
        HudInventoryBackgroundLayer {
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
        }
    }));
    let action_tooltip = highlighted_button.and_then(|index| {
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
        floating_items,
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        text_labels,
        hovered_slot_id: None,
        tooltip: action_tooltip.or(item_tooltip),
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
    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
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

#[cfg(test)]
pub(crate) fn handle_dialog_key_input(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
) -> bool {
    handle_dialog_key_input_with_modifiers(
        counters,
        world,
        net_commands,
        clipboard,
        physical_key,
        state,
        false,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_key_input_with_modifiers(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    mut clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
    shift_down: bool,
    control_down: bool,
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
    if world.current_dialog().is_none() {
        return false;
    }
    if state != ElementState::Pressed {
        return true;
    }
    if handle_focused_dialog_text_key(
        world,
        &mut clipboard,
        physical_key,
        shift_down,
        control_down,
    ) {
        return true;
    }
    if handle_focused_dialog_input_key(world, physical_key, shift_down) {
        return true;
    }
    if physical_key != PhysicalKey::Code(KeyCode::Escape) {
        return true;
    }
    let Some(controls) = world
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(simple_dialog_controls)
    else {
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

fn handle_focused_dialog_input_key(
    world: &mut WorldStore,
    physical_key: PhysicalKey,
    shift_down: bool,
) -> bool {
    let PhysicalKey::Code(code) = physical_key else {
        return false;
    };
    let Some(index) = world.dialog_focused_input() else {
        return false;
    };
    let control = world
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(simple_dialog_controls)
        .and_then(|controls| controls.common.input_data.get(index))
        .map(|input| input.control.clone());
    let selection = matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space);
    match control {
        Some(DialogInputControlState::Boolean { .. }) if selection => {
            world.toggle_dialog_boolean_input(index);
            true
        }
        Some(DialogInputControlState::SingleOption { .. }) if selection => {
            cycle_dialog_single_option(world, index, shift_down);
            true
        }
        Some(DialogInputControlState::NumberRange { .. }) if selection => {
            world.toggle_dialog_number_range_keyboard_change(index);
            true
        }
        Some(DialogInputControlState::NumberRange { width, .. })
            if world.dialog_number_range_keyboard_can_change(index)
                && matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight) =>
        {
            let Some(Some(DialogInputValueState::NumberRange { slider_value_bits })) =
                world.current_dialog_input_values().get(index)
            else {
                return true;
            };
            let direction = if code == KeyCode::ArrowLeft {
                -1.0
            } else {
                1.0
            };
            let value =
                f64::from_bits(*slider_value_bits) + direction / f64::from(width.saturating_sub(8));
            world.set_dialog_number_range_slider_value(index, value);
            true
        }
        _ => false,
    }
}

fn handle_focused_dialog_text_key(
    world: &mut WorldStore,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    shift_down: bool,
    control_down: bool,
) -> bool {
    let PhysicalKey::Code(code) = physical_key else {
        return false;
    };
    if world.dialog_text_edit_state().is_none() {
        return false;
    }
    match code {
        KeyCode::Backspace => {
            delete_focused_dialog_text(world, -1, control_down);
            true
        }
        KeyCode::Delete => {
            delete_focused_dialog_text(world, 1, control_down);
            true
        }
        KeyCode::ArrowLeft => {
            move_focused_dialog_text_cursor(world, -1, shift_down, control_down);
            true
        }
        KeyCode::ArrowRight => {
            move_focused_dialog_text_cursor(world, 1, shift_down, control_down);
            true
        }
        KeyCode::Home => {
            move_focused_dialog_text_cursor_to(world, 0, shift_down);
            true
        }
        KeyCode::End => {
            let end = focused_dialog_text_snapshot(world)
                .map(|(_, value, _, _, _)| text_edit::char_len(&value))
                .unwrap_or(0);
            move_focused_dialog_text_cursor_to(world, end, shift_down);
            true
        }
        KeyCode::KeyA if control_down => {
            if let Some((index, value, _, _, _)) = focused_dialog_text_snapshot(world) {
                let end = text_edit::char_len(&value);
                world.set_dialog_text_input_value_and_selection(index, value, end, 0);
            }
            true
        }
        KeyCode::KeyC if control_down => {
            copy_focused_dialog_text_selection(world, clipboard);
            true
        }
        KeyCode::KeyV if control_down => {
            if let Some(text) = clipboard
                .as_deref_mut()
                .and_then(DebugClipboard::get_debug_clipboard_text)
            {
                replace_focused_dialog_text_selection(world, &text);
            }
            true
        }
        KeyCode::KeyX if control_down => {
            copy_focused_dialog_text_selection(world, clipboard);
            replace_focused_dialog_text_selection(world, "");
            true
        }
        _ => false,
    }
}

fn focused_dialog_text_snapshot(
    world: &WorldStore,
) -> Option<(usize, String, usize, usize, usize)> {
    let edit = world.dialog_text_edit_state()?;
    let Some(Some(DialogInputValueState::Text { value })) =
        world.current_dialog_input_values().get(edit.input_index)
    else {
        return None;
    };
    let max_length = world
        .current_dialog()?
        .data
        .as_ref()
        .and_then(simple_dialog_controls)?
        .common
        .input_data
        .get(edit.input_index)
        .and_then(|input| match &input.control {
            DialogInputControlState::Text {
                max_length,
                multiline: None,
                ..
            } => usize::try_from(*max_length).ok(),
            _ => None,
        })?;
    Some((
        edit.input_index,
        value.clone(),
        edit.cursor,
        edit.selection,
        max_length,
    ))
}

fn move_focused_dialog_text_cursor(
    world: &mut WorldStore,
    direction: i32,
    extend_selection: bool,
    whole_word: bool,
) -> bool {
    let Some((index, value, cursor, selection, _)) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let next = if whole_word {
        text_edit::word_position(&value, cursor, direction)
    } else if direction < 0 {
        cursor.saturating_sub(1)
    } else {
        cursor.saturating_add(1).min(text_edit::char_len(&value))
    };
    world.set_dialog_text_input_value_and_selection(
        index,
        value,
        next,
        if extend_selection { selection } else { next },
    )
}

fn move_focused_dialog_text_cursor_to(
    world: &mut WorldStore,
    cursor: usize,
    extend_selection: bool,
) -> bool {
    let Some((index, value, _, selection, _)) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    world.set_dialog_text_input_value_and_selection(
        index,
        value,
        cursor,
        if extend_selection { selection } else { cursor },
    )
}

fn delete_focused_dialog_text(world: &mut WorldStore, direction: i32, whole_word: bool) -> bool {
    let Some((index, mut value, cursor, selection, _)) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let (start, end) = if cursor != selection {
        (cursor.min(selection), cursor.max(selection))
    } else {
        let target = if whole_word {
            text_edit::word_position(&value, cursor, direction)
        } else if direction < 0 {
            cursor.saturating_sub(1)
        } else {
            cursor.saturating_add(1).min(text_edit::char_len(&value))
        };
        (cursor.min(target), cursor.max(target))
    };
    if start != end {
        value.replace_range(
            text_edit::byte_index(&value, start)..text_edit::byte_index(&value, end),
            "",
        );
    }
    world.set_dialog_text_input_value_and_selection(index, value, start, start)
}

fn replace_focused_dialog_text_selection(world: &mut WorldStore, input: &str) -> bool {
    let Some((index, mut value, cursor, selection, max_length)) =
        focused_dialog_text_snapshot(world)
    else {
        return false;
    };
    let start = cursor.min(selection);
    let end = cursor.max(selection);
    let start_byte = text_edit::byte_index(&value, start);
    let end_byte = text_edit::byte_index(&value, end);
    let selected_utf16 = value[start_byte..end_byte].encode_utf16().count();
    let retained_utf16 = value.encode_utf16().count().saturating_sub(selected_utf16);
    let mut remaining = max_length.saturating_sub(retained_utf16);
    let mut replacement = String::new();
    for ch in input
        .chars()
        .filter(|ch| dialog_text_character_allowed(*ch))
    {
        let units = ch.len_utf16();
        if units > remaining {
            break;
        }
        replacement.push(ch);
        remaining -= units;
    }
    let replacement_len = text_edit::char_len(&replacement);
    value.replace_range(start_byte..end_byte, &replacement);
    let cursor = start.saturating_add(replacement_len);
    world.set_dialog_text_input_value_and_selection(index, value, cursor, cursor)
}

fn dialog_text_character_allowed(ch: char) -> bool {
    ch != '\u{00a7}' && ch >= ' ' && ch != '\u{007f}'
}

fn copy_focused_dialog_text_selection(
    world: &WorldStore,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
) -> bool {
    let Some((_, value, cursor, selection, _)) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let start = cursor.min(selection);
    let end = cursor.max(selection);
    if start == end {
        return true;
    }
    let selected = &value[text_edit::byte_index(&value, start)..text_edit::byte_index(&value, end)];
    clipboard
        .as_deref_mut()
        .is_some_and(|clipboard| clipboard.set_debug_clipboard_text(selected))
}

pub(crate) fn handle_dialog_text_input(world: &mut WorldStore, text: &str) -> bool {
    if world.dialog_text_edit_state().is_none() {
        return world.dialog_screen_is_open();
    }
    replace_focused_dialog_text_selection(world, text);
    true
}

#[cfg(test)]
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
    handle_dialog_mouse_input_with_modifiers_and_measurer(
        counters,
        world,
        net_commands,
        clipboard,
        button,
        state,
        cursor_position,
        surface_size,
        false,
        &ApproximateDialogTextMeasurer,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_mouse_input_with_modifiers(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    shift_down: bool,
) -> bool {
    handle_dialog_mouse_input_with_modifiers_and_measurer(
        counters,
        world,
        net_commands,
        clipboard,
        button,
        state,
        cursor_position,
        surface_size,
        shift_down,
        &ApproximateDialogTextMeasurer,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_mouse_input_with_modifiers_and_measurer(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    shift_down: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if button == MouseButton::Left && state == ElementState::Released {
        world.end_dialog_number_range_drag();
        world.end_dialog_text_input_drag();
        return world.dialog_screen_is_open();
    }
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
    if button != MouseButton::Left || state != ElementState::Pressed {
        return world.current_dialog().is_some();
    }
    world.end_dialog_number_range_drag();
    world.end_dialog_text_input_drag();
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    let Some(controls) = dialog.data.as_ref().and_then(simple_dialog_controls) else {
        return true;
    };
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let input_hit = dialog_input_layouts(controls.common, surface_size).and_then(|layouts| {
        layouts.into_iter().find_map(|layout| {
            let control = &controls.common.input_data.get(layout.index)?.control;
            let hit_layout = match control {
                DialogInputControlState::Text {
                    label_visible,
                    multiline: None,
                    ..
                } => dialog_single_line_text_box_layout(layout, *label_visible),
                _ => layout,
            };
            if !dialog_widget_contains(
                hit_layout.x,
                hit_layout.y,
                hit_layout.width,
                hit_layout.height,
                cursor_position,
                surface_size,
                screen_width,
                screen_height,
            ) {
                return None;
            }
            match control {
                DialogInputControlState::Boolean { .. } => {
                    Some(DialogInputMouseHit::Boolean(layout.index))
                }
                DialogInputControlState::NumberRange { .. } => {
                    Some(DialogInputMouseHit::NumberRange(layout))
                }
                DialogInputControlState::SingleOption { .. } => {
                    Some(DialogInputMouseHit::SingleOption(layout))
                }
                DialogInputControlState::Text {
                    multiline: None, ..
                } => Some(DialogInputMouseHit::Text(hit_layout)),
                _ => None,
            }
        })
    });
    let action = SimpleDialogLayout::new(
        &controls
            .actions
            .iter()
            .map(|action| action.width)
            .collect::<Vec<_>>(),
        surface_size,
    )
    .and_then(|layout| layout.button_at(cursor_position, surface_size))
    .and_then(|action_index| {
        Some((
            controls.actions.get(action_index)?.action_data.clone(),
            controls.common.after_action.clone(),
        ))
    });
    match input_hit {
        Some(DialogInputMouseHit::Boolean(index)) => {
            world.focus_dialog_mouse_input(index);
            world.toggle_dialog_boolean_input(index);
            return true;
        }
        Some(DialogInputMouseHit::NumberRange(layout)) => {
            world.focus_dialog_mouse_input(layout.index);
            world.begin_dialog_number_range_drag(layout.index);
            set_dialog_number_range_from_cursor(world, layout, cursor_position, surface_size);
            return true;
        }
        Some(DialogInputMouseHit::SingleOption(layout)) => {
            world.focus_dialog_mouse_input(layout.index);
            cycle_dialog_single_option(world, layout.index, shift_down);
            return true;
        }
        Some(DialogInputMouseHit::Text(layout)) => {
            focus_dialog_text_from_cursor(
                world,
                layout,
                cursor_position,
                surface_size,
                shift_down,
                text_measurer,
            );
            world.begin_dialog_text_input_drag(layout.index);
            return true;
        }
        None if action.is_some() => {
            world.clear_dialog_input_focus();
        }
        None => {}
    }
    let Some(action) = action else {
        return true;
    };
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

fn focus_dialog_text_from_cursor(
    world: &mut WorldStore,
    layout: DialogInputLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    extend_selection: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(cursor_position) = cursor_position else {
        return false;
    };
    let Some(Some(DialogInputValueState::Text { value })) =
        world.current_dialog_input_values().get(layout.index)
    else {
        return false;
    };
    let text_len = text_edit::char_len(value);
    let scroll_to = world
        .dialog_text_edit_state()
        .filter(|edit| edit.input_index == layout.index)
        .map_or(text_len, |edit| {
            if edit.selection != edit.cursor {
                edit.selection
            } else {
                edit.cursor
            }
        });
    let inner_width = u32::try_from(
        layout
            .width
            .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
            .max(1),
    )
    .unwrap_or(1);
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let (origin_x, _) = centered_screen_origin(surface_size, screen_width, screen_height);
    let text_x = origin_x + i64::from(layout.x) + i64::from(TEXT_INPUT_INSET);
    let position_in_text =
        (cursor_position.x.floor() - text_x as f64).clamp(0.0, f64::from(inner_width)) as u32;
    let display_start = text_measurer.display_start(value, scroll_to, inner_width);
    let cursor = text_measurer.cursor_for_width(value, display_start, position_in_text);
    world.focus_dialog_text_input(layout.index, cursor, extend_selection)
}

fn cycle_dialog_single_option(world: &mut WorldStore, index: usize, reverse: bool) -> bool {
    let (selected_index, option_count) = {
        let Some(dialog) = world.current_dialog() else {
            return false;
        };
        let Some(controls) = dialog.data.as_ref().and_then(simple_dialog_controls) else {
            return false;
        };
        let Some(input) = controls.common.input_data.get(index) else {
            return false;
        };
        let DialogInputControlState::SingleOption { options, .. } = &input.control else {
            return false;
        };
        let Some(Some(DialogInputValueState::SingleOption { selected_index })) =
            world.current_dialog_input_values().get(index)
        else {
            return false;
        };
        (*selected_index, options.len())
    };
    if option_count == 0 {
        return false;
    }
    let selected_index = if reverse {
        selected_index
            .checked_sub(1)
            .unwrap_or(option_count.saturating_sub(1))
    } else {
        selected_index.saturating_add(1) % option_count
    };
    world.set_dialog_single_option_index(index, selected_index)
}

pub(crate) fn handle_dialog_mouse_wheel(
    world: &mut WorldStore,
    delta: MouseScrollDelta,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if world.dialog_waiting_ticks().is_some() {
        return true;
    }
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    let Some(controls) = dialog.data.as_ref().and_then(simple_dialog_controls) else {
        return true;
    };
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let hovered_index = dialog_input_layouts(controls.common, surface_size).and_then(|layouts| {
        layouts.into_iter().find_map(|layout| {
            matches!(
                controls
                    .common
                    .input_data
                    .get(layout.index)
                    .map(|input| &input.control),
                Some(DialogInputControlState::SingleOption { .. })
            )
            .then_some(layout)
            .filter(|layout| {
                dialog_widget_contains(
                    layout.x,
                    layout.y,
                    layout.width,
                    layout.height,
                    cursor_position,
                    surface_size,
                    screen_width,
                    screen_height,
                )
            })
            .map(|layout| layout.index)
        })
    });
    let Some(index) = hovered_index else {
        return true;
    };
    let scroll_y = match delta {
        MouseScrollDelta::LineDelta(_, y) => f64::from(y),
        MouseScrollDelta::PixelDelta(position) => position.y,
    };
    if scroll_y > 0.0 {
        cycle_dialog_single_option(world, index, true);
    } else if scroll_y < 0.0 {
        cycle_dialog_single_option(world, index, false);
    }
    true
}

#[cfg(test)]
pub(crate) fn handle_dialog_cursor_moved(
    world: &mut WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    handle_dialog_cursor_moved_with_measurer(
        world,
        cursor_position,
        surface_size,
        &ApproximateDialogTextMeasurer,
    )
}

pub(crate) fn handle_dialog_cursor_moved_with_measurer(
    world: &mut WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if world.dialog_waiting_ticks().is_some() {
        world.end_dialog_number_range_drag();
        world.end_dialog_text_input_drag();
        return true;
    }
    if let Some(index) = world.dialog_dragging_text_input() {
        let layout = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(simple_dialog_controls)
            .and_then(|controls| {
                let layouts = dialog_input_layouts(controls.common, surface_size)?;
                let layout = layouts.into_iter().find(|layout| layout.index == index)?;
                let label_visible = match &controls.common.input_data.get(index)?.control {
                    DialogInputControlState::Text {
                        label_visible,
                        multiline: None,
                        ..
                    } => *label_visible,
                    _ => return None,
                };
                Some(dialog_single_line_text_box_layout(layout, label_visible))
            });
        let Some(layout) = layout else {
            world.end_dialog_text_input_drag();
            return world.current_dialog().is_some();
        };
        focus_dialog_text_from_cursor(
            world,
            layout,
            cursor_position,
            surface_size,
            true,
            text_measurer,
        );
        return true;
    }
    let Some(index) = world.dialog_dragging_number_range_input() else {
        return world.current_dialog().is_some();
    };
    let layout = world
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(simple_dialog_controls)
        .and_then(|controls| dialog_input_layouts(controls.common, surface_size))
        .and_then(|layouts| layouts.into_iter().find(|layout| layout.index == index));
    let Some(layout) = layout else {
        world.end_dialog_number_range_drag();
        return world.current_dialog().is_some();
    };
    set_dialog_number_range_from_cursor(world, layout, cursor_position, surface_size);
    true
}

fn set_dialog_number_range_from_cursor(
    world: &mut WorldStore,
    layout: DialogInputLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let (origin_x, _) = centered_screen_origin(surface_size, screen_width, screen_height);
    let slider_value = (cursor.x
        - (origin_x + i64::from(layout.x) + i64::from(SLIDER_HANDLE_HALF_WIDTH)) as f64)
        / f64::from(
            layout
                .width
                .saturating_sub(i32::try_from(SLIDER_HANDLE_WIDTH).unwrap_or(i32::MAX)),
        );
    world.set_dialog_number_range_slider_value(layout.index, slider_value)
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
        Some(DialogActionState::RunCommand { command }) => {
            queue_chat_command(
                counters,
                world,
                net_commands,
                command.strip_prefix('/').unwrap_or(command),
            );
        }
        Some(DialogActionState::DynamicRunCommand { template }) => {
            let Some(values) = bound_dialog_input_values(world) else {
                return false;
            };
            let command = instantiate_dialog_command_template(template, &values);
            queue_chat_command(
                counters,
                world,
                net_commands,
                command.strip_prefix('/').unwrap_or(&command),
            );
        }
        Some(DialogActionState::DynamicCustom { id, additions }) => {
            let Some(values) = bound_dialog_input_values(world) else {
                return false;
            };
            let payload = dynamic_custom_payload(additions.as_ref(), &values);
            if let Some(tx) = net_commands {
                if tx
                    .try_send(NetCommand::CustomClickAction(CustomClickAction {
                        id: id.clone(),
                        payload: Some(payload),
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
        Some(DialogActionState::SuggestCommand { .. }) => {
            // Screen.defaultHandleClickEvent calls `activeScreen.insertText` for
            // SuggestCommand. DialogScreen does not override Screen.insertText,
            // so the suggestion has no text side effect, but vanilla still
            // activates the screen computed from `after_action` below.
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundDialogInputValue {
    template_substitution: String,
    tag: NbtSummaryValue,
}

fn bound_dialog_input_values(
    world: &WorldStore,
) -> Option<BTreeMap<String, BoundDialogInputValue>> {
    let controls = world
        .current_dialog()?
        .data
        .as_ref()
        .and_then(simple_dialog_controls)?;
    let values = world.current_dialog_input_values();
    let mut bound = BTreeMap::new();
    for (index, input) in controls.common.input_data.iter().enumerate() {
        let value = match (&input.control, values.get(index)?) {
            (
                DialogInputControlState::Boolean {
                    on_true, on_false, ..
                },
                Some(DialogInputValueState::Boolean { selected }),
            ) => BoundDialogInputValue {
                template_substitution: if *selected { on_true } else { on_false }.clone(),
                tag: NbtSummaryValue::Byte(i8::from(*selected)),
            },
            (
                DialogInputControlState::NumberRange { range, .. },
                Some(DialogInputValueState::NumberRange { slider_value_bits }),
            ) => {
                let value =
                    dialog_number_range_scaled_value(range, f64::from_bits(*slider_value_bits));
                BoundDialogInputValue {
                    template_substitution: dialog_number_range_value_string(value),
                    tag: NbtSummaryValue::Float(value.to_bits()),
                }
            }
            (
                DialogInputControlState::SingleOption { options, .. },
                Some(DialogInputValueState::SingleOption { selected_index }),
            ) => {
                let selected = options.get(*selected_index)?;
                BoundDialogInputValue {
                    template_substitution: selected.id.clone(),
                    tag: NbtSummaryValue::String(selected.id.clone()),
                }
            }
            (DialogInputControlState::Text { .. }, Some(DialogInputValueState::Text { value })) => {
                BoundDialogInputValue {
                    template_substitution: escape_dialog_text_template(value),
                    tag: NbtSummaryValue::String(value.clone()),
                }
            }
            _ => return None,
        };
        bound.insert(input.key.clone(), value);
    }
    Some(bound)
}

fn escape_dialog_text_template(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' | '\'' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            '\u{0008}' => escaped.push_str("\\b"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\u{000c}' => escaped.push_str("\\f"),
            '\r' => escaped.push_str("\\r"),
            ch if ch < ' ' => escaped.push_str(&format!("\\x{:02x}", ch as u8)),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn instantiate_dialog_command_template(
    template: &DialogCommandTemplateState,
    values: &BTreeMap<String, BoundDialogInputValue>,
) -> String {
    let mut command = String::new();
    for (index, variable) in template.variables.iter().enumerate() {
        command.push_str(template.segments.get(index).map_or("", String::as_str));
        if let Some(value) = values.get(variable) {
            command.push_str(&value.template_substitution);
        }
    }
    if let Some(trailing) = template.segments.get(template.variables.len()) {
        command.push_str(trailing);
    }
    command
}

fn dynamic_custom_payload(
    additions: Option<&NbtSummaryValue>,
    values: &BTreeMap<String, BoundDialogInputValue>,
) -> NbtSummaryValue {
    let mut fields = BTreeMap::new();
    if let Some(NbtSummaryValue::Compound(entries)) = additions {
        for entry in entries {
            fields.insert(entry.name.clone(), entry.value.clone());
        }
    }
    for (key, value) in values {
        fields.insert(key.clone(), value.tag.clone());
    }
    NbtSummaryValue::Compound(
        fields
            .into_iter()
            .map(|(name, value)| bbb_protocol::NbtSummaryEntry { name, value })
            .collect(),
    )
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

fn dialog_text_block(message: &DialogPlainMessageState) -> Option<DialogTextBlock> {
    let width = u32::try_from(message.width).ok()?;
    let text_width = width
        .saturating_sub(u32::try_from(FOCUSABLE_TEXT_PADDING * 2).unwrap_or_default())
        .max(1);
    let lines = wrapped_text_lines(&message.contents, text_width);
    let height = i32::try_from(lines.len())
        .unwrap_or(i32::MAX)
        .saturating_mul(TEXT_LINE_HEIGHT)
        .saturating_add(FOCUSABLE_TEXT_PADDING * 2);
    Some(DialogTextBlock {
        width,
        height,
        lines,
    })
}

#[allow(clippy::too_many_arguments)]
fn dialog_item_block(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_state: &InventoryHudLocalState,
    item: &DialogItemStackTemplateState,
    description: Option<&DialogPlainMessageState>,
    show_decorations: bool,
    show_tooltip: bool,
    width: i32,
    height: i32,
    partial_tick: f32,
) -> Option<DialogItemBlock> {
    let item_runtime = item_runtime?;
    let stack =
        item.item_stack_summary_for_protocol_id(item_runtime.item_protocol_id(&item.resource_id)?)?;
    let icon = hud_item_icon_for_stack(
        world,
        Some(item_runtime),
        &stack,
        None,
        false,
        false,
        false,
        false,
        local_state.shift_down,
        local_state.keybind_context,
        0,
        partial_tick,
    )?;
    Some(DialogItemBlock {
        item: HudInventoryItem {
            x: 0,
            y: 0,
            scale: 1.0,
            scale_y: 1.0,
            icon,
            scissor: None,
            draw_decorations: show_decorations,
            block_model: block_item_3d_model(&stack, Some(item_runtime), terrain_textures),
        },
        stack,
        widget_width: width,
        widget_height: height,
        description: description.and_then(dialog_text_block),
        show_tooltip,
    })
}

fn push_dialog_text_block(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: DialogTextBlock,
    x: i32,
    y: i32,
) {
    let center_x = x.saturating_add(i32::try_from(text.width).unwrap_or(i32::MAX) / 2);
    let mut line_y = y.saturating_add(FOCUSABLE_TEXT_PADDING);
    for line in text.lines {
        let line_width_i32 = i32::try_from(line.width).unwrap_or(i32::MAX);
        labels.push(HudInventoryTextLabel {
            x: center_x.saturating_sub(line_width_i32 / 2),
            y: line_y,
            width: line.width.max(1),
            text: line.text,
            tint: TEXT_COLOR,
            background: None,
            input: None,
            shadow: false,
            runs: line.runs,
        });
        line_y = line_y.saturating_add(TEXT_LINE_HEIGHT);
    }
}

fn vertical_center_offset(available_height: i32, child_height: i32) -> i32 {
    available_height
        .saturating_sub(child_height)
        .saturating_add(1)
        / 2
}

#[allow(clippy::too_many_arguments)]
fn dialog_widget_contains(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let left = origin_x + i64::from(x);
    let top = origin_y + i64::from(y);
    cursor.x >= left as f64
        && cursor.x < (left + i64::from(width)) as f64
        && cursor.y >= top as f64
        && cursor.y < (top + i64::from(height)) as f64
}

#[allow(clippy::too_many_arguments)]
fn dialog_item_tooltip(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    stack: &ItemStackSummary,
    advanced: bool,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
    block_index: usize,
) -> Option<HudInventoryTooltip> {
    let (cursor_x, cursor_y) = cursor_position?;
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(block_index).unwrap_or(u16::MAX),
        x: i32::try_from(i64::from(cursor_x) - origin_x).ok()?,
        y: i32::try_from(i64::from(cursor_y) - origin_y).ok()?,
        lines: hud_item_tooltip_lines(
            world,
            item_runtime,
            stack,
            advanced,
            world.gameplay().game_type == GameType::Creative.id(),
        )?,
    })
}

fn inworld_menu_background_layer(
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> HudInventoryBackgroundLayer {
    let surface_width = surface_size.width.max(1);
    let surface_height = surface_size.height.max(1);
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    HudInventoryBackgroundLayer {
        texture: HudInventoryBackgroundTexture::InworldMenuBackground,
        x: i32::try_from(-origin_x).unwrap_or(i32::MIN),
        y: i32::try_from(-origin_y).unwrap_or(i32::MIN),
        width: surface_width,
        height: surface_height,
        uv: HudUvRect {
            min: [0.0, 0.0],
            // Screen.extractMenuBackgroundTexture declares a logical 32x32
            // texture size even though vanilla's PNG is 16x16, so each source
            // texel occupies two GUI pixels before the repeat sampler wraps.
            max: [surface_width as f32 / 32.0, surface_height as f32 / 32.0],
        },
    }
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

fn push_centered_runs_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<HudStyledTextRun>,
    center_x: i32,
    y: i32,
    width_limit: Option<u32>,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    let measured_width = runs_approx_width(&runs).max(1);
    let width = width_limit.map_or(measured_width, |limit| measured_width.min(limit.max(1)));
    labels.push(HudInventoryTextLabel {
        x: center_x.saturating_sub(i32::try_from(width).unwrap_or(i32::MAX) / 2),
        y,
        width,
        text,
        tint: TEXT_COLOR,
        background: None,
        input: None,
        shadow: false,
        runs,
    });
}

fn push_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    text: &DialogTextState,
    x: i32,
    y: i32,
) {
    let runs = dialog_text_runs(text);
    let width = runs_approx_width(&runs).max(1);
    labels.push(HudInventoryTextLabel {
        x,
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

        fn get_debug_clipboard_text(&mut self) -> Option<String> {
            self.text.clone()
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
    fn inworld_menu_background_covers_surface_outside_centered_dialog_width() {
        let layer = inworld_menu_background_layer(PhysicalSize::new(800, 600), 512, 512);

        assert_eq!(
            layer.texture,
            HudInventoryBackgroundTexture::InworldMenuBackground
        );
        assert_eq!((layer.x, layer.y), (-144, -44));
        assert_eq!((layer.width, layer.height), (800, 600));
        assert_eq!(layer.uv.min, [0.0, 0.0]);
        assert_eq!(layer.uv.max, [25.0, 18.75]);
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
    fn notice_escape_requires_permission_and_forces_close_after_cancel_action() {
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
        assert!(action_notice.current_dialog().is_none());
        assert_eq!(counters.chat_command_commands_queued, 0);
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
        let terrain_textures = TerrainTextureState::default();
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &terrain_textures,
            &InventoryHudLocalState {
                cursor_position: Some((100, 220)),
                ..Default::default()
            },
            PhysicalSize::new(320, 240),
            0.0,
        )
        .unwrap();

        assert_eq!(screen.background_layers.len(), 3);
        assert_eq!(screen.background_layers[0].x, 0);
        assert_eq!(screen.background_layers[0].width, 320);
        assert_eq!(
            screen.background_layers[0].texture,
            HudInventoryBackgroundTexture::InworldMenuBackground
        );
        assert_eq!(screen.background_layers[0].uv.max, [10.0, 7.5]);
        assert_eq!(screen.background_layers[1].x, 66);
        assert_eq!(screen.background_layers[1].width, 80);
        assert_eq!(
            screen.background_layers[1].texture,
            HudInventoryBackgroundTexture::WidgetButtonHighlighted
        );
        assert_eq!(screen.background_layers[2].x, 154);
        assert_eq!(screen.background_layers[2].width, 100);
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
    fn boolean_input_renders_toggles_and_binds_dynamic_actions() {
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![boolean_input(
                    "enabled", "Enabled", false, "yes", "no",
                )]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Send".to_string())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:dynamic/custom".to_string()),
                            ),
                            entry("id", NbtSummaryValue::String("example:dynamic".to_string())),
                            entry(
                                "additions",
                                NbtSummaryValue::Compound(vec![
                                    entry("enabled", NbtSummaryValue::String("old".to_string())),
                                    entry("fixed", NbtSummaryValue::Int(7)),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let input_layout = {
            let controls = world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(simple_dialog_controls)
                .unwrap();
            dialog_input_layouts(controls.common, surface_size).unwrap()[0]
        };
        let input_cursor = (
            input_layout.x.saturating_add(1),
            input_layout.y.saturating_add(1),
        );
        let terrain_textures = TerrainTextureState::default();
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &terrain_textures,
            &InventoryHudLocalState {
                cursor_position: Some(input_cursor),
                ..Default::default()
            },
            surface_size,
            0.0,
        )
        .unwrap();
        assert_eq!(screen.text_labels[1].text, "Enabled");
        assert!(screen.background_layers.iter().any(|layer| {
            layer.texture == HudInventoryBackgroundTexture::WidgetCheckboxHighlighted
                && (layer.x, layer.y) == (input_layout.x, input_layout.y)
        }));

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
            Some(PhysicalPosition::new(
                f64::from(input_cursor.0),
                f64::from(input_cursor.1),
            )),
            surface_size,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Boolean { selected: true })]
        );
        assert_eq!(world.dialog_focused_input(), Some(0));
        let focused_screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &terrain_textures,
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert!(focused_screen.background_layers.iter().any(|layer| {
            layer.texture == HudInventoryBackgroundTexture::WidgetCheckboxSelectedHighlighted
                && (layer.x, layer.y) == (input_layout.x, input_layout.y)
        }));
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Space),
            ElementState::Pressed,
            false,
            false,
        ));
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Boolean { selected: true })]
        );
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(0.0, 0.0)),
            surface_size,
        ));
        assert_eq!(world.dialog_focused_input(), Some(0));

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            surface_size,
        ));
        assert!(world.current_dialog().is_some());
        assert_eq!(world.dialog_focused_input(), None);
        assert_eq!(counters.custom_click_action_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::CustomClickAction(CustomClickAction {
                id,
                payload: Some(NbtSummaryValue::Compound(entries)),
            }) if id == "example:dynamic"
                && entries == vec![
                    entry("enabled", NbtSummaryValue::Byte(1)),
                    entry("fixed", NbtSummaryValue::Int(7)),
                ]
        ));

        let mut command_world = world_with_notice(vec![
            entry(
                "inputs",
                NbtSummaryValue::List(vec![boolean_input("enabled", "Enabled", true, "yes", "no")]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Run".to_string())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String(
                                    "minecraft:dynamic/run_command".to_string(),
                                ),
                            ),
                            entry(
                                "template",
                                NbtSummaryValue::String("/say $(enabled)-$(missing)".to_string()),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut command_world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            surface_size,
        ));
        assert!(command_world.current_dialog().is_none());
        assert_eq!(counters.chat_command_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(command) if command.command == "say yes-"
        ));
    }

    #[test]
    fn number_range_renders_drags_and_binds_float_actions() {
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![number_range_input(
                    "speed",
                    "Speed",
                    100,
                    0.0,
                    10.0,
                    Some(2.0),
                    Some(2.0),
                )]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Send".to_string())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:dynamic/custom".to_string()),
                            ),
                            entry("id", NbtSummaryValue::String("example:range".to_string())),
                        ]),
                    ),
                ]),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let input_layout = {
            let dialog = world.current_dialog().unwrap();
            let controls = dialog
                .data
                .as_ref()
                .and_then(simple_dialog_controls)
                .unwrap_or_else(|| panic!("invalid range dialog: {:?}", dialog.semantic_error));
            dialog_input_layouts(controls.common, surface_size).unwrap()[0]
        };
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert!(screen
            .text_labels
            .iter()
            .any(|label| label.text == "Speed: 2"));
        assert_eq!(
            screen
                .background_layers
                .iter()
                .filter(|layer| layer.texture == HudInventoryBackgroundTexture::WidgetSlider)
                .count(),
            3
        );
        assert!(screen.background_layers.iter().any(|layer| {
            layer.texture == HudInventoryBackgroundTexture::WidgetSliderHandle
                && layer.x == input_layout.x + 18
        }));

        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(2);
        let net_commands = Some(tx);
        let pressed_x = input_layout.x + SLIDER_HANDLE_HALF_WIDTH + 69;
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(
                f64::from(pressed_x),
                f64::from(input_layout.y + 1),
            )),
            surface_size,
        ));
        assert_eq!(world.dialog_dragging_number_range_input(), Some(0));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::NumberRange {
                slider_value_bits: 0.75f64.to_bits(),
            })]
        );

        assert!(handle_dialog_cursor_moved(
            &mut world,
            Some(PhysicalPosition::new(
                f64::from(input_layout.x + SLIDER_HANDLE_HALF_WIDTH + 92),
                f64::from(input_layout.y + 1),
            )),
            surface_size,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::NumberRange {
                slider_value_bits: 1.0f64.to_bits(),
            })]
        );
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Released,
            None,
            surface_size,
        ));
        assert_eq!(world.dialog_dragging_number_range_input(), None);
        assert_eq!(world.dialog_focused_input(), Some(0));
        assert!(world.dialog_number_range_keyboard_can_change(0));
        let focused_screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert_eq!(
            focused_screen
                .background_layers
                .iter()
                .filter(|layer| {
                    layer.texture == HudInventoryBackgroundTexture::WidgetSliderHandleHighlighted
                })
                .count(),
            1
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::ArrowLeft),
            ElementState::Pressed,
            false,
            false,
        ));
        let keyboard_value = 1.0f64 - 1.0 / 92.0;
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::NumberRange {
                slider_value_bits: keyboard_value.to_bits(),
            })]
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Space),
            ElementState::Pressed,
            false,
            false,
        ));
        assert!(!world.dialog_number_range_keyboard_can_change(0));
        let locked_screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert_eq!(
            locked_screen
                .background_layers
                .iter()
                .filter(|layer| {
                    layer.texture == HudInventoryBackgroundTexture::WidgetSliderHighlighted
                })
                .count(),
            3
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::ArrowLeft),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::NumberRange {
                slider_value_bits: keyboard_value.to_bits(),
            })]
        );

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            surface_size,
        ));
        assert_eq!(counters.custom_click_action_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::CustomClickAction(CustomClickAction {
                id,
                payload: Some(NbtSummaryValue::Compound(entries)),
            }) if id == "example:range"
                && entries == vec![entry("speed", NbtSummaryValue::Float(10.0f32.to_bits()))]
        ));

        world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Range".to_string())),
                    entry(
                        "inputs",
                        NbtSummaryValue::List(vec![number_range_input(
                            "speed",
                            "Speed",
                            100,
                            0.0,
                            10.0,
                            Some(2.0),
                            Some(2.0),
                        )]),
                    ),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("label", NbtSummaryValue::String("Run".to_string())),
                            entry(
                                "action",
                                NbtSummaryValue::Compound(vec![
                                    entry(
                                        "type",
                                        NbtSummaryValue::String(
                                            "minecraft:dynamic/run_command".to_string(),
                                        ),
                                    ),
                                    entry(
                                        "template",
                                        NbtSummaryValue::String("/say $(speed)".to_string()),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            },
        });
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            surface_size,
        ));
        assert_eq!(counters.chat_command_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(command) if command.command == "say 2"
        ));
    }

    #[test]
    fn number_range_step_rounding_and_boundary_correction_match_vanilla() {
        let range = DialogNumberRangeState {
            start_bits: 10.0f32.to_bits(),
            end_bits: 0.0f32.to_bits(),
            initial_bits: Some(7.5f32.to_bits()),
            step_bits: Some(2.0f32.to_bits()),
        };

        let rounded_negative = dialog_number_range_scaled_value(&range, 0.6);
        assert_eq!(rounded_negative.to_bits(), 3.5f32.to_bits());
        assert_eq!(dialog_number_range_value_string(rounded_negative), "3.5");
        assert_eq!(
            dialog_number_range_scaled_value(&range, 1.0).to_bits(),
            1.5f32.to_bits()
        );

        let label = DialogTextState {
            text: "Speed".to_string(),
            runs: Vec::new(),
            component: NbtSummaryValue::String("Speed".to_string()),
        };
        let runs = dialog_number_range_label_runs("%2$s / %1$s %%", &label, "3.5");
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "3.5 / Speed %"
        );
        assert_eq!(
            dialog_number_range_label_runs("bad %d", &label, "3.5"),
            vec![HudStyledTextRun::plain("bad %d")]
        );
    }

    #[test]
    fn single_option_renders_cycles_and_binds_string_actions() {
        let options = vec![
            NbtSummaryValue::String("one".to_string()),
            NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::String("two".to_string())),
                entry("display", NbtSummaryValue::String("Second".to_string())),
                entry("initial", NbtSummaryValue::Byte(1)),
            ]),
            NbtSummaryValue::String("three".to_string()),
        ];
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![single_option_input(
                    "mode",
                    "Mode",
                    100,
                    true,
                    options.clone(),
                )]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Send".to_string())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:dynamic/custom".to_string()),
                            ),
                            entry("id", NbtSummaryValue::String("example:option".to_string())),
                        ]),
                    ),
                ]),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let input_layout = {
            let controls = world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(simple_dialog_controls)
                .unwrap();
            dialog_input_layouts(controls.common, surface_size).unwrap()[0]
        };
        let input_cursor = (input_layout.x + 1, input_layout.y + 1);
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState {
                cursor_position: Some(input_cursor),
                ..Default::default()
            },
            surface_size,
            0.0,
        )
        .unwrap();
        assert!(screen
            .text_labels
            .iter()
            .any(|label| label.text == "Mode: Second"));
        assert_eq!(
            screen
                .background_layers
                .iter()
                .filter(|layer| {
                    layer.texture == HudInventoryBackgroundTexture::WidgetButtonHighlighted
                        && layer.y == input_layout.y
                })
                .count(),
            3
        );

        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(2);
        let net_commands = Some(tx);
        let cursor = Some(PhysicalPosition::new(
            f64::from(input_cursor.0),
            f64::from(input_cursor.1),
        ));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            cursor,
            surface_size,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 2,
            })]
        );
        assert!(handle_dialog_mouse_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            cursor,
            surface_size,
            true,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 1,
            })]
        );
        assert_eq!(world.dialog_focused_input(), Some(0));
        let focused_screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert_eq!(
            focused_screen
                .background_layers
                .iter()
                .filter(|layer| {
                    layer.texture == HudInventoryBackgroundTexture::WidgetButtonHighlighted
                        && layer.y == input_layout.y
                })
                .count(),
            3
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Space),
            ElementState::Pressed,
            false,
            false,
        ));
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::NumpadEnter),
            ElementState::Pressed,
            true,
            false,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 1,
            })]
        );
        assert!(handle_dialog_mouse_wheel(
            &mut world,
            MouseScrollDelta::LineDelta(0.0, -1.0),
            cursor,
            surface_size,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 2,
            })]
        );
        assert!(handle_dialog_mouse_wheel(
            &mut world,
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 1.0)),
            cursor,
            surface_size,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 1,
            })]
        );

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            surface_size,
        ));
        assert_eq!(counters.custom_click_action_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::CustomClickAction(CustomClickAction {
                id,
                payload: Some(NbtSummaryValue::Compound(entries)),
            }) if id == "example:option"
                && entries == vec![entry(
                    "mode",
                    NbtSummaryValue::String("two".to_string()),
                )]
        ));

        world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:notice".to_string()),
                    ),
                    entry("title", NbtSummaryValue::String("Options".to_string())),
                    entry(
                        "inputs",
                        NbtSummaryValue::List(vec![single_option_input(
                            "mode",
                            "Mode",
                            100,
                            false,
                            vec![NbtSummaryValue::Compound(vec![
                                entry("id", NbtSummaryValue::String("alpha".to_string())),
                                entry(
                                    "display",
                                    NbtSummaryValue::String("Alpha Display".to_string()),
                                ),
                            ])],
                        )]),
                    ),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry("label", NbtSummaryValue::String("Run".to_string())),
                            entry(
                                "action",
                                NbtSummaryValue::Compound(vec![
                                    entry(
                                        "type",
                                        NbtSummaryValue::String(
                                            "minecraft:dynamic/run_command".to_string(),
                                        ),
                                    ),
                                    entry(
                                        "template",
                                        NbtSummaryValue::String("/say $(mode)".to_string()),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            },
        });
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert!(screen
            .text_labels
            .iter()
            .any(|label| label.text == "Alpha Display"));
        assert!(!screen
            .text_labels
            .iter()
            .any(|label| label.text.contains("Mode:")));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            surface_size,
        ));
        assert_eq!(counters.chat_command_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(command) if command.command == "say alpha"
        ));
    }

    #[test]
    fn single_line_text_renders_edits_and_binds_escaped_and_raw_strings() {
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![text_input("name", "Name", 100, true, "ab😀", 5)]),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let input_layout = {
            let controls = world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(simple_dialog_controls)
                .unwrap();
            dialog_input_layouts(controls.common, surface_size).unwrap()[0]
        };
        let box_layout = dialog_single_line_text_box_layout(input_layout, true);
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert!(screen.text_labels.iter().any(|label| {
            label.text == "Name" && (label.x, label.y) == (input_layout.x, input_layout.y)
        }));
        let text = screen
            .text_labels
            .iter()
            .find(|label| label.text == "ab😀")
            .unwrap();
        assert_eq!(
            (text.x, text.y, text.width),
            (box_layout.x + 4, box_layout.y + 6, 92)
        );
        assert_eq!(text.input.as_ref().unwrap().cursor, 3);
        assert!(!text.input.as_ref().unwrap().cursor_visible);
        assert_eq!(text.tint, TEXT_INPUT_COLOR);
        assert!(text.shadow);
        assert_eq!(
            screen
                .background_layers
                .iter()
                .filter(|layer| {
                    layer.texture == HudInventoryBackgroundTexture::WidgetTextField
                        && layer.y == box_layout.y
                })
                .count(),
            3
        );

        let mut counters = NetCounters::default();
        let cursor = Some(PhysicalPosition::new(
            f64::from(box_layout.x + TEXT_INPUT_INSET + 6),
            f64::from(box_layout.y + 1),
        ));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            cursor,
            surface_size,
        ));
        assert_eq!(
            world.dialog_text_edit_state(),
            Some(bbb_world::DialogTextEditState {
                input_index: 0,
                cursor: 1,
                selection: 1,
            })
        );
        assert!(handle_dialog_cursor_moved(
            &mut world,
            Some(PhysicalPosition::new(
                f64::from(box_layout.x + TEXT_INPUT_INSET + 12),
                f64::from(box_layout.y + 1),
            )),
            surface_size,
        ));
        assert_eq!(
            world.dialog_text_edit_state(),
            Some(bbb_world::DialogTextEditState {
                input_index: 0,
                cursor: 2,
                selection: 1,
            })
        );
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Released,
            cursor,
            surface_size,
        ));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            cursor,
            surface_size,
        ));
        assert!(handle_dialog_text_input(&mut world, "§C😀\n"));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "aCb😀".to_string(),
            })]
        );

        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::ArrowRight),
            ElementState::Pressed,
            true,
            false,
        ));
        let mut clipboard = MockDialogClipboard::default();
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            Some(&mut clipboard),
            PhysicalKey::Code(KeyCode::KeyX),
            ElementState::Pressed,
            false,
            true,
        ));
        assert_eq!(clipboard.text.as_deref(), Some("b"));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "aC😀".to_string(),
            })]
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            Some(&mut clipboard),
            PhysicalKey::Code(KeyCode::KeyV),
            ElementState::Pressed,
            false,
            true,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "aCb😀".to_string(),
            })]
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::KeyA),
            ElementState::Pressed,
            false,
            true,
        ));
        assert!(handle_dialog_text_input(&mut world, "😀😀😀"));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "😀😀".to_string(),
            })]
        );
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState::default(),
            surface_size,
            0.0,
        )
        .unwrap();
        assert_eq!(
            screen
                .background_layers
                .iter()
                .filter(|layer| {
                    layer.texture == HudInventoryBackgroundTexture::WidgetTextFieldHighlighted
                        && layer.y == box_layout.y
                })
                .count(),
            3
        );
        let edited = screen
            .text_labels
            .iter()
            .find(|label| label.text == "😀😀")
            .unwrap();
        assert_eq!(edited.input.as_ref().unwrap().cursor, 2);

        let mut word_world = world_with_notice(vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input("name", "Name", 100, false, "one two", 32)]),
        )]);
        assert!(word_world.focus_dialog_text_input(0, 7, false));
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut word_world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::ArrowLeft),
            ElementState::Pressed,
            false,
            true,
        ));
        assert_eq!(word_world.dialog_text_edit_state().unwrap().cursor, 4);
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut word_world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Home),
            ElementState::Pressed,
            true,
            false,
        ));
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut word_world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Backspace),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(
            word_world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "two".to_string(),
            })]
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut word_world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::End),
            ElementState::Pressed,
            false,
            false,
        ));
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut word_world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Backspace),
            ElementState::Pressed,
            false,
            true,
        ));
        assert_eq!(
            word_world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: String::new(),
            })]
        );

        let escaped_world = world_with_notice(vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input(
                "name",
                "Name",
                100,
                false,
                "a\"'\\\n\u{0001}",
                32,
            )]),
        )]);
        let values = bound_dialog_input_values(&escaped_world).unwrap();
        assert_eq!(values["name"].template_substitution, "a\\\"\\'\\\\\\n\\x01");
        assert_eq!(
            instantiate_dialog_command_template(
                &DialogCommandTemplateState {
                    raw: "/say $(name)".to_string(),
                    segments: vec!["/say ".to_string(), String::new()],
                    variables: vec!["name".to_string()],
                },
                &values,
            ),
            "/say a\\\"\\'\\\\\\n\\x01"
        );
        assert_eq!(
            values["name"].tag,
            NbtSummaryValue::String("a\"'\\\n\u{0001}".to_string())
        );
    }

    #[test]
    fn static_run_command_action_trims_optional_slash_before_send() {
        let mut world = world_with_notice(vec![entry(
            "action",
            NbtSummaryValue::Compound(vec![
                entry("label", NbtSummaryValue::String("Run".to_string())),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "type",
                            NbtSummaryValue::String("minecraft:run_command".to_string()),
                        ),
                        entry(
                            "command",
                            NbtSummaryValue::String("/say static".to_string()),
                        ),
                    ]),
                ),
            ]),
        )]);
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
        assert_eq!(counters.chat_command_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(command) if command.command == "say static"
        ));
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
            entry("pause", NbtSummaryValue::Byte(0)),
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
    fn suggest_command_dialog_action_only_applies_after_action() {
        let mut world = world_with_notice(vec![
            entry(
                "after_action",
                NbtSummaryValue::String("wait_for_response".to_string()),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Suggest".to_string())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:suggest_command".to_string()),
                            ),
                            entry(
                                "command",
                                NbtSummaryValue::String("/say unchanged".to_string()),
                            ),
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
        assert_eq!(counters.chat_command_commands_queued, 0);
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

    fn boolean_input(
        key: &str,
        label: &str,
        initial: bool,
        on_true: &str,
        on_false: &str,
    ) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("key", NbtSummaryValue::String(key.to_string())),
            entry(
                "type",
                NbtSummaryValue::String("minecraft:boolean".to_string()),
            ),
            entry("label", NbtSummaryValue::String(label.to_string())),
            entry("initial", NbtSummaryValue::Byte(i8::from(initial))),
            entry("on_true", NbtSummaryValue::String(on_true.to_string())),
            entry("on_false", NbtSummaryValue::String(on_false.to_string())),
        ])
    }

    #[allow(clippy::too_many_arguments)]
    fn number_range_input(
        key: &str,
        label: &str,
        width: i32,
        start: f32,
        end: f32,
        initial: Option<f32>,
        step: Option<f32>,
    ) -> NbtSummaryValue {
        let mut fields = vec![
            entry("key", NbtSummaryValue::String(key.to_string())),
            entry(
                "type",
                NbtSummaryValue::String("minecraft:number_range".to_string()),
            ),
            entry("width", NbtSummaryValue::Int(width)),
            entry("label", NbtSummaryValue::String(label.to_string())),
            entry("start", NbtSummaryValue::Float(start.to_bits())),
            entry("end", NbtSummaryValue::Float(end.to_bits())),
        ];
        if let Some(initial) = initial {
            fields.push(entry("initial", NbtSummaryValue::Float(initial.to_bits())));
        }
        if let Some(step) = step {
            fields.push(entry("step", NbtSummaryValue::Float(step.to_bits())));
        }
        NbtSummaryValue::Compound(fields)
    }

    fn single_option_input(
        key: &str,
        label: &str,
        width: i32,
        label_visible: bool,
        options: Vec<NbtSummaryValue>,
    ) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("key", NbtSummaryValue::String(key.to_string())),
            entry(
                "type",
                NbtSummaryValue::String("minecraft:single_option".to_string()),
            ),
            entry("width", NbtSummaryValue::Int(width)),
            entry("options", NbtSummaryValue::List(options)),
            entry("label", NbtSummaryValue::String(label.to_string())),
            entry(
                "label_visible",
                NbtSummaryValue::Byte(i8::from(label_visible)),
            ),
        ])
    }

    fn text_input(
        key: &str,
        label: &str,
        width: i32,
        label_visible: bool,
        initial: &str,
        max_length: i32,
    ) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("key", NbtSummaryValue::String(key.to_string())),
            entry(
                "type",
                NbtSummaryValue::String("minecraft:text".to_string()),
            ),
            entry("width", NbtSummaryValue::Int(width)),
            entry("label", NbtSummaryValue::String(label.to_string())),
            entry(
                "label_visible",
                NbtSummaryValue::Byte(i8::from(label_visible)),
            ),
            entry("initial", NbtSummaryValue::String(initial.to_string())),
            entry("max_length", NbtSummaryValue::Int(max_length)),
        ])
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
        assert_eq!(hidden.background_layers.len(), 1);
        assert_eq!(
            hidden.background_layers[0].texture,
            HudInventoryBackgroundTexture::InworldMenuBackground
        );
        assert_eq!(hidden.text_labels.len(), 1);

        let first_countdown = hud_waiting_response_screen(21, None, PhysicalSize::new(320, 240));
        assert_eq!(first_countdown.text_labels[1].text, "Back (4s)");
        assert_eq!(
            first_countdown.background_layers[1].texture,
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
            active.background_layers[1].texture,
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
