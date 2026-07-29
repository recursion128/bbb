use std::collections::BTreeMap;

use bbb_control::NetCounters;
use bbb_item_model::NativeItemRuntime;
use bbb_net::NetCommand;
use bbb_protocol::{
    java_float_to_string,
    packets::{CustomClickAction, GameType, ItemStackSummary},
    ComponentClickEvent, NbtSummaryValue, StyledTextRun,
};
use bbb_renderer::{
    HudComponentTextRun, HudInventoryBackgroundLayer, HudInventoryBackgroundTexture,
    HudInventoryFillLayer, HudInventoryFillStage, HudInventoryItem, HudInventoryItemScissor,
    HudInventoryPreeditOverlay, HudInventoryScreen, HudInventoryScrollArea, HudInventoryScrollbar,
    HudInventoryScrollingText, HudInventoryTextArea, HudInventoryTextInputDecoration,
    HudInventoryTextLabel, HudInventoryTooltip, HudInventoryTooltipLine, HudStyledTextRun,
    HudTextStyle, HudUvRect, Renderer,
};
use bbb_world::{
    CommonDialogDataState, DialogActionState, DialogBodyFocusPartState, DialogBodyState,
    DialogButtonState, DialogCommandTemplateState, DialogDataState, DialogFocusState,
    DialogInputControlState, DialogInputValueState, DialogItemStackTemplateState,
    DialogLinkConfirmationState, DialogNumberRangeState, DialogPlainMessageState, DialogState,
    DialogTextMultilineState, DialogTextState, ServerLinkState, WorldStore,
};
use tokio::sync::mpsc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::input::{queue_chat_command, text_edit, DebugClipboard, DialogImePreeditHudState};
use crate::terrain_runtime::TerrainTextureState;

use super::{
    block_item_3d_model, hud_ascii_approx_char_width, hud_item_icon_for_stack, hud_item_tooltip,
    wall_clock_millis, InventoryHudLocalState,
};

// Vanilla `HeaderAndFooterLayout` uses 33-pixel header/footer frames and places
// content 30 pixels below the header. `DialogScreen` stacks body elements at a
// 10-pixel gap; vanilla font lines are 9 pixels high.
const HEADER_HEIGHT: i32 = 33;
const FOOTER_HEIGHT: i32 = 33;
const WARNING_BUTTON_SIZE: i32 = 20;
const WARNING_BUTTON_SPACING: i32 = 10;
const BODY_SCROLLBAR_RESERVE: i32 = 10;
const CONFIRM_BUTTON_WIDTH: u32 = 150;
const CONFIRM_LINK_BUTTON_WIDTH: u32 = 100;
const CONFIRM_BUTTON_SPACING: i32 = 4;
const CONFIRM_ELEMENT_SPACING: i32 = 8;
const CONFIRM_BUTTON_PADDING_TOP: i32 = 16;
const CONFIRM_MESSAGE_HORIZONTAL_PADDING: u32 = 50;
const CONFIRM_MESSAGE_MAX_ROWS: usize = 15;
const CONTENT_MARGIN_TOP: i32 = 30;
const BODY_SPACING: i32 = 10;
const ITEM_DESCRIPTION_SPACING: i32 = 2;
const FOCUSABLE_TEXT_PADDING: i32 = 4;
const SIMPLE_BUTTON_SPACING: i32 = 8;
const BUTTON_LIST_SPACING: i32 = 2;
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
const PREEDIT_INPUT_HEIGHT: i32 = 10;
const PREEDIT_TEXT_HEIGHT: i32 = 9;
const PREEDIT_INPUT_SEPARATION: i32 = 4;
const PREEDIT_BACKGROUND_OFFSET: i32 = 5;
const PREEDIT_HOT_AREA_MARGIN: i32 = 2;
const TEXT_INPUT_COLOR: [f32; 4] = [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0];
const TEXT_SELECTION_TINT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const INACTIVE_TEXT_COLOR: [f32; 4] = [160.0 / 255.0, 160.0 / 255.0, 160.0 / 255.0, 1.0];
const WAITING_BUTTON_WIDTH: u32 = 200;
const WAITING_BUTTON_VISIBLE_TICKS: u32 = 21;
const WAITING_BUTTON_ACTIVE_TICKS: u32 = 101;
const WAITING_TITLE: &str = "Waiting for Server";
const DIALOG_WARNING_TITLE: &str = "Note about custom screens";
const DIALOG_WARNING_CONTENTS: &str = concat!(
    "The contents of this screen are controlled by third-party servers and maps that are not ",
    "owned, operated, or supervised by Mojang Studios or Microsoft.\n\n",
    "Handle with care! Always be careful when following links and never give away your personal ",
    "information, including login details.\n\n",
    "If this screen prevents you from playing, you can also disconnect from the current server ",
    "by using the button below."
);
const DIALOG_WARNING_TOOLTIP: &str = "This is a custom screen. Click here to learn more.";
const DIALOG_WARNING_DISCONNECT: &str = "Disconnect";
const DIALOG_WARNING_BACK: &str = "Back";
const CONFIRM_LINK_TITLE: &str = "Are you sure you want to open the following website?";
const CONFIRM_LINK_WARNING: &str = "Never open links from people that you don't trust!";
const CONFIRM_LINK_YES: &str = "Yes";
const CONFIRM_LINK_COPY: &str = "Copy to Clipboard";
const CONFIRM_LINK_NO: &str = "No";
const CONFIRM_LINK_WARNING_COLOR: [f32; 4] = [1.0, 204.0 / 255.0, 204.0 / 255.0, 1.0];

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogTextLine {
    text: String,
    width: u32,
    runs: Vec<HudStyledTextRun>,
    click_spans: Vec<DialogTextClickSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogTextClickSpan {
    x: u32,
    width: u32,
    event: ComponentClickEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogButtonLayout {
    x: i32,
    y: i32,
    width: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogWarningButtonLayout {
    title_x: i32,
    title_y: i32,
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogWarningScreenLayout {
    screen_width: u32,
    screen_height: u32,
    title_y: i32,
    message_y: i32,
    yes: DialogButtonLayout,
    no: DialogButtonLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogConfirmLinkLayout {
    screen_width: u32,
    screen_height: u32,
    title_y: i32,
    message_y: i32,
    warning_y: i32,
    buttons: [DialogButtonLayout; 3],
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
struct DialogFocusRectangle {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogFocusNode {
    target: DialogFocusState,
    rectangle: DialogFocusRectangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogBodyFocusContainer {
    rectangle: DialogFocusRectangle,
    content_rectangle: DialogFocusRectangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogBodyScrollLayout {
    rectangle: DialogFocusRectangle,
    content_height: i32,
    max_scroll: i32,
    scroll_amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogScrollbarLayout {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    thumb_y: i32,
    thumb_height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogTopFocusNode {
    Body(DialogBodyFocusContainer),
    Target(DialogFocusNode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogFocusGraph {
    screen: DialogFocusRectangle,
    body: Vec<DialogFocusNode>,
    top: Vec<DialogTopFocusNode>,
    tab: Vec<DialogFocusState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogFocusDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogInputMouseHit {
    Boolean(usize),
    NumberRange(DialogInputLayout),
    SingleOption(DialogInputLayout),
    Text {
        layout: DialogInputLayout,
        multiline: bool,
    },
}

pub(crate) trait DialogTextMeasurer {
    fn width(&self, text: &str) -> u32;

    fn display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize;

    fn cursor_for_width(&self, text: &str, display_start: usize, width: u32) -> usize;
}

impl DialogTextMeasurer for Renderer {
    fn width(&self, text: &str) -> u32 {
        self.hud_plain_text_width(text)
    }

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
pub(super) struct ApproximateDialogTextMeasurer;

#[cfg(test)]
impl DialogTextMeasurer for ApproximateDialogTextMeasurer {
    fn width(&self, text: &str) -> u32 {
        text.chars().fold(0, |width, ch| {
            width.saturating_add(hud_ascii_approx_char_width(ch))
        })
    }

    fn display_start(&self, text: &str, scroll_to: usize, width: u32) -> usize {
        approximate_dialog_text_display_start(text, scroll_to, width)
    }

    fn cursor_for_width(&self, text: &str, display_start: usize, width: u32) -> usize {
        approximate_dialog_text_cursor(text, display_start, width)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogFooterLayout {
    screen_width: u32,
    screen_height: u32,
    buttons: Vec<DialogButtonLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogBodyActionGridLayout {
    width: i32,
    height: i32,
    buttons: Vec<DialogButtonLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DialogContentLayout {
    body_dimensions: Vec<(i32, i32)>,
    input_dimensions: Vec<(i32, i32)>,
    action_grid: Option<DialogBodyActionGridLayout>,
    width: i32,
    height: i32,
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
                dialog_item_dimensions(
                    item.widget_width,
                    item.widget_height,
                    item.description.as_ref(),
                )
                .0
            }
        }
    }

    fn height(&self) -> i32 {
        match self {
            Self::Text(text) => text.height,
            Self::Item(item) => {
                dialog_item_dimensions(
                    item.widget_width,
                    item.widget_height,
                    item.description.as_ref(),
                )
                .1
            }
        }
    }
}

fn dialog_item_dimensions(
    widget_width: i32,
    widget_height: i32,
    description: Option<&DialogTextBlock>,
) -> (i32, i32) {
    description.map_or((widget_width, widget_height), |description| {
        (
            widget_width
                .saturating_add(ITEM_DESCRIPTION_SPACING)
                .saturating_add(i32::try_from(description.width).unwrap_or(i32::MAX)),
            widget_height.max(description.height),
        )
    })
}

impl DialogFooterLayout {
    fn new(
        action_widths: &[i32],
        footer_height: i32,
        surface_size: PhysicalSize<u32>,
    ) -> Option<Self> {
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        let screen_width_i32 = i32::try_from(screen_width).ok()?;
        let screen_height_i32 = i32::try_from(screen_height).ok()?;
        let button_widths = action_widths
            .iter()
            .map(|width| u32::try_from(*width).ok())
            .collect::<Option<Vec<_>>>()?;
        let spacing_width = SIMPLE_BUTTON_SPACING.saturating_mul(
            i32::try_from(button_widths.len().saturating_sub(1)).unwrap_or(i32::MAX),
        );
        let total_width = button_widths.iter().fold(spacing_width, |total, width| {
            total.saturating_add(i32::try_from(*width).unwrap_or(i32::MAX))
        });
        let button_y = screen_height_i32 - footer_height
            + (footer_height - i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX)) / 2;
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

impl DialogWarningButtonLayout {
    fn new(title: &DialogTextState, surface_size: PhysicalSize<u32>) -> Option<Self> {
        let screen_width = i32::try_from(surface_size.width.clamp(1, 512)).ok()?;
        let screen_height = i32::try_from(surface_size.height.clamp(1, 512)).ok()?;
        let title_width = i32::try_from(runs_approx_width(&dialog_text_runs(title))).ok()?;
        let row_width = title_width
            .saturating_add(WARNING_BUTTON_SPACING)
            .saturating_add(WARNING_BUTTON_SIZE);
        let row_x = (screen_width - row_width) / 2;
        let row_y = (HEADER_HEIGHT - WARNING_BUTTON_SIZE)
            .saturating_add(1)
            .div_euclid(2);
        let mut x = row_x
            .saturating_add(title_width)
            .saturating_add(WARNING_BUTTON_SPACING);
        let mut y = row_y;
        if x < 0
            || y < 0
            || x > screen_width.saturating_sub(WARNING_BUTTON_SIZE)
            || y > screen_height.saturating_sub(WARNING_BUTTON_SIZE)
        {
            x = 0.max(screen_width.saturating_sub(40));
            y = 5.min(screen_height);
        }
        Some(Self {
            title_x: row_x,
            title_y: row_y.saturating_add(
                (WARNING_BUTTON_SIZE - TEXT_LINE_HEIGHT)
                    .saturating_add(1)
                    .div_euclid(2),
            ),
            x,
            y,
        })
    }

    fn contains(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        dialog_widget_contains(
            self.x,
            self.y,
            WARNING_BUTTON_SIZE,
            WARNING_BUTTON_SIZE,
            cursor_position,
            surface_size,
            screen_width,
            screen_height,
        )
    }
}

impl DialogWarningScreenLayout {
    fn new(message_line_count: usize, surface_size: PhysicalSize<u32>) -> Self {
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        let screen_width_i32 = i32::try_from(screen_width).unwrap_or(i32::MAX);
        let screen_height_i32 = i32::try_from(screen_height).unwrap_or(i32::MAX);
        let message_height = i32::try_from(message_line_count)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT);
        let layout_height = TEXT_LINE_HEIGHT
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP)
            .saturating_add(i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX));
        let layout_y = (screen_height_i32 - layout_height) / 2;
        let button_row_width = i32::try_from(CONFIRM_BUTTON_WIDTH)
            .unwrap_or(i32::MAX)
            .saturating_mul(2)
            .saturating_add(CONFIRM_BUTTON_SPACING);
        let yes_x = (screen_width_i32 - button_row_width) / 2;
        let button_y = layout_y
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP);
        Self {
            screen_width,
            screen_height,
            title_y: layout_y,
            message_y: layout_y
                .saturating_add(TEXT_LINE_HEIGHT)
                .saturating_add(CONFIRM_ELEMENT_SPACING),
            yes: DialogButtonLayout {
                x: yes_x,
                y: button_y,
                width: CONFIRM_BUTTON_WIDTH,
            },
            no: DialogButtonLayout {
                x: yes_x
                    .saturating_add(i32::try_from(CONFIRM_BUTTON_WIDTH).unwrap_or(i32::MAX))
                    .saturating_add(CONFIRM_BUTTON_SPACING),
                y: button_y,
                width: CONFIRM_BUTTON_WIDTH,
            },
        }
    }

    fn button_at(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> Option<bool> {
        [(&self.yes, true), (&self.no, false)]
            .into_iter()
            .find_map(|(button, result)| {
                dialog_widget_contains(
                    button.x,
                    button.y,
                    i32::try_from(button.width).unwrap_or(i32::MAX),
                    i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                    cursor_position,
                    surface_size,
                    self.screen_width,
                    self.screen_height,
                )
                .then_some(result)
            })
    }
}

impl DialogConfirmLinkLayout {
    fn new(message_line_count: usize, surface_size: PhysicalSize<u32>) -> Self {
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        let screen_width_i32 = i32::try_from(screen_width).unwrap_or(i32::MAX);
        let screen_height_i32 = i32::try_from(screen_height).unwrap_or(i32::MAX);
        let message_height = i32::try_from(message_line_count)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT);
        let layout_height = TEXT_LINE_HEIGHT
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP)
            .saturating_add(i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX));
        let layout_y = (screen_height_i32 - layout_height) / 2;
        let warning_y = layout_y
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING);
        let button_y = warning_y
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP);
        let button_width = i32::try_from(CONFIRM_LINK_BUTTON_WIDTH).unwrap_or(i32::MAX);
        let button_row_width = button_width
            .saturating_mul(3)
            .saturating_add(CONFIRM_BUTTON_SPACING.saturating_mul(2));
        let button_x = (screen_width_i32 - button_row_width) / 2;
        Self {
            screen_width,
            screen_height,
            title_y: layout_y,
            message_y: layout_y
                .saturating_add(TEXT_LINE_HEIGHT)
                .saturating_add(CONFIRM_ELEMENT_SPACING),
            warning_y,
            buttons: [
                DialogButtonLayout {
                    x: button_x,
                    y: button_y,
                    width: CONFIRM_LINK_BUTTON_WIDTH,
                },
                DialogButtonLayout {
                    x: button_x
                        .saturating_add(button_width)
                        .saturating_add(CONFIRM_BUTTON_SPACING),
                    y: button_y,
                    width: CONFIRM_LINK_BUTTON_WIDTH,
                },
                DialogButtonLayout {
                    x: button_x
                        .saturating_add(button_width.saturating_mul(2))
                        .saturating_add(CONFIRM_BUTTON_SPACING.saturating_mul(2)),
                    y: button_y,
                    width: CONFIRM_LINK_BUTTON_WIDTH,
                },
            ],
        }
    }

    fn button_at(
        self,
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

#[derive(Clone, Copy)]
enum DialogControlAction<'a> {
    Button(&'a DialogButtonState),
    DialogListEntry {
        index: usize,
        label: &'a DialogTextState,
        width: i32,
    },
    ServerLink {
        label: &'a str,
        url: &'a str,
        width: i32,
    },
}

impl<'a> DialogControlAction<'a> {
    fn width(self) -> i32 {
        match self {
            Self::Button(button) => button.width,
            Self::DialogListEntry { width, .. } | Self::ServerLink { width, .. } => width,
        }
    }

    fn styled_label(self) -> Option<&'a DialogTextState> {
        match self {
            Self::Button(button) => Some(&button.label),
            Self::DialogListEntry { label, .. } => Some(label),
            Self::ServerLink { .. } => None,
        }
    }

    fn label_text(self) -> &'a str {
        match self {
            Self::Button(button) => &button.label.text,
            Self::DialogListEntry { label, .. } => &label.text,
            Self::ServerLink { label, .. } => label,
        }
    }

    fn tooltip(self) -> Option<&'a DialogTextState> {
        match self {
            Self::Button(button) => button.tooltip.as_ref(),
            Self::DialogListEntry { .. } | Self::ServerLink { .. } => None,
        }
    }

    fn activation(self) -> DialogControlActivation {
        match self {
            Self::Button(button) => DialogControlActivation::Action(button.action_data.clone()),
            Self::DialogListEntry { index, .. } => DialogControlActivation::DialogListEntry(index),
            Self::ServerLink { url, .. } => {
                DialogControlActivation::Action(Some(DialogActionState::OpenUrl {
                    url: url.to_string(),
                }))
            }
        }
    }
}

enum DialogControlActivation {
    Action(Option<DialogActionState>),
    DialogListEntry(usize),
}

struct DialogControls<'a> {
    common: &'a CommonDialogDataState,
    actions: Vec<DialogControlAction<'a>>,
    body_action_count: usize,
    body_action_columns: i32,
    cancel: Option<&'a DialogButtonState>,
    footer_height: i32,
}

impl DialogControls<'_> {
    fn body_actions(&self) -> &[DialogControlAction<'_>] {
        &self.actions[..self.body_action_count]
    }

    fn footer_actions(&self) -> &[DialogControlAction<'_>] {
        &self.actions[self.body_action_count..]
    }
}

fn dialog_controls<'a>(
    data: &'a DialogDataState,
    server_links: &'a [ServerLinkState],
) -> Option<DialogControls<'a>> {
    match data {
        DialogDataState::Notice { common, action } => Some(DialogControls {
            common,
            actions: vec![DialogControlAction::Button(action)],
            body_action_count: 0,
            body_action_columns: 1,
            cancel: Some(action),
            footer_height: FOOTER_HEIGHT,
        }),
        DialogDataState::Confirmation { common, yes, no } => Some(DialogControls {
            common,
            actions: vec![
                DialogControlAction::Button(yes),
                DialogControlAction::Button(no),
            ],
            body_action_count: 0,
            body_action_columns: 1,
            cancel: Some(no),
            footer_height: FOOTER_HEIGHT,
        }),
        DialogDataState::MultiAction {
            common,
            actions,
            exit_action,
            columns,
        } => {
            let mut all_actions = actions
                .iter()
                .map(DialogControlAction::Button)
                .collect::<Vec<_>>();
            all_actions.extend(exit_action.iter().map(DialogControlAction::Button));
            Some(DialogControls {
                common,
                actions: all_actions,
                body_action_count: actions.len(),
                body_action_columns: *columns,
                cancel: exit_action.as_ref(),
                footer_height: if exit_action.is_some() {
                    FOOTER_HEIGHT
                } else {
                    5
                },
            })
        }
        DialogDataState::DialogList {
            common,
            dialogs,
            exit_action,
            columns,
            button_width,
        } => {
            let mut all_actions = dialogs
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| DialogControlAction::DialogListEntry {
                    index,
                    label: &entry.external_title,
                    width: *button_width,
                })
                .collect::<Vec<_>>();
            all_actions.extend(exit_action.iter().map(DialogControlAction::Button));
            Some(DialogControls {
                common,
                actions: all_actions,
                body_action_count: dialogs.entries.len(),
                body_action_columns: *columns,
                cancel: exit_action.as_ref(),
                footer_height: if exit_action.is_some() {
                    FOOTER_HEIGHT
                } else {
                    5
                },
            })
        }
        DialogDataState::ServerLinks {
            common,
            exit_action,
            columns,
            button_width,
        } => {
            let mut all_actions = server_links
                .iter()
                .map(|link| DialogControlAction::ServerLink {
                    label: server_link_display_label(link),
                    url: &link.url,
                    width: *button_width,
                })
                .collect::<Vec<_>>();
            all_actions.extend(exit_action.iter().map(DialogControlAction::Button));
            Some(DialogControls {
                common,
                actions: all_actions,
                body_action_count: server_links.len(),
                body_action_columns: *columns,
                cancel: exit_action.as_ref(),
                footer_height: if exit_action.is_some() {
                    FOOTER_HEIGHT
                } else {
                    5
                },
            })
        }
    }
}

fn server_link_display_label(link: &ServerLinkState) -> &str {
    match link.known_type.as_deref() {
        Some("report_bug") => "Report Server Bug",
        Some("community_guidelines") => "Community Guidelines",
        Some("support") => "Support",
        Some("status") => "Status",
        Some("feedback") => "Feedback",
        Some("community") => "Community",
        Some("website") => "Website",
        Some("forums") => "Forums",
        Some("news") => "News",
        Some("announcements") => "Announcements",
        _ => &link.label,
    }
}

fn dialog_body_action_grid_layout(
    controls: &DialogControls<'_>,
) -> Option<DialogBodyActionGridLayout> {
    let actions = controls.body_actions();
    if actions.is_empty() {
        return None;
    }
    let count = actions.len();
    let columns = usize::try_from(controls.body_action_columns).ok()?.max(1);
    let widths = actions
        .iter()
        .map(|action| i32::try_from(u32::try_from(action.width()).ok()?).ok())
        .collect::<Option<Vec<_>>>()?;
    if columns > count {
        let row_width = widths.iter().copied().fold(
            i32::try_from(count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
            i32::saturating_add,
        );
        let spacing_width = i32::try_from(columns.saturating_sub(1))
            .unwrap_or(i32::MAX)
            .saturating_mul(BUTTON_LIST_SPACING);
        // GridLayout lets the partial LinearLayout span the declared column
        // count. Its Divisor can contribute only non-negative column widths;
        // preserve that result without allocating an attacker-sized column
        // array when the codec's positive column count exceeds the actions.
        let grid_width =
            spacing_width.saturating_add(row_width.saturating_sub(spacing_width).max(0));
        let mut x = (grid_width.saturating_sub(row_width)) / 2;
        let mut buttons = Vec::with_capacity(count);
        for width in widths {
            buttons.push(DialogButtonLayout {
                x,
                y: 0,
                width: u32::try_from(width).ok()?,
            });
            x = x.saturating_add(width).saturating_add(BUTTON_LIST_SPACING);
        }
        return Some(DialogBodyActionGridLayout {
            width: grid_width,
            height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
            buttons,
        });
    }
    let complete_rows = count / columns;
    let remainder = count % columns;
    let complete_count = complete_rows.saturating_mul(columns);
    let mut column_widths = vec![0i32; columns];
    for (index, width) in widths.iter().copied().take(complete_count).enumerate() {
        column_widths[index % columns] = column_widths[index % columns].max(width);
    }
    if remainder > 0 {
        let last_row_width = widths[complete_count..].iter().copied().fold(
            i32::try_from(remainder.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
            i32::saturating_add,
        );
        let numerator = last_row_width.saturating_sub(
            i32::try_from(columns.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
        );
        let denominator = i32::try_from(columns).ok()?;
        let quotient = numerator / denominator;
        let modulus = numerator % denominator;
        let mut carry = 0i32;
        for column_width in &mut column_widths {
            let mut part = quotient;
            carry = carry.saturating_add(modulus);
            if carry >= denominator {
                carry = carry.saturating_sub(denominator);
                part = part.saturating_add(1);
            }
            *column_width = (*column_width).max(part);
        }
    }
    let grid_width = column_widths.iter().copied().fold(
        i32::try_from(columns.saturating_sub(1))
            .unwrap_or(i32::MAX)
            .saturating_mul(BUTTON_LIST_SPACING),
        i32::saturating_add,
    );
    let rows = complete_rows.saturating_add(usize::from(remainder > 0));
    let grid_height = i32::try_from(rows)
        .unwrap_or(i32::MAX)
        .saturating_mul(i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX))
        .saturating_add(
            i32::try_from(rows.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
        );
    let mut column_offsets = Vec::with_capacity(columns);
    let mut next_x = 0i32;
    for width in &column_widths {
        column_offsets.push(next_x);
        next_x = next_x
            .saturating_add(*width)
            .saturating_add(BUTTON_LIST_SPACING);
    }
    let mut buttons = Vec::with_capacity(count);
    for (index, width) in widths.iter().copied().take(complete_count).enumerate() {
        let row = index / columns;
        let column = index % columns;
        buttons.push(DialogButtonLayout {
            x: column_offsets[column]
                .saturating_add((column_widths[column].saturating_sub(width)) / 2),
            y: i32::try_from(row).unwrap_or(i32::MAX).saturating_mul(
                i32::try_from(BUTTON_HEIGHT)
                    .unwrap_or(i32::MAX)
                    .saturating_add(BUTTON_LIST_SPACING),
            ),
            width: u32::try_from(width).ok()?,
        });
    }
    if remainder > 0 {
        let last_widths = &widths[complete_count..];
        let last_row_width = last_widths.iter().copied().fold(
            i32::try_from(remainder.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
            i32::saturating_add,
        );
        let mut x = (grid_width.saturating_sub(last_row_width)) / 2;
        let y = i32::try_from(complete_rows)
            .unwrap_or(i32::MAX)
            .saturating_mul(
                i32::try_from(BUTTON_HEIGHT)
                    .unwrap_or(i32::MAX)
                    .saturating_add(BUTTON_LIST_SPACING),
            );
        for width in last_widths {
            buttons.push(DialogButtonLayout {
                x,
                y,
                width: u32::try_from(*width).ok()?,
            });
            x = x.saturating_add(*width).saturating_add(BUTTON_LIST_SPACING);
        }
    }
    Some(DialogBodyActionGridLayout {
        width: grid_width,
        height: grid_height,
        buttons,
    })
}

fn dialog_content_layout(controls: &DialogControls<'_>) -> Option<DialogContentLayout> {
    let body_dimensions = controls
        .common
        .body_data
        .iter()
        .map(dialog_body_declared_dimensions)
        .collect::<Option<Vec<_>>>()?;
    let input_dimensions = controls
        .common
        .input_data
        .iter()
        .map(|input| dialog_input_dimensions(&input.control))
        .collect::<Option<Vec<_>>>()?;
    let action_grid = dialog_body_action_grid_layout(controls);
    let element_count = body_dimensions
        .len()
        .saturating_add(input_dimensions.len())
        .saturating_add(usize::from(action_grid.is_some()));
    let width = body_dimensions
        .iter()
        .chain(&input_dimensions)
        .map(|(width, _)| *width)
        .max()
        .unwrap_or(0)
        .max(action_grid.as_ref().map_or(0, |grid| grid.width));
    let height = body_dimensions
        .iter()
        .chain(&input_dimensions)
        .map(|(_, height)| *height)
        .fold(0i32, i32::saturating_add)
        .saturating_add(action_grid.as_ref().map_or(0, |grid| grid.height))
        .saturating_add(
            i32::try_from(element_count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BODY_SPACING),
        );
    Some(DialogContentLayout {
        body_dimensions,
        input_dimensions,
        action_grid,
        width,
        height,
    })
}

fn dialog_body_declared_dimensions(body: &DialogBodyState) -> Option<(i32, i32)> {
    match body {
        DialogBodyState::PlainMessage { message } => {
            let text = dialog_text_block(message)?;
            Some((i32::try_from(text.width).ok()?, text.height))
        }
        DialogBodyState::Item {
            description,
            width,
            height,
            ..
        } => {
            let description = description.as_ref().and_then(dialog_text_block);
            Some(dialog_item_dimensions(
                *width,
                *height,
                description.as_ref(),
            ))
        }
    }
}

fn dialog_content_y(screen_height: i32, content_height: i32, footer_height: i32) -> i32 {
    let container_height = content_height.min(
        screen_height
            .saturating_sub(HEADER_HEIGHT)
            .saturating_sub(footer_height),
    );
    (HEADER_HEIGHT + CONTENT_MARGIN_TOP).min(
        screen_height
            .saturating_sub(footer_height)
            .saturating_sub(container_height),
    )
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

#[cfg(test)]
fn dialog_input_layouts(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
) -> Option<Vec<DialogInputLayout>> {
    dialog_input_layouts_at_scroll(controls, surface_size, 0)
}

fn dialog_input_layouts_at_scroll(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
    scroll_amount: i32,
) -> Option<Vec<DialogInputLayout>> {
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let screen_width_i32 = i32::try_from(screen_width).ok()?;
    let screen_height_i32 = i32::try_from(screen_height).ok()?;
    let content = dialog_content_layout(controls)?;
    let mut y = dialog_content_y(screen_height_i32, content.height, controls.footer_height)
        .saturating_sub(scroll_amount);
    let mut ordinal = 0usize;
    for (_, height) in content.body_dimensions {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        ordinal = ordinal.saturating_add(1);
        y = y.saturating_add(height);
    }
    content
        .input_dimensions
        .into_iter()
        .enumerate()
        .map(|(index, (width, height))| {
            if ordinal > 0 {
                y = y.saturating_add(BODY_SPACING);
            }
            ordinal = ordinal.saturating_add(1);
            let layout = DialogInputLayout {
                index,
                x: screen_width_i32 / 2 - width / 2,
                y,
                width,
                height,
            };
            y = y.saturating_add(height);
            Some(layout)
        })
        .collect()
}

#[cfg(test)]
fn dialog_focus_graph(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
) -> Option<DialogFocusGraph> {
    dialog_focus_graph_at_scroll(controls, surface_size, 0)
}

fn dialog_focus_graph_at_scroll(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
    scroll_amount: i32,
) -> Option<DialogFocusGraph> {
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let screen_width_i32 = i32::try_from(screen_width).ok()?;
    let screen_height_i32 = i32::try_from(screen_height).ok()?;
    let content = dialog_content_layout(controls)?;
    let content_y = dialog_content_y(screen_height_i32, content.height, controls.footer_height);
    let mut y = content_y.saturating_sub(scroll_amount);
    let mut body = Vec::new();
    let mut ordinal = 0usize;
    for (index, (body_state, (block_width, block_height))) in controls
        .common
        .body_data
        .iter()
        .zip(content.body_dimensions.iter().copied())
        .enumerate()
    {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        ordinal = ordinal.saturating_add(1);
        let block_x = screen_width_i32 / 2 - block_width / 2;
        match body_state {
            DialogBodyState::PlainMessage { .. } => body.push(DialogFocusNode {
                target: DialogFocusState::Body {
                    index,
                    part: DialogBodyFocusPartState::Primary,
                },
                rectangle: DialogFocusRectangle {
                    x: block_x,
                    y,
                    width: block_width,
                    height: block_height,
                },
            }),
            DialogBodyState::Item {
                description,
                width,
                height,
                ..
            } => {
                let item_y = y.saturating_add(vertical_center_offset(block_height, *height));
                body.push(DialogFocusNode {
                    target: DialogFocusState::Body {
                        index,
                        part: DialogBodyFocusPartState::Primary,
                    },
                    rectangle: DialogFocusRectangle {
                        x: block_x,
                        y: item_y,
                        width: *width,
                        height: *height,
                    },
                });
                if let Some(description) = description.as_ref().and_then(dialog_text_block) {
                    body.push(DialogFocusNode {
                        target: DialogFocusState::Body {
                            index,
                            part: DialogBodyFocusPartState::Description,
                        },
                        rectangle: DialogFocusRectangle {
                            x: block_x
                                .saturating_add(*width)
                                .saturating_add(ITEM_DESCRIPTION_SPACING),
                            y: y.saturating_add(vertical_center_offset(
                                block_height,
                                description.height,
                            )),
                            width: i32::try_from(description.width).unwrap_or(i32::MAX),
                            height: description.height,
                        },
                    });
                }
            }
        }
        y = y.saturating_add(block_height);
    }
    for (index, (input, (width, height))) in controls
        .common
        .input_data
        .iter()
        .zip(content.input_dimensions.iter().copied())
        .enumerate()
    {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        ordinal = ordinal.saturating_add(1);
        let layout = DialogInputLayout {
            index,
            x: screen_width_i32 / 2 - width / 2,
            y,
            width,
            height,
        };
        let focus_layout = match &input.control {
            DialogInputControlState::Boolean { .. }
            | DialogInputControlState::NumberRange { .. }
            | DialogInputControlState::SingleOption { .. } => Some(layout),
            DialogInputControlState::Text {
                label_visible,
                multiline: None,
                ..
            } => Some(dialog_single_line_text_box_layout(layout, *label_visible)),
            DialogInputControlState::Text {
                label_visible,
                multiline: Some(_),
                ..
            } => Some(dialog_multiline_text_box_layout(layout, *label_visible)),
        };
        if let Some(layout) = focus_layout {
            body.push(DialogFocusNode {
                target: DialogFocusState::Input { index },
                rectangle: DialogFocusRectangle {
                    x: layout.x,
                    y: layout.y,
                    width: layout.width,
                    height: layout.height,
                },
            });
        }
        y = y.saturating_add(height);
    }
    if let Some(action_grid) = content.action_grid.as_ref() {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        let grid_x = screen_width_i32 / 2 - action_grid.width / 2;
        body.extend(
            action_grid
                .buttons
                .iter()
                .enumerate()
                .map(|(index, button)| DialogFocusNode {
                    target: DialogFocusState::Action { index },
                    rectangle: DialogFocusRectangle {
                        x: grid_x.saturating_add(button.x),
                        y: y.saturating_add(button.y),
                        width: i32::try_from(button.width).unwrap_or(i32::MAX),
                        height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                    },
                }),
        );
    }

    let container_width = content.width.saturating_add(BODY_SCROLLBAR_RESERVE * 2);
    let container_height = content.height.min(
        screen_height_i32
            .saturating_sub(HEADER_HEIGHT)
            .saturating_sub(controls.footer_height),
    );
    let container = DialogBodyFocusContainer {
        rectangle: DialogFocusRectangle {
            x: screen_width_i32 / 2 - container_width / 2,
            y: content_y,
            width: container_width,
            height: container_height,
        },
        content_rectangle: DialogFocusRectangle {
            x: screen_width_i32 / 2 - container_width / 2,
            y: content_y.saturating_sub(scroll_amount),
            width: container_width,
            height: content.height,
        },
    };
    let warning = DialogWarningButtonLayout::new(&controls.common.title, surface_size)?;
    let warning = DialogFocusNode {
        target: DialogFocusState::WarningButton,
        rectangle: DialogFocusRectangle {
            x: warning.x,
            y: warning.y,
            width: WARNING_BUTTON_SIZE,
            height: WARNING_BUTTON_SIZE,
        },
    };
    let action_layout = DialogFooterLayout::new(
        &controls
            .footer_actions()
            .iter()
            .map(|action| action.width())
            .collect::<Vec<_>>(),
        controls.footer_height,
        surface_size,
    )?;
    let actions = action_layout
        .buttons
        .iter()
        .enumerate()
        .map(|(index, button)| DialogFocusNode {
            target: DialogFocusState::Action {
                index: controls.body_action_count.saturating_add(index),
            },
            rectangle: DialogFocusRectangle {
                x: button.x,
                y: button.y,
                width: i32::try_from(button.width).unwrap_or(i32::MAX),
                height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
            },
        })
        .collect::<Vec<_>>();
    let mut top = Vec::with_capacity(actions.len().saturating_add(2));
    top.push(DialogTopFocusNode::Body(container));
    top.extend(actions.iter().copied().map(DialogTopFocusNode::Target));
    top.push(DialogTopFocusNode::Target(warning));
    let mut tab = Vec::with_capacity(body.len().saturating_add(actions.len()).saturating_add(1));
    tab.push(warning.target);
    tab.extend(body.iter().map(|node| node.target));
    tab.extend(actions.iter().map(|node| node.target));
    Some(DialogFocusGraph {
        screen: DialogFocusRectangle {
            x: 0,
            y: 0,
            width: screen_width_i32,
            height: screen_height_i32,
        },
        body,
        top,
        tab,
    })
}

fn dialog_body_scroll_layout(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
    scroll_amount: f64,
) -> Option<DialogBodyScrollLayout> {
    let screen_width = i32::try_from(surface_size.width.clamp(1, 512)).ok()?;
    let screen_height = i32::try_from(surface_size.height.clamp(1, 512)).ok()?;
    let content = dialog_content_layout(controls)?;
    let height = content.height.min(
        screen_height
            .saturating_sub(HEADER_HEIGHT)
            .saturating_sub(controls.footer_height),
    );
    let max_scroll = content.height.saturating_sub(height).max(0);
    let scroll_amount = if scroll_amount.is_finite() {
        scroll_amount.clamp(0.0, f64::from(max_scroll)) as i32
    } else {
        0
    };
    let width = content.width.saturating_add(BODY_SCROLLBAR_RESERVE * 2);
    Some(DialogBodyScrollLayout {
        rectangle: DialogFocusRectangle {
            x: screen_width / 2 - width / 2,
            y: dialog_content_y(screen_height, content.height, controls.footer_height),
            width,
            height,
        },
        content_height: content.height,
        max_scroll,
        scroll_amount,
    })
}

fn dialog_scrollbar_layout(layout: DialogBodyScrollLayout) -> Option<DialogScrollbarLayout> {
    if layout.max_scroll <= 0 || layout.rectangle.height <= 8 || layout.content_height <= 0 {
        return None;
    }
    let thumb_height = layout
        .rectangle
        .height
        .saturating_mul(layout.rectangle.height)
        .div_euclid(layout.content_height)
        .clamp(32, layout.rectangle.height.saturating_sub(8));
    let thumb_y = layout.rectangle.y.max(
        layout
            .scroll_amount
            .saturating_mul(layout.rectangle.height.saturating_sub(thumb_height))
            .div_euclid(layout.max_scroll)
            .saturating_add(layout.rectangle.y),
    );
    Some(DialogScrollbarLayout {
        x: layout
            .rectangle
            .x
            .saturating_add(layout.rectangle.width)
            .saturating_sub(6),
        y: layout.rectangle.y,
        width: 6,
        height: layout.rectangle.height,
        thumb_y,
        thumb_height,
    })
}

fn dialog_body_viewport_contains(
    layout: DialogBodyScrollLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> bool {
    dialog_widget_contains(
        layout.rectangle.x,
        layout.rectangle.y,
        layout.rectangle.width,
        layout.rectangle.height,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    )
}

fn dialog_cursor_screen_y(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_height: u32,
) -> Option<i32> {
    let cursor_y = cursor_position?.y.floor();
    if !cursor_y.is_finite() {
        return None;
    }
    let (_, origin_y) = centered_screen_origin(
        surface_size,
        surface_size.width.clamp(1, 512),
        screen_height,
    );
    i32::try_from((cursor_y as i64).saturating_sub(origin_y)).ok()
}

fn dialog_scrollbar_contains(
    scrollbar: DialogScrollbarLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let left = origin_x + i64::from(scrollbar.x);
    let top = origin_y + i64::from(scrollbar.y);
    cursor.x >= left as f64
        && cursor.x <= (left + i64::from(scrollbar.width)) as f64
        && cursor.y >= top as f64
        && cursor.y < (top + i64::from(scrollbar.height)) as f64
}

impl DialogFocusRectangle {
    fn axis_bound(self, horizontal: bool, positive: bool) -> i32 {
        let (start, length) = if horizontal {
            (self.x, self.width)
        } else {
            (self.y, self.height)
        };
        if positive {
            start.saturating_add(length).saturating_sub(1)
        } else {
            start
        }
    }

    fn center(self, horizontal: bool) -> i32 {
        let low = self.axis_bound(horizontal, false);
        let high = self.axis_bound(horizontal, true);
        low.saturating_add(high) / 2
    }

    fn overlaps_axis(self, other: Self, horizontal: bool) -> bool {
        self.axis_bound(horizontal, false)
            .max(other.axis_bound(horizontal, false))
            <= self
                .axis_bound(horizontal, true)
                .min(other.axis_bound(horizontal, true))
    }

    fn border(self, direction: DialogFocusDirection) -> Self {
        match direction {
            DialogFocusDirection::Up => Self {
                x: self.x,
                y: self.y.saturating_sub(1),
                width: self.width,
                height: 1,
            },
            DialogFocusDirection::Down => Self {
                x: self.x,
                y: self.y.saturating_add(self.height),
                width: self.width,
                height: 1,
            },
            DialogFocusDirection::Left => Self {
                x: self.x.saturating_sub(1),
                y: self.y,
                width: 1,
                height: self.height,
            },
            DialogFocusDirection::Right => Self {
                x: self.x.saturating_add(self.width),
                y: self.y,
                width: 1,
                height: self.height,
            },
        }
    }
}

impl DialogFocusDirection {
    fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    fn positive(self) -> bool {
        matches!(self, Self::Down | Self::Right)
    }

    fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    fn is_after(self, value: i32, reference: i32) -> bool {
        if self.positive() {
            value > reference
        } else {
            value < reference
        }
    }
}

fn ordered_dialog_focus_candidates<T: Copy>(
    focused: DialogFocusRectangle,
    direction: DialogFocusDirection,
    candidates: impl IntoIterator<Item = (T, DialogFocusRectangle)>,
) -> Vec<T> {
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    let horizontal = direction.horizontal();
    let other_horizontal = !horizontal;
    let focused_first = focused.axis_bound(horizontal, !direction.positive());
    let mut exact = candidates
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, (_, rectangle))| {
            if !rectangle.overlaps_axis(focused, other_horizontal) {
                return false;
            }
            let first = rectangle.axis_bound(horizontal, !direction.positive());
            direction.is_after(first, focused_first)
                || (first == focused_first
                    && direction.is_after(
                        rectangle.axis_bound(horizontal, direction.positive()),
                        focused.axis_bound(horizontal, direction.positive()),
                    ))
        })
        .collect::<Vec<_>>();
    exact.sort_by(|(left_index, (_, left)), (right_index, (_, right))| {
        let left_primary = left.axis_bound(horizontal, !direction.positive());
        let right_primary = right.axis_bound(horizontal, !direction.positive());
        let primary = if direction.positive() {
            left_primary.cmp(&right_primary)
        } else {
            right_primary.cmp(&left_primary)
        };
        primary
            .then_with(|| {
                left.axis_bound(other_horizontal, false)
                    .cmp(&right.axis_bound(other_horizontal, false))
            })
            .then_with(|| left_index.cmp(right_index))
    });
    let focused_axis = focused.axis_bound(horizontal, direction.positive());
    let focused_other = focused.center(other_horizontal);
    let mut vague = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, (target, rectangle))| {
            let candidate_axis = rectangle.axis_bound(horizontal, !direction.positive());
            if !direction.is_after(candidate_axis, focused_axis) {
                return None;
            }
            let candidate_other = rectangle.center(other_horizontal);
            let axis_delta = i64::from(candidate_axis) - i64::from(focused_axis);
            let other_delta = i64::from(candidate_other) - i64::from(focused_other);
            Some((
                index,
                target,
                axis_delta
                    .saturating_mul(axis_delta)
                    .saturating_add(other_delta.saturating_mul(other_delta)),
            ))
        })
        .collect::<Vec<_>>();
    vague.sort_by_key(|(index, _, distance)| (*distance, *index));
    exact
        .into_iter()
        .map(|(_, (target, _))| target)
        .chain(vague.into_iter().map(|(_, target, _)| target))
        .collect()
}

fn dialog_focus_node(
    nodes: &[DialogFocusNode],
    target: DialogFocusState,
) -> Option<DialogFocusNode> {
    nodes.iter().copied().find(|node| node.target == target)
}

fn next_body_focus_from_rectangle(
    graph: &DialogFocusGraph,
    focused: DialogFocusRectangle,
    direction: DialogFocusDirection,
    excluded: Option<DialogFocusState>,
) -> Option<DialogFocusState> {
    ordered_dialog_focus_candidates(
        focused,
        direction,
        graph.body.iter().filter_map(|node| {
            (Some(node.target) != excluded).then_some((node.target, node.rectangle))
        }),
    )
    .into_iter()
    .next()
}

fn resolve_top_focus_candidates(
    graph: &DialogFocusGraph,
    focused: DialogFocusRectangle,
    direction: DialogFocusDirection,
    excluded: Option<DialogFocusState>,
    exclude_body: bool,
    body_previous_focus: Option<DialogFocusRectangle>,
) -> Option<DialogFocusState> {
    let candidates = graph
        .top
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            DialogTopFocusNode::Body(container) if !exclude_body && !graph.body.is_empty() => {
                Some((index, container.rectangle))
            }
            DialogTopFocusNode::Target(node) if Some(node.target) != excluded => {
                Some((index, node.rectangle))
            }
            _ => None,
        });
    for index in ordered_dialog_focus_candidates(focused, direction, candidates) {
        match graph.top[index] {
            DialogTopFocusNode::Target(node) => return Some(node.target),
            DialogTopFocusNode::Body(container) => {
                let body_focused = body_previous_focus
                    .unwrap_or_else(|| container.content_rectangle.border(direction.opposite()));
                if let Some(target) =
                    next_body_focus_from_rectangle(graph, body_focused, direction, None)
                {
                    return Some(target);
                }
            }
        }
    }
    None
}

fn next_dialog_focus_in_direction(
    graph: &DialogFocusGraph,
    current: Option<DialogFocusState>,
    direction: DialogFocusDirection,
) -> Option<DialogFocusState> {
    if let Some(current) = current {
        if let Some(node) = dialog_focus_node(&graph.body, current) {
            let border = node.rectangle.border(direction);
            if let Some(target) =
                next_body_focus_from_rectangle(graph, border, direction, Some(current))
            {
                return Some(target);
            }
            return resolve_top_focus_candidates(
                graph,
                border,
                direction,
                None,
                true,
                Some(border),
            );
        }
        let top_node = graph.top.iter().find_map(|node| match node {
            DialogTopFocusNode::Target(node) if node.target == current => Some(*node),
            _ => None,
        })?;
        let border = top_node.rectangle.border(direction);
        return resolve_top_focus_candidates(
            graph,
            border,
            direction,
            Some(current),
            false,
            Some(border),
        );
    }

    let border = graph.screen.border(direction.opposite());
    resolve_top_focus_candidates(graph, border, direction, None, false, None)
}

fn next_dialog_tab_focus(
    graph: &DialogFocusGraph,
    current: Option<DialogFocusState>,
    forward: bool,
) -> Option<DialogFocusState> {
    if graph.tab.is_empty() {
        return None;
    }
    let Some(index) =
        current.and_then(|current| graph.tab.iter().position(|node| *node == current))
    else {
        return if forward {
            graph.tab.first().copied()
        } else {
            graph.tab.last().copied()
        };
    };
    if forward {
        graph
            .tab
            .get(index.saturating_add(1))
            .copied()
            .or_else(|| graph.tab.first().copied())
    } else {
        index
            .checked_sub(1)
            .and_then(|index| graph.tab.get(index).copied())
            .or_else(|| graph.tab.last().copied())
    }
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

fn dialog_multiline_text_box_layout(
    layout: DialogInputLayout,
    label_visible: bool,
) -> DialogInputLayout {
    let label_height = if label_visible {
        TEXT_LINE_HEIGHT + 4
    } else {
        0
    };
    DialogInputLayout {
        y: layout.y.saturating_add(label_height),
        height: layout.height.saturating_sub(label_height),
        ..layout
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DialogMultilineLine {
    begin: usize,
    end: usize,
}

fn dialog_multiline_lines(
    text: &str,
    width: u32,
    text_measurer: &dyn DialogTextMeasurer,
) -> Vec<DialogMultilineLine> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut lines = Vec::new();
    let mut begin = 0usize;
    while begin < chars.len() {
        let mut cursor = begin;
        let mut last_space = None;
        let mut wrapped = false;
        while cursor < chars.len() {
            let ch = chars[cursor];
            if ch == '\n' {
                lines.push(DialogMultilineLine { begin, end: cursor });
                begin = cursor.saturating_add(1);
                wrapped = true;
                break;
            }
            if ch == ' ' {
                last_space = Some(cursor);
            }
            let candidate = chars[begin..=cursor].iter().collect::<String>();
            if cursor > begin && text_measurer.width(&candidate) > width {
                if let Some(space) = last_space.filter(|space| *space >= begin) {
                    lines.push(DialogMultilineLine { begin, end: space });
                    begin = space.saturating_add(1);
                } else {
                    lines.push(DialogMultilineLine { begin, end: cursor });
                    begin = cursor;
                }
                wrapped = true;
                break;
            }
            cursor = cursor.saturating_add(1);
        }
        if !wrapped {
            lines.push(DialogMultilineLine {
                begin,
                end: chars.len(),
            });
            begin = chars.len();
        }
    }
    if chars.is_empty() || chars.last() == Some(&'\n') {
        lines.push(DialogMultilineLine {
            begin: chars.len(),
            end: chars.len(),
        });
    }
    lines
}

fn dialog_multiline_content_height(line_count: usize) -> i32 {
    i32::try_from(line_count)
        .unwrap_or(i32::MAX)
        .saturating_mul(TEXT_LINE_HEIGHT)
        .saturating_add(8)
}

fn dialog_multiline_max_scroll(line_count: usize, height: i32) -> i32 {
    dialog_multiline_content_height(line_count)
        .saturating_sub(height)
        .max(0)
}

fn dialog_multiline_cursor_line(lines: &[DialogMultilineLine], cursor: usize) -> usize {
    lines
        .iter()
        .position(|line| cursor <= line.end)
        .unwrap_or_else(|| lines.len().saturating_sub(1))
}

fn dialog_multiline_reveal_cursor(
    scroll_amount: i32,
    height: i32,
    lines: &[DialogMultilineLine],
    cursor: usize,
) -> i32 {
    let max_scroll = dialog_multiline_max_scroll(lines.len(), height);
    let scroll_amount = scroll_amount.clamp(0, max_scroll);
    let cursor_line = dialog_multiline_cursor_line(lines, cursor);
    let first_line = usize::try_from(scroll_amount / TEXT_LINE_HEIGHT).unwrap_or(0);
    if lines
        .get(first_line)
        .is_some_and(|line| cursor <= line.begin)
    {
        return i32::try_from(cursor_line)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT)
            .clamp(0, max_scroll);
    }
    let last_line = usize::try_from(
        scroll_amount
            .saturating_add(height)
            .saturating_div(TEXT_LINE_HEIGHT)
            .saturating_sub(1),
    )
    .unwrap_or(0)
    .min(lines.len().saturating_sub(1));
    if lines.get(last_line).is_some_and(|line| cursor > line.end) {
        return i32::try_from(cursor_line)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT)
            .saturating_sub(height)
            .saturating_add(TEXT_LINE_HEIGHT + 8)
            .clamp(0, max_scroll);
    }
    scroll_amount
}

fn dialog_multiline_scrollbar_layout(
    layout: DialogInputLayout,
    line_count: usize,
    scroll_amount: i32,
) -> Option<DialogScrollbarLayout> {
    let content_height = dialog_multiline_content_height(line_count);
    let max_scroll = content_height.saturating_sub(layout.height).max(0);
    if max_scroll == 0 {
        return None;
    }
    let calculated_thumb_height = layout
        .height
        .saturating_mul(layout.height)
        .checked_div(content_height.max(1))
        .unwrap_or(0);
    let thumb_upper = layout.height.saturating_sub(8);
    let thumb_height = if thumb_upper < 32 {
        thumb_upper.max(1)
    } else {
        calculated_thumb_height.clamp(32, thumb_upper)
    };
    let travel = layout.height.saturating_sub(thumb_height);
    let thumb_y = layout.y.max(
        scroll_amount
            .clamp(0, max_scroll)
            .saturating_mul(travel)
            .checked_div(max_scroll)
            .unwrap_or(0)
            .saturating_add(layout.y),
    );
    Some(DialogScrollbarLayout {
        x: layout.x.saturating_add(layout.width),
        y: layout.y,
        width: 6,
        height: layout.height,
        thumb_y,
        thumb_height,
    })
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

fn push_full_nine_slice_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    layout: DialogInputLayout,
    texture: HudInventoryBackgroundTexture,
    sprite_width: u32,
    sprite_height: u32,
    declared_border: u32,
) {
    let width = u32::try_from(layout.width).unwrap_or(0);
    let height = u32::try_from(layout.height).unwrap_or(0);
    if width == 0 || height == 0 || sprite_width == 0 || sprite_height == 0 {
        return;
    }
    let border_x = declared_border.min(width / 2).min(sprite_width / 2);
    let border_y = declared_border.min(height / 2).min(sprite_height / 2);
    let columns = [
        (0, border_x, 0, border_x),
        (
            border_x,
            width.saturating_sub(border_x.saturating_mul(2)),
            border_x,
            sprite_width.saturating_sub(border_x.saturating_mul(2)),
        ),
        (
            width.saturating_sub(border_x),
            border_x,
            sprite_width.saturating_sub(border_x),
            border_x,
        ),
    ];
    let rows = [
        (0, border_y, 0, border_y),
        (
            border_y,
            height.saturating_sub(border_y.saturating_mul(2)),
            border_y,
            sprite_height.saturating_sub(border_y.saturating_mul(2)),
        ),
        (
            height.saturating_sub(border_y),
            border_y,
            sprite_height.saturating_sub(border_y),
            border_y,
        ),
    ];
    for (screen_y, screen_height, texture_y, texture_height) in rows {
        for (screen_x, screen_width, texture_x, texture_width) in columns {
            if screen_width == 0 || screen_height == 0 || texture_width == 0 || texture_height == 0
            {
                continue;
            }
            layers.push(HudInventoryBackgroundLayer {
                texture,
                x: layout
                    .x
                    .saturating_add(i32::try_from(screen_x).unwrap_or(i32::MAX)),
                y: layout
                    .y
                    .saturating_add(i32::try_from(screen_y).unwrap_or(i32::MAX)),
                width: screen_width,
                height: screen_height,
                uv: HudUvRect {
                    min: [
                        texture_x as f32 / sprite_width as f32,
                        texture_y as f32 / sprite_height as f32,
                    ],
                    max: [
                        texture_x.saturating_add(texture_width) as f32 / sprite_width as f32,
                        texture_y.saturating_add(texture_height) as f32 / sprite_height as f32,
                    ],
                },
            });
        }
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

#[cfg(test)]
fn hud_dialog_screen(
    world: &WorldStore,
    dialog: &DialogState,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_state: &InventoryHudLocalState,
    surface_size: PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    hud_dialog_screen_with_measurer(
        world,
        dialog,
        item_runtime,
        terrain_textures,
        local_state,
        surface_size,
        &ApproximateDialogTextMeasurer,
        partial_tick,
    )
}

pub(super) fn hud_dialog_screen_with_measurer(
    world: &WorldStore,
    dialog: &DialogState,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_state: &InventoryHudLocalState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    let controls = dialog_controls(dialog.data.as_ref()?, world.server_links())?;
    let layout = DialogFooterLayout::new(
        &controls
            .footer_actions()
            .iter()
            .map(|action| action.width())
            .collect::<Vec<_>>(),
        controls.footer_height,
        surface_size,
    )?;
    let screen_width_i32 = i32::try_from(layout.screen_width).ok()?;
    let screen_height_i32 = i32::try_from(layout.screen_height).ok()?;
    let scroll_layout =
        dialog_body_scroll_layout(&controls, surface_size, world.dialog_body_scroll_amount())?;
    let cursor_screen_position = local_state.cursor_position;
    let cursor_position = local_state
        .cursor_position
        .map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted_footer_button = layout
        .button_at(cursor_position, surface_size)
        .map(|index| controls.body_action_count.saturating_add(index));
    let warning_button = DialogWarningButtonLayout::new(&controls.common.title, surface_size)?;
    let warning_button_hovered = warning_button.contains(cursor_position, surface_size);
    let warning_button_focused = world.dialog_focus() == Some(DialogFocusState::WarningButton);
    let focused_action = match world.dialog_focus() {
        Some(DialogFocusState::Action { index }) => Some(index),
        _ => None,
    };

    let mut text_labels = Vec::new();
    push_text_label(
        &mut text_labels,
        &controls.common.title,
        warning_button.title_x,
        warning_button.title_y,
    );
    let scroll_text_start = text_labels.len();
    let content = dialog_content_layout(&controls)?;

    let body_blocks = controls
        .common
        .body_data
        .iter()
        .enumerate()
        .filter_map(|(body_index, body)| match body {
            DialogBodyState::PlainMessage { message } => dialog_text_block(message)
                .map(DialogBodyBlock::Text)
                .map(|block| (body_index, block)),
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
            .map(DialogBodyBlock::Item)
            .map(|block| (body_index, block)),
        })
        .collect::<Vec<_>>();
    let content_y = dialog_content_y(screen_height_i32, content.height, controls.footer_height);
    let mut body_y = content_y.saturating_sub(scroll_layout.scroll_amount);
    let mut floating_items = Vec::new();
    let mut scroll_fill_layers = Vec::new();
    let mut item_tooltip = None;
    for (render_index, (body_index, block)) in body_blocks.into_iter().enumerate() {
        if render_index > 0 {
            body_y = body_y.saturating_add(BODY_SPACING);
        }
        let block_x = screen_width_i32 / 2 - block.width() / 2;
        let block_height = block.height();
        match block {
            DialogBodyBlock::Text(text) => {
                if world.dialog_focus()
                    == Some(DialogFocusState::Body {
                        index: body_index,
                        part: DialogBodyFocusPartState::Primary,
                    })
                {
                    push_dialog_outline_layers(
                        &mut scroll_fill_layers,
                        block_x,
                        body_y,
                        i32::try_from(text.width).unwrap_or(i32::MAX),
                        text.height,
                    );
                }
                push_dialog_text_block(&mut text_labels, text, block_x, body_y);
            }
            DialogBodyBlock::Item(mut item) => {
                let item_y =
                    body_y.saturating_add(vertical_center_offset(block_height, item.widget_height));
                item.item.x = block_x;
                item.item.y = item_y;
                item.item.scissor = Some(HudInventoryItemScissor {
                    x: scroll_layout.rectangle.x,
                    y: scroll_layout.rectangle.y,
                    width: u32::try_from(scroll_layout.rectangle.width).unwrap_or_default(),
                    height: u32::try_from(scroll_layout.rectangle.height).unwrap_or_default(),
                });
                if world.dialog_focus()
                    == Some(DialogFocusState::Body {
                        index: body_index,
                        part: DialogBodyFocusPartState::Primary,
                    })
                {
                    push_dialog_outline_layers(
                        &mut scroll_fill_layers,
                        block_x,
                        item_y,
                        item.widget_width,
                        item.widget_height,
                    );
                }
                if item.show_tooltip
                    && dialog_body_viewport_contains(
                        scroll_layout,
                        cursor_position,
                        surface_size,
                        layout.screen_width,
                        layout.screen_height,
                    )
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
                        body_index,
                    );
                }
                floating_items.push(item.item);
                if let Some(description) = item.description {
                    let description_y = body_y
                        .saturating_add(vertical_center_offset(block_height, description.height));
                    let description_x = block_x
                        .saturating_add(item.widget_width)
                        .saturating_add(ITEM_DESCRIPTION_SPACING);
                    if world.dialog_focus()
                        == Some(DialogFocusState::Body {
                            index: body_index,
                            part: DialogBodyFocusPartState::Description,
                        })
                    {
                        push_dialog_outline_layers(
                            &mut scroll_fill_layers,
                            description_x,
                            description_y,
                            i32::try_from(description.width).unwrap_or(i32::MAX),
                            description.height,
                        );
                    }
                    push_dialog_text_block(
                        &mut text_labels,
                        description,
                        description_x,
                        description_y,
                    );
                }
            }
        }
        body_y = body_y.saturating_add(block_height);
    }

    let input_layouts =
        dialog_input_layouts_at_scroll(&controls, surface_size, scroll_layout.scroll_amount)?;
    let mut input_layers = Vec::new();
    let mut text_areas = Vec::new();
    for input_layout in input_layouts {
        let Some(input) = controls.common.input_data.get(input_layout.index) else {
            continue;
        };
        let hovered = dialog_body_viewport_contains(
            scroll_layout,
            cursor_position,
            surface_size,
            layout.screen_width,
            layout.screen_height,
        ) && dialog_widget_contains(
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
                push_scrolling_widget_text_label(&mut text_labels, runs, input_layout);
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
                push_scrolling_widget_text_label(&mut text_labels, runs, input_layout);
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
                    scrolling: None,
                    shadow: true,
                    runs: Vec::new(),
                });
            }
            (
                DialogInputControlState::Text {
                    label,
                    label_visible,
                    max_length,
                    multiline: Some(_),
                    ..
                },
                Some(Some(DialogInputValueState::Text { value })),
            ) => {
                if *label_visible {
                    push_text_label(&mut text_labels, label, input_layout.x, input_layout.y);
                }
                let box_layout = dialog_multiline_text_box_layout(input_layout, *label_visible);
                let edit = world.dialog_text_edit_state_for_input(input_layout.index);
                let focused = world.dialog_focused_input() == Some(input_layout.index);
                push_full_nine_slice_layers(
                    &mut input_layers,
                    box_layout,
                    if focused {
                        HudInventoryBackgroundTexture::WidgetTextFieldHighlighted
                    } else {
                        HudInventoryBackgroundTexture::WidgetTextField
                    },
                    WIDGET_SPRITE_WIDTH,
                    20,
                    1,
                );
                let text_len = text_edit::char_len(value);
                text_areas.push(HudInventoryTextArea {
                    x: box_layout.x,
                    y: box_layout.y,
                    width: u32::try_from(box_layout.width).unwrap_or_default(),
                    height: u32::try_from(box_layout.height).unwrap_or_default(),
                    text: value.clone(),
                    cursor: edit.map_or(text_len, |edit| edit.cursor),
                    selection: edit.map_or(text_len, |edit| edit.selection),
                    scroll_amount: edit.map_or(i32::MAX, |edit| edit.scroll_amount),
                    max_length: usize::try_from(*max_length).unwrap_or(usize::MAX),
                    cursor_visible: focused && (wall_clock_millis() / 300) % 2 == 0,
                    text_tint: TEXT_INPUT_COLOR,
                    cursor_tint: TEXT_INPUT_COLOR,
                    selection_tint: TEXT_SELECTION_TINT,
                    counter_tint: INACTIVE_TEXT_COLOR,
                    shadow: true,
                });
            }
            _ => {}
        }
    }

    let mut body_action_buttons = Vec::new();
    let mut highlighted_body_action = None;
    if let Some(action_grid) = content.action_grid {
        let grid_x = screen_width_i32 / 2 - action_grid.width / 2;
        let grid_y = content_y
            .saturating_sub(scroll_layout.scroll_amount)
            .saturating_add(content.height.saturating_sub(action_grid.height));
        for (index, (action, button)) in controls
            .body_actions()
            .iter()
            .zip(action_grid.buttons)
            .enumerate()
        {
            let button = DialogButtonLayout {
                x: grid_x.saturating_add(button.x),
                y: grid_y.saturating_add(button.y),
                width: button.width,
            };
            let hovered = dialog_body_viewport_contains(
                scroll_layout,
                cursor_position,
                surface_size,
                layout.screen_width,
                layout.screen_height,
            ) && dialog_widget_contains(
                button.x,
                button.y,
                i32::try_from(button.width).unwrap_or(i32::MAX),
                i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                cursor_position,
                surface_size,
                layout.screen_width,
                layout.screen_height,
            );
            if hovered {
                highlighted_body_action = Some(index);
            }
            push_horizontal_nine_slice_layers(
                &mut input_layers,
                DialogInputLayout {
                    index,
                    x: button.x,
                    y: button.y,
                    width: i32::try_from(button.width).unwrap_or(i32::MAX),
                    height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                },
                if hovered || focused_action == Some(index) {
                    HudInventoryBackgroundTexture::WidgetButtonHighlighted
                } else {
                    HudInventoryBackgroundTexture::WidgetButton
                },
                WIDGET_SPRITE_WIDTH,
                3,
            );
            push_centered_dialog_control_label(
                &mut text_labels,
                *action,
                button.x + i32::try_from(button.width).unwrap_or(i32::MAX) / 2,
                button.y + BUTTON_TEXT_Y_OFFSET,
            );
            body_action_buttons.push(button);
        }
    }

    let scroll_text_labels = text_labels.split_off(scroll_text_start);

    for (action, button) in controls.footer_actions().iter().zip(&layout.buttons) {
        push_centered_dialog_control_label(
            &mut text_labels,
            *action,
            button.x + i32::try_from(button.width).unwrap_or(i32::MAX) / 2,
            button.y + BUTTON_TEXT_Y_OFFSET,
        );
    }

    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    background_layers.push(HudInventoryBackgroundLayer {
        texture: if warning_button_hovered || warning_button_focused {
            HudInventoryBackgroundTexture::DialogWarningButtonHighlighted
        } else {
            HudInventoryBackgroundTexture::DialogWarningButton
        },
        x: warning_button.x,
        y: warning_button.y,
        width: u32::try_from(WARNING_BUTTON_SIZE).unwrap_or_default(),
        height: u32::try_from(WARNING_BUTTON_SIZE).unwrap_or_default(),
        uv: HudUvRect {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        },
    });
    background_layers.extend(layout.buttons.iter().enumerate().map(|(index, button)| {
        let action_index = controls.body_action_count.saturating_add(index);
        HudInventoryBackgroundLayer {
            texture: if highlighted_footer_button == Some(action_index)
                || focused_action == Some(action_index)
            {
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
    let highlighted_action = highlighted_body_action.or(highlighted_footer_button);
    let action_tooltip = highlighted_action.or(focused_action).and_then(|index| {
        let tooltip = controls.actions.get(index)?.tooltip()?;
        let button = if index < controls.body_action_count {
            body_action_buttons.get(index)?
        } else {
            layout
                .buttons
                .get(index.saturating_sub(controls.body_action_count))?
        };
        let (cursor_x, cursor_y) = cursor_screen_position.unwrap_or_else(|| {
            let (origin_x, origin_y) =
                centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
            (
                i32::try_from(origin_x + i64::from(button.x)).unwrap_or(i32::MAX),
                i32::try_from(origin_y + i64::from(button.y) + i64::from(BUTTON_HEIGHT))
                    .unwrap_or(i32::MAX),
            )
        });
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
            style: None,
            lines: wrapped_text_lines(tooltip, 170)
                .into_iter()
                .map(|line| HudInventoryTooltipLine {
                    text: line.text,
                    tint: TEXT_COLOR,
                    runs: line
                        .runs
                        .into_iter()
                        .map(HudComponentTextRun::from)
                        .collect(),
                })
                .collect(),
        })
    });
    let warning_tooltip = (warning_button_hovered || warning_button_focused).then(|| {
        let (cursor_x, cursor_y) = cursor_screen_position.unwrap_or_else(|| {
            let (origin_x, origin_y) =
                centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
            (
                i32::try_from(origin_x + i64::from(warning_button.x)).unwrap_or(i32::MAX),
                i32::try_from(
                    origin_y + i64::from(warning_button.y) + i64::from(WARNING_BUTTON_SIZE),
                )
                .unwrap_or(i32::MAX),
            )
        });
        let (origin_x, origin_y) =
            centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
        HudInventoryTooltip {
            slot_id: u16::MAX,
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
            style: None,
            lines: plain_dialog_text_lines(DIALOG_WARNING_TOOLTIP, 170)
                .into_iter()
                .map(|line| HudInventoryTooltipLine {
                    text: line.text,
                    tint: TEXT_COLOR,
                    runs: line
                        .runs
                        .into_iter()
                        .map(HudComponentTextRun::from)
                        .collect(),
                })
                .collect(),
        }
    });
    let preedit_overlay = local_state.dialog_ime_preedit.as_ref().and_then(|preedit| {
        dialog_preedit_projection(world, preedit, surface_size, text_measurer)
            .map(|projection| projection.overlay)
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
        scroll_area: Some(HudInventoryScrollArea {
            scissor: HudInventoryItemScissor {
                x: scroll_layout.rectangle.x,
                y: scroll_layout.rectangle.y,
                width: u32::try_from(scroll_layout.rectangle.width).unwrap_or_default(),
                height: u32::try_from(scroll_layout.rectangle.height).unwrap_or_default(),
            },
            background_layers: input_layers,
            fill_layers: scroll_fill_layers,
            text_labels: scroll_text_labels,
            text_areas,
            scrollbar: dialog_scrollbar_layout(scroll_layout).map(|scrollbar| {
                HudInventoryScrollbar {
                    x: scrollbar.x,
                    y: scrollbar.y,
                    width: u32::try_from(scrollbar.width).unwrap_or_default(),
                    height: u32::try_from(scrollbar.height).unwrap_or_default(),
                    thumb_y: scrollbar.thumb_y,
                    thumb_height: u32::try_from(scrollbar.thumb_height).unwrap_or_default(),
                }
            }),
        }),
        preedit_overlay,
        hovered_slot_id: None,
        tooltip: warning_tooltip.or(action_tooltip).or(item_tooltip),
    })
}

struct DialogPreeditProjection {
    overlay: HudInventoryPreeditOverlay,
    ime_position: PhysicalPosition<i32>,
    ime_size: PhysicalSize<u32>,
}

pub(crate) fn dialog_ime_cursor_area(
    world: &WorldStore,
    preedit: Option<&DialogImePreeditHudState>,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    let projection = dialog_preedit_projection(world, preedit?, surface_size, text_measurer)?;
    Some((projection.ime_position, projection.ime_size))
}

fn dialog_preedit_projection(
    world: &WorldStore,
    preedit: &DialogImePreeditHudState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> Option<DialogPreeditProjection> {
    if preedit.screen_revision != world.dialog_screen_revision()
        || world.dialog_link_confirmation().is_some()
        || world.dialog_warning_is_open()
        || preedit.text.is_empty()
    {
        return None;
    }
    let snapshot = focused_dialog_text_snapshot(world)?;
    let dialog = world.current_dialog()?;
    let controls = dialog_controls(dialog.data.as_ref()?, world.server_links())?;
    let scroll_layout =
        dialog_body_scroll_layout(&controls, surface_size, world.dialog_body_scroll_amount())?;
    let input_layout =
        dialog_input_layouts_at_scroll(&controls, surface_size, scroll_layout.scroll_amount)?
            .into_iter()
            .find(|layout| layout.index == snapshot.index)?;
    let input = controls.common.input_data.get(snapshot.index)?;
    let (input_left, input_top) = match &input.control {
        DialogInputControlState::Text {
            label_visible,
            multiline: None,
            ..
        } => {
            let layout = dialog_single_line_text_box_layout(input_layout, *label_visible);
            let inner_width = snapshot.inner_width();
            let scroll_to = if snapshot.selection != snapshot.cursor {
                snapshot.selection
            } else {
                snapshot.cursor
            };
            let display_start =
                text_measurer.display_start(&snapshot.value, scroll_to, inner_width);
            let display_end =
                text_measurer.cursor_for_width(&snapshot.value, display_start, inner_width);
            let prefix = dialog_char_slice(
                &snapshot.value,
                display_start,
                snapshot.cursor.min(display_end),
            );
            let append_offset = usize::from(
                snapshot.cursor >= text_edit::char_len(&snapshot.value)
                    && snapshot.value.encode_utf16().count() < snapshot.max_length,
            );
            (
                layout
                    .x
                    .saturating_add(TEXT_INPUT_INSET)
                    .saturating_add(i32::try_from(text_measurer.width(&prefix)).unwrap_or(i32::MAX))
                    .saturating_add(i32::try_from(append_offset).unwrap_or(0)),
                layout.y.saturating_add(TEXT_INPUT_Y_OFFSET),
            )
        }
        DialogInputControlState::Text {
            label_visible,
            multiline: Some(_),
            ..
        } => {
            let layout = dialog_multiline_text_box_layout(input_layout, *label_visible);
            let lines =
                dialog_multiline_lines(&snapshot.value, snapshot.inner_width(), text_measurer);
            let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
            let scroll_amount = snapshot.scroll_amount.clamp(0, max_scroll);
            let cursor_line = lines
                .iter()
                .position(|line| snapshot.cursor <= line.end)
                .unwrap_or_else(|| lines.len().saturating_sub(1));
            let line = lines[cursor_line];
            let prefix =
                dialog_char_slice(&snapshot.value, line.begin, snapshot.cursor.min(line.end));
            (
                layout.x.saturating_add(TEXT_INPUT_INSET).saturating_add(
                    i32::try_from(text_measurer.width(&prefix)).unwrap_or(i32::MAX),
                ),
                layout
                    .y
                    .saturating_add(TEXT_INPUT_INSET)
                    .saturating_sub(scroll_amount)
                    .saturating_add(
                        i32::try_from(cursor_line)
                            .unwrap_or(i32::MAX)
                            .saturating_mul(TEXT_LINE_HEIGHT),
                    ),
            )
        }
        _ => return None,
    };

    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let input_left = i32::try_from(origin_x + i64::from(input_left)).unwrap_or_else(|_| {
        if origin_x + i64::from(input_left) < 0 {
            i32::MIN
        } else {
            i32::MAX
        }
    });
    let input_top = i32::try_from(origin_y + i64::from(input_top)).unwrap_or_else(|_| {
        if origin_y + i64::from(input_top) < 0 {
            i32::MIN
        } else {
            i32::MAX
        }
    });
    let preedit_width = i32::try_from(text_measurer.width(&preedit.text)).unwrap_or(i32::MAX);
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let mut preedit_left = input_left;
    let mut preedit_right = preedit_left.saturating_add(preedit_width);
    if preedit_right > surface_width {
        preedit_left = surface_width.saturating_sub(preedit_width);
        preedit_right = preedit_left.saturating_add(preedit_width);
    }
    let input_bottom = input_top.saturating_add(PREEDIT_INPUT_HEIGHT);
    let mut preedit_bottom = input_bottom
        .saturating_add(PREEDIT_INPUT_SEPARATION)
        .saturating_add(PREEDIT_TEXT_HEIGHT);
    if preedit_bottom > surface_height {
        preedit_bottom = input_top
            .saturating_sub(PREEDIT_INPUT_SEPARATION)
            .saturating_sub(PREEDIT_TEXT_HEIGHT);
    }
    let preedit_top = preedit_bottom.saturating_sub(PREEDIT_TEXT_HEIGHT);
    let hot_left = preedit_left
        .min(input_left)
        .saturating_sub(PREEDIT_HOT_AREA_MARGIN);
    let hot_top = preedit_top
        .min(input_top)
        .saturating_sub(PREEDIT_HOT_AREA_MARGIN);
    let hot_right = preedit_right.saturating_add(PREEDIT_HOT_AREA_MARGIN);
    let hot_bottom = preedit_bottom
        .max(input_bottom)
        .saturating_add(PREEDIT_HOT_AREA_MARGIN);

    Some(DialogPreeditProjection {
        overlay: HudInventoryPreeditOverlay {
            text: preedit.text.clone(),
            focused_start: preedit.focused_start,
            focused_end: preedit.focused_end,
            caret: preedit.caret,
            caret_visible: preedit.caret_visible,
            text_x: preedit_left,
            text_y: preedit_top,
            background_x: preedit_left.saturating_sub(PREEDIT_BACKGROUND_OFFSET),
            background_y: preedit_top.saturating_sub(PREEDIT_BACKGROUND_OFFSET),
            background_width: u32::try_from(
                preedit_width.saturating_add(PREEDIT_BACKGROUND_OFFSET.saturating_mul(2)),
            )
            .unwrap_or(u32::MAX),
            background_height: u32::try_from(
                PREEDIT_TEXT_HEIGHT.saturating_add(PREEDIT_BACKGROUND_OFFSET.saturating_mul(2)),
            )
            .unwrap_or(u32::MAX),
        },
        ime_position: PhysicalPosition::new(hot_left, hot_top),
        ime_size: PhysicalSize::new(
            u32::try_from(hot_right.saturating_sub(hot_left).max(1)).unwrap_or(u32::MAX),
            u32::try_from(hot_bottom.saturating_sub(hot_top).max(1)).unwrap_or(u32::MAX),
        ),
    })
}

pub(super) fn hud_dialog_confirm_link_screen(
    confirmation: &DialogLinkConfirmationState,
    focused_button: Option<usize>,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> HudInventoryScreen {
    let message_width = surface_size
        .width
        .clamp(1, 512)
        .saturating_sub(CONFIRM_MESSAGE_HORIZONTAL_PADDING)
        .max(1);
    let message_lines = plain_dialog_text_lines(&confirmation.url, message_width)
        .into_iter()
        .take(CONFIRM_MESSAGE_MAX_ROWS)
        .collect::<Vec<_>>();
    let layout = DialogConfirmLinkLayout::new(message_lines.len(), surface_size);
    let cursor = cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted = layout.button_at(cursor, surface_size);
    let screen_center_x = i32::try_from(layout.screen_width).unwrap_or(i32::MAX) / 2;
    let mut text_labels = Vec::new();
    push_centered_plain_text_label(
        &mut text_labels,
        CONFIRM_LINK_TITLE,
        screen_center_x,
        layout.title_y,
        TEXT_COLOR,
    );
    let mut message_y = layout.message_y;
    for line in message_lines {
        push_centered_runs_text_label(
            &mut text_labels,
            line.runs,
            screen_center_x,
            message_y,
            None,
        );
        message_y = message_y.saturating_add(TEXT_LINE_HEIGHT);
    }
    push_centered_plain_text_label(
        &mut text_labels,
        CONFIRM_LINK_WARNING,
        screen_center_x,
        layout.warning_y,
        CONFIRM_LINK_WARNING_COLOR,
    );
    for (button, label) in
        layout
            .buttons
            .into_iter()
            .zip([CONFIRM_LINK_YES, CONFIRM_LINK_COPY, CONFIRM_LINK_NO])
    {
        push_centered_plain_text_label(
            &mut text_labels,
            label,
            button
                .x
                .saturating_add(i32::try_from(button.width).unwrap_or(i32::MAX) / 2),
            button.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
            TEXT_COLOR,
        );
    }
    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    background_layers.extend(
        layout
            .buttons
            .into_iter()
            .enumerate()
            .map(|(index, button)| HudInventoryBackgroundLayer {
                texture: if highlighted == Some(index) || focused_button == Some(index) {
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
            }),
    );
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
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}

pub(super) fn hud_dialog_warning_screen(
    focused_button: Option<bool>,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> HudInventoryScreen {
    let message_lines = dialog_warning_message_lines(surface_size);
    let layout = DialogWarningScreenLayout::new(message_lines.len(), surface_size);
    let cursor = cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted = layout.button_at(cursor, surface_size);
    let screen_center_x = i32::try_from(layout.screen_width).unwrap_or(i32::MAX) / 2;
    let mut text_labels = Vec::new();
    push_centered_plain_text_label(
        &mut text_labels,
        DIALOG_WARNING_TITLE,
        screen_center_x,
        layout.title_y,
        TEXT_COLOR,
    );
    let mut message_y = layout.message_y;
    for line in message_lines {
        push_centered_runs_text_label(
            &mut text_labels,
            line.runs,
            screen_center_x,
            message_y,
            None,
        );
        message_y = message_y.saturating_add(TEXT_LINE_HEIGHT);
    }
    for (button, label) in [
        (layout.yes, DIALOG_WARNING_DISCONNECT),
        (layout.no, DIALOG_WARNING_BACK),
    ] {
        push_centered_plain_text_label(
            &mut text_labels,
            label,
            button
                .x
                .saturating_add(i32::try_from(button.width).unwrap_or(i32::MAX) / 2),
            button.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
            TEXT_COLOR,
        );
    }
    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    background_layers.extend([(layout.yes, true), (layout.no, false)].into_iter().map(
        |(button, result)| HudInventoryBackgroundLayer {
            texture: if highlighted == Some(result) || focused_button == Some(result) {
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
        },
    ));
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
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
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
        scroll_area: None,
        preedit_overlay: None,
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

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_key_input_with_modifiers(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
    shift_down: bool,
    control_down: bool,
) -> bool {
    handle_dialog_key_input_with_modifiers_for_surface(
        counters,
        world,
        net_commands,
        clipboard,
        physical_key,
        state,
        shift_down,
        control_down,
        PhysicalSize::new(320, 240),
    )
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn handle_dialog_key_input_with_modifiers_for_surface(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
    shift_down: bool,
    control_down: bool,
    surface_size: PhysicalSize<u32>,
) -> bool {
    handle_dialog_key_input_with_modifiers_for_surface_and_measurer(
        counters,
        world,
        net_commands,
        clipboard,
        physical_key,
        state,
        shift_down,
        control_down,
        surface_size,
        &ApproximateDialogTextMeasurer,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_key_input_with_modifiers_for_surface_and_measurer(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    mut clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
    shift_down: bool,
    control_down: bool,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if world.dialog_link_confirmation().is_some() {
        if state != ElementState::Pressed {
            return true;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        match code {
            KeyCode::Escape => {
                world.resolve_dialog_link_confirmation(false);
            }
            KeyCode::Tab => {
                let current = world.dialog_link_focused_button();
                let next = if shift_down {
                    current.map_or(2, |index| index.checked_sub(1).unwrap_or(2))
                } else {
                    current.map_or(0, |index| index.saturating_add(1) % 3)
                };
                world.focus_dialog_link_button(next);
            }
            KeyCode::ArrowLeft => {
                let next = world
                    .dialog_link_focused_button()
                    .map_or(2, |index| index.saturating_sub(1));
                world.focus_dialog_link_button(next);
            }
            KeyCode::ArrowRight => {
                let next = world
                    .dialog_link_focused_button()
                    .map_or(0, |index| index.saturating_add(1).min(2));
                world.focus_dialog_link_button(next);
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                if world.dialog_link_focused_button().is_none() {
                    world.focus_dialog_link_button(0);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(index) = world.dialog_link_focused_button() {
                    run_dialog_confirm_link_button(world, &mut clipboard, index);
                }
            }
            _ => {}
        }
        return true;
    }
    if world.dialog_warning_is_open() {
        if state != ElementState::Pressed {
            return true;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        match code {
            KeyCode::Escape => {
                world.close_dialog_warning();
            }
            KeyCode::Tab => {
                let current = world.dialog_warning_focused_button();
                let next = if shift_down {
                    match current {
                        None | Some(true) => false,
                        Some(false) => true,
                    }
                } else {
                    match current {
                        None | Some(false) => true,
                        Some(true) => false,
                    }
                };
                world.focus_dialog_warning_button(next);
            }
            KeyCode::ArrowLeft => match world.dialog_warning_focused_button() {
                None => {
                    world.focus_dialog_warning_button(false);
                }
                Some(false) => {
                    world.focus_dialog_warning_button(true);
                }
                Some(true) => {}
            },
            KeyCode::ArrowRight => match world.dialog_warning_focused_button() {
                None => {
                    world.focus_dialog_warning_button(true);
                }
                Some(true) => {
                    world.focus_dialog_warning_button(false);
                }
                Some(false) => {}
            },
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                if world.dialog_warning_focused_button().is_none() {
                    world.focus_dialog_warning_button(true);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(disconnect) = world.dialog_warning_focused_button() {
                    run_dialog_warning_button(world, net_commands, disconnect);
                }
            }
            _ => {}
        }
        return true;
    }
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
        text_measurer,
    ) {
        return true;
    }
    if handle_focused_dialog_input_key(world, physical_key, shift_down) {
        return true;
    }
    let PhysicalKey::Code(code) = physical_key else {
        return true;
    };
    if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
        match world.dialog_focus() {
            Some(DialogFocusState::WarningButton) => {
                world.open_dialog_warning();
                return true;
            }
            Some(DialogFocusState::Action { index }) => {
                let action = world
                    .current_dialog()
                    .and_then(|dialog| dialog.data.as_ref())
                    .and_then(|data| dialog_controls(data, world.server_links()))
                    .and_then(|controls| {
                        Some((
                            controls.actions.get(index)?.activation(),
                            controls.common.after_action.clone(),
                        ))
                    });
                if let Some((activation, after_action)) = action {
                    run_dialog_control_activation(
                        counters,
                        world,
                        net_commands,
                        clipboard,
                        activation,
                        &after_action,
                    );
                }
                return true;
            }
            _ => {}
        }
    }
    let direction = match code {
        KeyCode::ArrowUp => Some(DialogFocusDirection::Up),
        KeyCode::ArrowDown => Some(DialogFocusDirection::Down),
        KeyCode::ArrowLeft => Some(DialogFocusDirection::Left),
        KeyCode::ArrowRight => Some(DialogFocusDirection::Right),
        _ => None,
    };
    if code == KeyCode::Tab || direction.is_some() {
        let scroll_amount = world.dialog_body_scroll_amount();
        let next = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let layout = dialog_body_scroll_layout(&controls, surface_size, scroll_amount)?;
                dialog_focus_graph_at_scroll(&controls, surface_size, layout.scroll_amount)
            })
            .and_then(|graph| {
                if code == KeyCode::Tab {
                    next_dialog_tab_focus(&graph, world.dialog_focus(), !shift_down)
                } else {
                    next_dialog_focus_in_direction(
                        &graph,
                        world.dialog_focus(),
                        direction.expect("direction checked above"),
                    )
                }
            });
        if let Some(target) = next {
            world.focus_dialog_navigation_target(target, code == KeyCode::Tab);
            reveal_dialog_keyboard_focus(world, target, surface_size);
        }
        return true;
    }
    if physical_key != PhysicalKey::Code(KeyCode::Escape) {
        return true;
    }
    let Some(controls) = world
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
    else {
        return true;
    };
    if !controls.common.can_close_with_escape {
        return true;
    }
    let action = controls
        .cancel
        .and_then(|cancel| cancel.action_data.clone());
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

fn reveal_dialog_keyboard_focus(
    world: &mut WorldStore,
    target: DialogFocusState,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let scroll_amount = world.dialog_body_scroll_amount();
    let Some((layout, focused)) = world
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .and_then(|controls| {
            let layout = dialog_body_scroll_layout(&controls, surface_size, scroll_amount)?;
            let graph =
                dialog_focus_graph_at_scroll(&controls, surface_size, layout.scroll_amount)?;
            let focused = dialog_focus_node(&graph.body, target)?.rectangle;
            Some((layout, focused))
        })
    else {
        return false;
    };
    let top_delta = focused.y.saturating_sub(layout.rectangle.y);
    let bottom_delta = focused
        .y
        .saturating_add(focused.height)
        .saturating_sub(layout.rectangle.y.saturating_add(layout.rectangle.height));
    let next = if top_delta < 0 {
        scroll_amount + f64::from(top_delta) - 10.0
    } else if bottom_delta > 0 {
        scroll_amount + f64::from(bottom_delta) + 10.0
    } else {
        return false;
    };
    world.set_dialog_body_scroll_amount(next, f64::from(layout.max_scroll))
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
        .and_then(|data| dialog_controls(data, world.server_links()))
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
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let PhysicalKey::Code(code) = physical_key else {
        return false;
    };
    if world.dialog_text_edit_state().is_none() {
        return false;
    }
    match code {
        KeyCode::Backspace => {
            delete_focused_dialog_text(world, -1, control_down, text_measurer);
            true
        }
        KeyCode::Delete => {
            delete_focused_dialog_text(world, 1, control_down, text_measurer);
            true
        }
        KeyCode::ArrowLeft => {
            move_focused_dialog_text_cursor(world, -1, shift_down, control_down, text_measurer);
            true
        }
        KeyCode::ArrowRight => {
            move_focused_dialog_text_cursor(world, 1, shift_down, control_down, text_measurer);
            true
        }
        KeyCode::ArrowUp | KeyCode::ArrowDown => {
            if focused_dialog_text_snapshot(world)
                .is_some_and(|snapshot| snapshot.multiline.is_some())
            {
                move_focused_dialog_multiline_cursor_vertically(
                    world,
                    code == KeyCode::ArrowDown,
                    shift_down,
                    control_down,
                    text_measurer,
                );
                true
            } else {
                false
            }
        }
        KeyCode::PageUp | KeyCode::PageDown => {
            let Some(snapshot) = focused_dialog_text_snapshot(world) else {
                return false;
            };
            if snapshot.multiline.is_some() {
                let cursor = if code == KeyCode::PageUp {
                    0
                } else {
                    text_edit::char_len(&snapshot.value)
                };
                move_focused_dialog_text_cursor_to(world, cursor, shift_down, text_measurer);
                true
            } else {
                false
            }
        }
        KeyCode::Home => {
            let cursor = focused_dialog_text_snapshot(world).map_or(0, |snapshot| {
                if snapshot.multiline.is_some() && !control_down {
                    let lines = dialog_multiline_lines(
                        &snapshot.value,
                        snapshot.inner_width(),
                        text_measurer,
                    );
                    lines[dialog_multiline_cursor_line(&lines, snapshot.cursor)].begin
                } else {
                    0
                }
            });
            move_focused_dialog_text_cursor_to(world, cursor, shift_down, text_measurer);
            true
        }
        KeyCode::End => {
            let end = focused_dialog_text_snapshot(world).map_or(0, |snapshot| {
                if snapshot.multiline.is_some() && !control_down {
                    let lines = dialog_multiline_lines(
                        &snapshot.value,
                        snapshot.inner_width(),
                        text_measurer,
                    );
                    lines[dialog_multiline_cursor_line(&lines, snapshot.cursor)].end
                } else {
                    text_edit::char_len(&snapshot.value)
                }
            });
            move_focused_dialog_text_cursor_to(world, end, shift_down, text_measurer);
            true
        }
        KeyCode::Enter | KeyCode::NumpadEnter => {
            if focused_dialog_text_snapshot(world)
                .is_some_and(|snapshot| snapshot.multiline.is_some())
            {
                replace_focused_dialog_text_selection(world, "\n", text_measurer);
                true
            } else {
                false
            }
        }
        KeyCode::KeyA if control_down => {
            if let Some(snapshot) = focused_dialog_text_snapshot(world) {
                let end = text_edit::char_len(&snapshot.value);
                set_focused_dialog_text(
                    world,
                    &snapshot,
                    snapshot.value.clone(),
                    end,
                    0,
                    text_measurer,
                );
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
                replace_focused_dialog_text_selection(world, &text, text_measurer);
            }
            true
        }
        KeyCode::KeyX if control_down => {
            copy_focused_dialog_text_selection(world, clipboard);
            replace_focused_dialog_text_selection(world, "", text_measurer);
            true
        }
        _ => false,
    }
}

#[derive(Debug, Clone)]
struct FocusedDialogTextSnapshot {
    index: usize,
    value: String,
    cursor: usize,
    selection: usize,
    scroll_amount: i32,
    width: i32,
    height: i32,
    max_length: usize,
    multiline: Option<DialogTextMultilineState>,
}

impl FocusedDialogTextSnapshot {
    fn inner_width(&self) -> u32 {
        u32::try_from(
            self.width
                .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                .max(1),
        )
        .unwrap_or(1)
    }
}

fn focused_dialog_text_snapshot(world: &WorldStore) -> Option<FocusedDialogTextSnapshot> {
    let edit = world.dialog_text_edit_state()?;
    let Some(Some(DialogInputValueState::Text { value })) =
        world.current_dialog_input_values().get(edit.input_index)
    else {
        return None;
    };
    let (width, max_length, multiline) = world
        .current_dialog()?
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))?
        .common
        .input_data
        .get(edit.input_index)
        .and_then(|input| match &input.control {
            DialogInputControlState::Text {
                width,
                max_length,
                multiline,
                ..
            } => Some((
                *width,
                usize::try_from(*max_length).ok()?,
                multiline.clone(),
            )),
            _ => None,
        })?;
    let height = multiline.as_ref().map_or(TEXT_INPUT_HEIGHT, |multiline| {
        multiline.height.unwrap_or_else(|| {
            multiline
                .max_lines
                .unwrap_or(4)
                .saturating_mul(TEXT_LINE_HEIGHT)
                .saturating_add(8)
                .min(512)
        })
    });
    Some(FocusedDialogTextSnapshot {
        index: edit.input_index,
        value: value.clone(),
        cursor: edit.cursor,
        selection: edit.selection,
        scroll_amount: edit.scroll_amount,
        width,
        height,
        max_length,
        multiline,
    })
}

fn set_focused_dialog_text(
    world: &mut WorldStore,
    snapshot: &FocusedDialogTextSnapshot,
    value: String,
    cursor: usize,
    selection: usize,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let lines = snapshot
        .multiline
        .as_ref()
        .map(|_| dialog_multiline_lines(&value, snapshot.inner_width(), text_measurer));
    if snapshot
        .multiline
        .as_ref()
        .and_then(|multiline| multiline.max_lines)
        .is_some_and(|max_lines| {
            lines
                .as_ref()
                .is_some_and(|lines| i32::try_from(lines.len()).unwrap_or(i32::MAX) > max_lines)
        })
    {
        return false;
    }
    if !world.set_dialog_text_input_value_and_selection(snapshot.index, value, cursor, selection) {
        return false;
    }
    if let Some(lines) = lines {
        let max_scroll = dialog_multiline_max_scroll(lines.len(), snapshot.height);
        let scroll_amount =
            dialog_multiline_reveal_cursor(snapshot.scroll_amount, snapshot.height, &lines, cursor);
        world.set_dialog_multiline_text_scroll_amount(snapshot.index, scroll_amount, max_scroll);
    }
    true
}

fn move_focused_dialog_text_cursor(
    world: &mut WorldStore,
    direction: i32,
    extend_selection: bool,
    whole_word: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(snapshot) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let next = if whole_word {
        if snapshot.multiline.is_some() {
            dialog_multiline_word_position(&snapshot.value, snapshot.cursor, direction)
        } else {
            text_edit::word_position(&snapshot.value, snapshot.cursor, direction)
        }
    } else if direction < 0 {
        snapshot.cursor.saturating_sub(1)
    } else {
        snapshot
            .cursor
            .saturating_add(1)
            .min(text_edit::char_len(&snapshot.value))
    };
    set_focused_dialog_text(
        world,
        &snapshot,
        snapshot.value.clone(),
        next,
        if extend_selection {
            snapshot.selection
        } else {
            next
        },
        text_measurer,
    )
}

fn move_focused_dialog_text_cursor_to(
    world: &mut WorldStore,
    cursor: usize,
    extend_selection: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(snapshot) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    set_focused_dialog_text(
        world,
        &snapshot,
        snapshot.value.clone(),
        cursor,
        if extend_selection {
            snapshot.selection
        } else {
            cursor
        },
        text_measurer,
    )
}

fn delete_focused_dialog_text(
    world: &mut WorldStore,
    direction: i32,
    whole_word: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(snapshot) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let mut value = snapshot.value.clone();
    let cursor = snapshot.cursor;
    let selection = snapshot.selection;
    let (start, end) = if cursor != selection {
        (cursor.min(selection), cursor.max(selection))
    } else {
        let target = if whole_word {
            if snapshot.multiline.is_some() {
                dialog_multiline_word_position(&value, cursor, direction)
            } else {
                text_edit::word_position(&value, cursor, direction)
            }
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
    set_focused_dialog_text(world, &snapshot, value, start, start, text_measurer)
}

fn replace_focused_dialog_text_selection(
    world: &mut WorldStore,
    input: &str,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(snapshot) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let mut value = snapshot.value.clone();
    let cursor = snapshot.cursor;
    let selection = snapshot.selection;
    let start = cursor.min(selection);
    let end = cursor.max(selection);
    let start_byte = text_edit::byte_index(&value, start);
    let end_byte = text_edit::byte_index(&value, end);
    let selected_utf16 = value[start_byte..end_byte].encode_utf16().count();
    let retained_utf16 = value.encode_utf16().count().saturating_sub(selected_utf16);
    let mut remaining = snapshot.max_length.saturating_sub(retained_utf16);
    let mut replacement = String::new();
    for ch in input
        .chars()
        .filter(|ch| dialog_text_character_allowed(*ch, snapshot.multiline.is_some()))
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
    set_focused_dialog_text(world, &snapshot, value, cursor, cursor, text_measurer)
}

fn dialog_text_character_allowed(ch: char, multiline: bool) -> bool {
    (multiline && ch == '\n') || (ch != '\u{00a7}' && ch >= ' ' && ch != '\u{007f}')
}

fn dialog_multiline_word_position(text: &str, cursor: usize, direction: i32) -> usize {
    let chars = text.chars().collect::<Vec<_>>();
    let mut cursor = cursor.min(chars.len());
    if direction < 0 {
        while cursor > 0 && chars[cursor - 1].is_whitespace() {
            cursor -= 1;
        }
        while cursor > 0 && !chars[cursor - 1].is_whitespace() {
            cursor -= 1;
        }
    } else {
        while cursor < chars.len() && !chars[cursor].is_whitespace() {
            cursor += 1;
        }
        while cursor < chars.len() && chars[cursor].is_whitespace() {
            cursor += 1;
        }
    }
    cursor
}

fn move_focused_dialog_multiline_cursor_vertically(
    world: &mut WorldStore,
    down: bool,
    extend_selection: bool,
    control_down: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(snapshot) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    if snapshot.multiline.is_none() {
        return false;
    }
    let lines = dialog_multiline_lines(&snapshot.value, snapshot.inner_width(), text_measurer);
    let max_scroll = dialog_multiline_max_scroll(lines.len(), snapshot.height);
    let scroll_amount = snapshot.scroll_amount.clamp(0, max_scroll);
    let scrolled = if down {
        scroll_amount.saturating_add(4).min(max_scroll)
    } else {
        scroll_amount.saturating_sub(4)
    };
    if scrolled != scroll_amount {
        world.set_dialog_multiline_text_scroll_amount(snapshot.index, scrolled, max_scroll);
        return true;
    }
    if control_down {
        return true;
    }

    let line_index = dialog_multiline_cursor_line(&lines, snapshot.cursor);
    let target_index = if down {
        line_index
            .saturating_add(1)
            .min(lines.len().saturating_sub(1))
    } else {
        line_index.saturating_sub(1)
    };
    if target_index == line_index {
        return true;
    }
    let line = lines[line_index];
    let prefix = dialog_char_slice(&snapshot.value, line.begin, snapshot.cursor.min(line.end));
    let target = lines[target_index];
    let target_text = dialog_char_slice(&snapshot.value, target.begin, target.end);
    let target_cursor = target.begin.saturating_add(
        text_measurer
            .cursor_for_width(
                &target_text,
                0,
                text_measurer.width(&prefix).saturating_add(2),
            )
            .min(target.end.saturating_sub(target.begin)),
    );
    set_focused_dialog_text(
        world,
        &snapshot,
        snapshot.value.clone(),
        target_cursor,
        if extend_selection {
            snapshot.selection
        } else {
            target_cursor
        },
        text_measurer,
    )
}

fn dialog_char_slice(text: &str, begin: usize, end: usize) -> String {
    text.chars()
        .skip(begin)
        .take(end.saturating_sub(begin))
        .collect()
}

fn copy_focused_dialog_text_selection(
    world: &WorldStore,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
) -> bool {
    let Some(snapshot) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let value = snapshot.value;
    let cursor = snapshot.cursor;
    let selection = snapshot.selection;
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

#[cfg(test)]
pub(crate) fn handle_dialog_text_input(world: &mut WorldStore, text: &str) -> bool {
    handle_dialog_text_input_with_measurer(world, text, &ApproximateDialogTextMeasurer)
}

pub(crate) fn handle_dialog_text_input_with_measurer(
    world: &mut WorldStore,
    text: &str,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if world.dialog_link_confirmation().is_some() || world.dialog_warning_is_open() {
        return true;
    }
    if world.dialog_text_edit_state().is_none() {
        return world.dialog_screen_is_open();
    }
    replace_focused_dialog_text_selection(world, text, text_measurer);
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
        false,
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
    double_click: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if button == MouseButton::Left && state == ElementState::Released {
        world.end_dialog_number_range_drag();
        world.end_dialog_text_input_drag();
        world.end_dialog_multiline_scroll_drag();
        world.end_dialog_body_scroll_drag();
        return world.dialog_screen_is_open();
    }
    if let Some(confirmation) = world.dialog_link_confirmation() {
        if button == MouseButton::Left && state == ElementState::Pressed {
            let message_width = surface_size
                .width
                .clamp(1, 512)
                .saturating_sub(CONFIRM_MESSAGE_HORIZONTAL_PADDING)
                .max(1);
            let message_line_count = plain_dialog_text_lines(&confirmation.url, message_width)
                .len()
                .min(CONFIRM_MESSAGE_MAX_ROWS);
            if let Some(index) = DialogConfirmLinkLayout::new(message_line_count, surface_size)
                .button_at(cursor_position, surface_size)
            {
                world.focus_dialog_link_button(index);
                let mut clipboard = clipboard;
                run_dialog_confirm_link_button(world, &mut clipboard, index);
            }
        }
        return true;
    }
    if world.dialog_warning_is_open() {
        if button == MouseButton::Left && state == ElementState::Pressed {
            let message_line_count = dialog_warning_message_lines(surface_size).len();
            match DialogWarningScreenLayout::new(message_line_count, surface_size)
                .button_at(cursor_position, surface_size)
            {
                Some(disconnect) => {
                    world.focus_dialog_warning_button(disconnect);
                    run_dialog_warning_button(world, net_commands, disconnect);
                }
                None => {}
            }
        }
        return true;
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
    world.end_dialog_multiline_scroll_drag();
    world.end_dialog_body_scroll_drag();
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    let Some(controls) = dialog
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))
    else {
        return true;
    };
    if DialogWarningButtonLayout::new(&controls.common.title, surface_size)
        .is_some_and(|layout| layout.contains(cursor_position, surface_size))
    {
        world.focus_dialog_navigation_target(DialogFocusState::WarningButton, false);
        world.open_dialog_warning();
        return true;
    }
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let scroll_layout =
        dialog_body_scroll_layout(&controls, surface_size, world.dialog_body_scroll_amount());
    if let Some(cursor_y) = scroll_layout.and_then(|scroll_layout| {
        let scrollbar = dialog_scrollbar_layout(scroll_layout)?;
        dialog_scrollbar_contains(
            scrollbar,
            cursor_position,
            surface_size,
            screen_width,
            screen_height,
        )
        .then_some(())?;
        dialog_cursor_screen_y(cursor_position, surface_size, screen_height)
    }) {
        world.begin_dialog_body_scroll_drag(cursor_y);
        return true;
    }
    let Some(scroll_layout) = scroll_layout else {
        return true;
    };
    let cursor_in_body = dialog_body_viewport_contains(
        scroll_layout,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    );
    let input_hit =
        dialog_input_layouts_at_scroll(&controls, surface_size, scroll_layout.scroll_amount)
            .and_then(|layouts| {
                layouts.into_iter().find_map(|layout| {
                    let control = &controls.common.input_data.get(layout.index)?.control;
                    let hit_layout = match control {
                        DialogInputControlState::Text {
                            label_visible,
                            multiline: None,
                            ..
                        } => dialog_single_line_text_box_layout(layout, *label_visible),
                        DialogInputControlState::Text {
                            label_visible,
                            multiline: Some(_),
                            ..
                        } => dialog_multiline_text_box_layout(layout, *label_visible),
                        _ => layout,
                    };
                    let hit_width = if matches!(
                        control,
                        DialogInputControlState::Text {
                            multiline: Some(_),
                            ..
                        }
                    ) {
                        hit_layout.width.saturating_add(6)
                    } else {
                        hit_layout.width
                    };
                    if !cursor_in_body
                        || !dialog_widget_contains(
                            hit_layout.x,
                            hit_layout.y,
                            hit_width,
                            hit_layout.height,
                            cursor_position,
                            surface_size,
                            screen_width,
                            screen_height,
                        )
                    {
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
                        } => Some(DialogInputMouseHit::Text {
                            layout: hit_layout,
                            multiline: false,
                        }),
                        DialogInputControlState::Text {
                            multiline: Some(_), ..
                        } => Some(DialogInputMouseHit::Text {
                            layout: hit_layout,
                            multiline: true,
                        }),
                    }
                })
            });
    let body_navigation_hit =
        dialog_focus_graph_at_scroll(&controls, surface_size, scroll_layout.scroll_amount)
            .and_then(|graph| {
                graph.body.into_iter().find_map(|node| {
                    (cursor_in_body
                        && dialog_widget_contains(
                            node.rectangle.x,
                            node.rectangle.y,
                            node.rectangle.width,
                            node.rectangle.height,
                            cursor_position,
                            surface_size,
                            screen_width,
                            screen_height,
                        ))
                    .then_some(node)
                })
            });
    let footer_action = DialogFooterLayout::new(
        &controls
            .footer_actions()
            .iter()
            .map(|action| action.width())
            .collect::<Vec<_>>(),
        controls.footer_height,
        surface_size,
    )
    .and_then(|layout| layout.button_at(cursor_position, surface_size))
    .and_then(|footer_index| {
        let action_index = controls.body_action_count.saturating_add(footer_index);
        Some((
            action_index,
            controls.actions.get(action_index)?.activation(),
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
        Some(DialogInputMouseHit::Text { layout, multiline }) => {
            let scrollbar_drag = multiline
                && begin_dialog_multiline_scroll_drag_from_cursor(
                    world,
                    layout,
                    cursor_position,
                    surface_size,
                    text_measurer,
                );
            if !scrollbar_drag {
                if double_click {
                    if multiline {
                        world.focus_dialog_navigation_target(
                            DialogFocusState::Input {
                                index: layout.index,
                            },
                            false,
                        );
                    } else {
                        focus_dialog_text_from_cursor(
                            world,
                            layout,
                            cursor_position,
                            surface_size,
                            false,
                            text_measurer,
                        );
                    }
                    select_focused_dialog_text_word(world, multiline, text_measurer);
                } else {
                    focus_dialog_text_from_cursor(
                        world,
                        layout,
                        cursor_position,
                        surface_size,
                        shift_down,
                        text_measurer,
                    );
                }
                world.begin_dialog_text_input_drag(layout.index);
            }
            return true;
        }
        None => {}
    }
    if let Some(node) = body_navigation_hit {
        let target = node.target;
        let action = match target {
            DialogFocusState::Action { index } => controls
                .actions
                .get(index)
                .map(|action| (action.activation(), controls.common.after_action.clone())),
            _ => None,
        };
        let component_click = match target {
            DialogFocusState::Body {
                index,
                part: DialogBodyFocusPartState::Primary,
            } => match controls.common.body_data.get(index) {
                Some(DialogBodyState::PlainMessage { message }) => dialog_text_click_event_at(
                    message,
                    node.rectangle,
                    cursor_position,
                    surface_size,
                ),
                _ => None,
            },
            DialogFocusState::Body {
                index,
                part: DialogBodyFocusPartState::Description,
            } => match controls.common.body_data.get(index) {
                Some(DialogBodyState::Item {
                    description: Some(description),
                    ..
                }) => dialog_text_click_event_at(
                    description,
                    node.rectangle,
                    cursor_position,
                    surface_size,
                ),
                _ => None,
            },
            _ => None,
        };
        let component_after_action = controls.common.after_action.clone();
        world.focus_dialog_navigation_target(target, false);
        if let Some(event) = component_click {
            run_dialog_component_click_event(
                counters,
                world,
                net_commands,
                clipboard,
                &event,
                &component_after_action,
            );
        } else if let Some((activation, after_action)) = action {
            run_dialog_control_activation(
                counters,
                world,
                net_commands,
                clipboard,
                activation,
                &after_action,
            );
        }
        return true;
    }
    let Some((action_index, action, after_action)) = footer_action else {
        return true;
    };
    world.focus_dialog_navigation_target(
        DialogFocusState::Action {
            index: action_index,
        },
        false,
    );
    run_dialog_control_activation(
        counters,
        world,
        net_commands,
        clipboard,
        action,
        &after_action,
    );
    true
}

fn select_focused_dialog_text_word(
    world: &mut WorldStore,
    multiline: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(snapshot) = focused_dialog_text_snapshot(world) else {
        return false;
    };
    let (start, end) = if multiline {
        dialog_multiline_word_selection(&snapshot.value, snapshot.cursor)
    } else {
        (
            text_edit::word_position(&snapshot.value, snapshot.cursor, -1),
            text_edit::word_position(&snapshot.value, snapshot.cursor, 1),
        )
    };
    set_focused_dialog_text(
        world,
        &snapshot,
        snapshot.value.clone(),
        end,
        start,
        text_measurer,
    )
}

fn dialog_multiline_word_selection(text: &str, cursor: usize) -> (usize, usize) {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return (0, 0);
    }
    let mut start = cursor.min(chars.len().saturating_sub(1));
    while start > 0 && chars[start - 1].is_whitespace() {
        start -= 1;
    }
    while start > 0 && !chars[start - 1].is_whitespace() {
        start -= 1;
    }
    let mut end = start;
    while end < chars.len() && !chars[end].is_whitespace() {
        end += 1;
    }
    (start, end)
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
    let value = value.clone();
    let multiline = world.dialog_multiline_text_settings(layout.index).is_some();
    let text_len = text_edit::char_len(&value);
    if multiline {
        let inner_width = u32::try_from(
            layout
                .width
                .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                .max(1),
        )
        .unwrap_or(1);
        let lines = dialog_multiline_lines(&value, inner_width, text_measurer);
        let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
        let scroll_amount = world
            .dialog_text_edit_state()
            .filter(|edit| edit.input_index == layout.index)
            .map_or(max_scroll, |edit| edit.scroll_amount.clamp(0, max_scroll));
        let screen_width = surface_size.width.clamp(1, 512);
        let screen_height = surface_size.height.clamp(1, 512);
        let (origin_x, origin_y) =
            centered_screen_origin(surface_size, screen_width, screen_height);
        let relative_x = (cursor_position.x.floor()
            - (origin_x + i64::from(layout.x) + i64::from(TEXT_INPUT_INSET)) as f64)
            .clamp(0.0, f64::from(inner_width)) as u32;
        let relative_y = (cursor_position.y.floor()
            - (origin_y + i64::from(layout.y) + i64::from(TEXT_INPUT_INSET)) as f64)
            as i32;
        let line_index = usize::try_from(
            relative_y
                .saturating_add(scroll_amount)
                .max(0)
                .saturating_div(TEXT_LINE_HEIGHT),
        )
        .unwrap_or(0)
        .min(lines.len().saturating_sub(1));
        let line = lines[line_index];
        let line_text = dialog_char_slice(&value, line.begin, line.end);
        let cursor = line.begin.saturating_add(
            text_measurer
                .cursor_for_width(&line_text, 0, relative_x)
                .min(line.end.saturating_sub(line.begin)),
        );
        let focused = world.focus_dialog_text_input(layout.index, cursor, extend_selection);
        let scroll_amount =
            dialog_multiline_reveal_cursor(scroll_amount, layout.height, &lines, cursor);
        world.set_dialog_multiline_text_scroll_amount(layout.index, scroll_amount, max_scroll);
        return focused;
    }
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
    let display_start = text_measurer.display_start(&value, scroll_to, inner_width);
    let cursor = text_measurer.cursor_for_width(&value, display_start, position_in_text);
    world.focus_dialog_text_input(layout.index, cursor, extend_selection)
}

fn begin_dialog_multiline_scroll_drag_from_cursor(
    world: &mut WorldStore,
    layout: DialogInputLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(Some(DialogInputValueState::Text { value })) =
        world.current_dialog_input_values().get(layout.index)
    else {
        return false;
    };
    let lines = dialog_multiline_lines(
        value,
        u32::try_from(
            layout
                .width
                .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                .max(1),
        )
        .unwrap_or(1),
        text_measurer,
    );
    let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
    let current_scroll = world
        .dialog_text_edit_state()
        .filter(|edit| edit.input_index == layout.index)
        .map_or(max_scroll, |edit| edit.scroll_amount.clamp(0, max_scroll));
    let Some(scrollbar) = dialog_multiline_scrollbar_layout(layout, lines.len(), current_scroll)
    else {
        return false;
    };
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    if !dialog_scrollbar_contains(
        scrollbar,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    ) {
        return false;
    }
    let Some(cursor_y) = dialog_cursor_screen_y(cursor_position, surface_size, screen_height)
    else {
        return false;
    };
    world.focus_dialog_navigation_target(
        DialogFocusState::Input {
            index: layout.index,
        },
        false,
    );
    let travel = scrollbar
        .height
        .saturating_sub(scrollbar.thumb_height)
        .max(1);
    let next = cursor_y
        .saturating_sub(scrollbar.y)
        .saturating_sub(scrollbar.thumb_height / 2)
        .clamp(0, travel)
        .saturating_mul(max_scroll)
        .checked_div(travel)
        .unwrap_or(0);
    world.set_dialog_multiline_text_scroll_amount(layout.index, next, max_scroll);
    world.begin_dialog_multiline_scroll_drag(layout.index, cursor_y)
}

fn cycle_dialog_single_option(world: &mut WorldStore, index: usize, reverse: bool) -> bool {
    let (selected_index, option_count) = {
        let Some(dialog) = world.current_dialog() else {
            return false;
        };
        let Some(controls) = dialog
            .data
            .as_ref()
            .and_then(|data| dialog_controls(data, world.server_links()))
        else {
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
    if world.dialog_link_confirmation().is_some() || world.dialog_warning_is_open() {
        return true;
    }
    if world.dialog_waiting_ticks().is_some() {
        return true;
    }
    let Some(dialog) = world.current_dialog() else {
        return false;
    };
    let Some(controls) = dialog
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))
    else {
        return true;
    };
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let Some(scroll_layout) =
        dialog_body_scroll_layout(&controls, surface_size, world.dialog_body_scroll_amount())
    else {
        return true;
    };
    if !dialog_body_viewport_contains(
        scroll_layout,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    ) {
        return true;
    }
    let scroll_y = match delta {
        MouseScrollDelta::LineDelta(_, y) => f64::from(y),
        MouseScrollDelta::PixelDelta(position) => position.y,
    };
    world.set_dialog_body_scroll_amount(
        world.dialog_body_scroll_amount() - scroll_y * 10.0,
        f64::from(scroll_layout.max_scroll),
    );
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
    if world.dialog_link_confirmation().is_some() || world.dialog_warning_is_open() {
        world.end_dialog_number_range_drag();
        world.end_dialog_text_input_drag();
        world.end_dialog_multiline_scroll_drag();
        world.end_dialog_body_scroll_drag();
        return true;
    }
    if world.dialog_waiting_ticks().is_some() {
        world.end_dialog_number_range_drag();
        world.end_dialog_text_input_drag();
        world.end_dialog_multiline_scroll_drag();
        world.end_dialog_body_scroll_drag();
        return true;
    }
    if let Some(index) = world.dialog_dragging_multiline_scroll_input() {
        let state = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let scroll_layout = dialog_body_scroll_layout(
                    &controls,
                    surface_size,
                    world.dialog_body_scroll_amount(),
                )?;
                let layout = dialog_input_layouts_at_scroll(
                    &controls,
                    surface_size,
                    scroll_layout.scroll_amount,
                )?
                .into_iter()
                .find(|layout| layout.index == index)?;
                let label_visible = match &controls.common.input_data.get(index)?.control {
                    DialogInputControlState::Text {
                        label_visible,
                        multiline: Some(_),
                        ..
                    } => *label_visible,
                    _ => return None,
                };
                let layout = dialog_multiline_text_box_layout(layout, label_visible);
                let value = match world.current_dialog_input_values().get(index) {
                    Some(Some(DialogInputValueState::Text { value })) => value,
                    _ => return None,
                };
                let lines = dialog_multiline_lines(
                    value,
                    u32::try_from(
                        layout
                            .width
                            .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                            .max(1),
                    )
                    .unwrap_or(1),
                    text_measurer,
                );
                let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
                let current = world
                    .dialog_text_edit_state()
                    .filter(|edit| edit.input_index == index)?
                    .scroll_amount
                    .clamp(0, max_scroll);
                let scrollbar = dialog_multiline_scrollbar_layout(layout, lines.len(), current)?;
                let cursor_y = dialog_cursor_screen_y(
                    cursor_position,
                    surface_size,
                    surface_size.height.clamp(1, 512),
                )?;
                Some((scrollbar, max_scroll, current, cursor_y))
            });
        let Some((scrollbar, max_scroll, current, cursor_y)) = state else {
            world.end_dialog_multiline_scroll_drag();
            return world.current_dialog().is_some();
        };
        let delta_y = world
            .update_dialog_multiline_scroll_drag_y(cursor_y)
            .unwrap_or_default();
        let travel = scrollbar
            .height
            .saturating_sub(scrollbar.thumb_height)
            .max(1);
        let next = if cursor_y < scrollbar.y {
            0
        } else if cursor_y > scrollbar.y.saturating_add(scrollbar.height) {
            max_scroll
        } else {
            current.saturating_add(
                delta_y
                    .saturating_mul(max_scroll.max(1))
                    .checked_div(travel)
                    .unwrap_or(0),
            )
        };
        world.set_dialog_multiline_text_scroll_amount(index, next, max_scroll);
        return true;
    }
    if world.dialog_body_scroll_drag_y().is_some() {
        let screen_height = surface_size.height.clamp(1, 512);
        let Some((layout, scrollbar, cursor_y)) = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let layout = dialog_body_scroll_layout(
                    &controls,
                    surface_size,
                    world.dialog_body_scroll_amount(),
                )?;
                let scrollbar = dialog_scrollbar_layout(layout)?;
                let cursor_y =
                    dialog_cursor_screen_y(cursor_position, surface_size, screen_height)?;
                Some((layout, scrollbar, cursor_y))
            })
        else {
            world.end_dialog_body_scroll_drag();
            return world.current_dialog().is_some();
        };
        let delta_y = world
            .update_dialog_body_scroll_drag_y(cursor_y)
            .unwrap_or_default();
        let next = if cursor_y < layout.rectangle.y {
            0.0
        } else if cursor_y > layout.rectangle.y.saturating_add(layout.rectangle.height) {
            f64::from(layout.max_scroll)
        } else {
            let travel = layout
                .rectangle
                .height
                .saturating_sub(scrollbar.thumb_height);
            let scale = (f64::from(layout.max_scroll.max(1)) / f64::from(travel.max(1))).max(1.0);
            world.dialog_body_scroll_amount() + f64::from(delta_y) * scale
        };
        world.set_dialog_body_scroll_amount(next, f64::from(layout.max_scroll));
        return true;
    }
    if let Some(index) = world.dialog_dragging_text_input() {
        let layout = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let scroll_layout = dialog_body_scroll_layout(
                    &controls,
                    surface_size,
                    world.dialog_body_scroll_amount(),
                )?;
                let layouts = dialog_input_layouts_at_scroll(
                    &controls,
                    surface_size,
                    scroll_layout.scroll_amount,
                )?;
                let layout = layouts.into_iter().find(|layout| layout.index == index)?;
                match &controls.common.input_data.get(index)?.control {
                    DialogInputControlState::Text {
                        label_visible,
                        multiline: None,
                        ..
                    } => Some(dialog_single_line_text_box_layout(layout, *label_visible)),
                    DialogInputControlState::Text {
                        label_visible,
                        multiline: Some(_),
                        ..
                    } => Some(dialog_multiline_text_box_layout(layout, *label_visible)),
                    _ => None,
                }
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
        .and_then(|data| dialog_controls(data, world.server_links()))
        .and_then(|controls| {
            let scroll_layout = dialog_body_scroll_layout(
                &controls,
                surface_size,
                world.dialog_body_scroll_amount(),
            )?;
            dialog_input_layouts_at_scroll(&controls, surface_size, scroll_layout.scroll_amount)
        })
        .and_then(|layouts| layouts.into_iter().find(|layout| layout.index == index));
    let Some(layout) = layout else {
        world.end_dialog_number_range_drag();
        return world.current_dialog().is_some();
    };
    set_dialog_number_range_from_cursor(world, layout, cursor_position, surface_size);
    true
}

fn run_dialog_confirm_link_button(
    world: &mut WorldStore,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
    index: usize,
) -> bool {
    match index {
        0 => world.resolve_dialog_link_confirmation(true),
        1 => {
            let url = world
                .dialog_link_confirmation()
                .map(|confirmation| confirmation.url.clone());
            if let Some(url) = url {
                if !url.is_empty() {
                    if let Some(clipboard) = clipboard.as_deref_mut() {
                        clipboard.set_debug_clipboard_text(&url);
                    }
                }
            }
            world.resolve_dialog_link_confirmation(false)
        }
        2 => world.resolve_dialog_link_confirmation(false),
        _ => false,
    }
}

fn run_dialog_warning_button(
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    disconnect: bool,
) {
    if disconnect {
        if let Some(tx) = net_commands {
            let _ = tx.try_send(NetCommand::Disconnect);
        }
    } else {
        world.close_dialog_warning();
    }
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

fn run_dialog_control_activation(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    activation: DialogControlActivation,
    after_action: &str,
) -> bool {
    match activation {
        DialogControlActivation::Action(action) => run_dialog_action(
            counters,
            world,
            net_commands,
            clipboard,
            action.as_ref(),
            after_action,
        ),
        DialogControlActivation::DialogListEntry(index) => world.show_dialog_list_entry(index),
    }
}

fn run_dialog_component_click_event(
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    event: &ComponentClickEvent,
    after_action: &str,
) -> bool {
    let action = match event {
        ComponentClickEvent::OpenFile { .. } => return false,
        ComponentClickEvent::OpenUrl { url } => DialogActionState::OpenUrl { url: url.clone() },
        ComponentClickEvent::RunCommand { command } => DialogActionState::RunCommand {
            command: command.clone(),
        },
        ComponentClickEvent::SuggestCommand { command } => DialogActionState::SuggestCommand {
            command: command.clone(),
        },
        ComponentClickEvent::ShowDialog { dialog } => {
            return world.show_dialog_from_component_click(dialog);
        }
        ComponentClickEvent::ChangePage { page } => DialogActionState::ChangePage { page: *page },
        ComponentClickEvent::CopyToClipboard { value } => DialogActionState::CopyToClipboard {
            value: value.clone(),
        },
        ComponentClickEvent::Custom { id, payload } => DialogActionState::Custom {
            id: id.clone(),
            payload: payload.clone(),
        },
    };
    run_dialog_action(
        counters,
        world,
        net_commands,
        clipboard,
        Some(&action),
        after_action,
    )
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
        Some(DialogActionState::OpenUrl { url }) => {
            return world.open_dialog_url(url.clone(), after_action.to_string());
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
        .and_then(|data| dialog_controls(data, world.server_links()))?;
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

fn push_dialog_outline_layers(
    layers: &mut Vec<HudInventoryFillLayer>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let right = x.saturating_add(width);
    let bottom = y.saturating_add(height);
    push_dialog_fill_layer(layers, x, y, right, y.saturating_add(1));
    push_dialog_fill_layer(layers, x, bottom.saturating_sub(1), right, bottom);
    push_dialog_fill_layer(
        layers,
        x,
        y.saturating_add(1),
        x.saturating_add(1),
        bottom.saturating_sub(1),
    );
    push_dialog_fill_layer(
        layers,
        right.saturating_sub(1),
        y.saturating_add(1),
        right,
        bottom.saturating_sub(1),
    );
}

fn push_dialog_fill_layer(
    layers: &mut Vec<HudInventoryFillLayer>,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) {
    let width = x0.abs_diff(x1);
    let height = y0.abs_diff(y1);
    if width == 0 || height == 0 {
        return;
    }
    layers.push(HudInventoryFillLayer {
        x: x0.min(x1),
        y: y0.min(y1),
        width,
        height,
        tint: TEXT_COLOR,
        stage: HudInventoryFillStage::Foreground,
    });
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

fn dialog_text_click_event_at(
    message: &DialogPlainMessageState,
    rectangle: DialogFocusRectangle,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<ComponentClickEvent> {
    let cursor = cursor_position?;
    if !cursor.x.is_finite() || !cursor.y.is_finite() {
        return None;
    }
    let screen_width = surface_size.width.clamp(1, 512);
    let screen_height = surface_size.height.clamp(1, 512);
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let cursor_x = i32::try_from((cursor.x.floor() as i64).saturating_sub(origin_x)).ok()?;
    let cursor_y = i32::try_from((cursor.y.floor() as i64).saturating_sub(origin_y)).ok()?;
    let line_y = cursor_y
        .saturating_sub(rectangle.y)
        .saturating_sub(FOCUSABLE_TEXT_PADDING);
    if line_y < 0 || line_y >= rectangle.height.saturating_sub(FOCUSABLE_TEXT_PADDING * 2) {
        return None;
    }
    let line_index = usize::try_from(line_y / TEXT_LINE_HEIGHT).ok()?;
    let block = dialog_text_block(message)?;
    let line = block.lines.get(line_index)?;
    let line_x = rectangle
        .x
        .saturating_add(rectangle.width / 2)
        .saturating_sub(i32::try_from(line.width).unwrap_or(i32::MAX) / 2);
    let x = cursor_x.saturating_sub(line_x);
    if x < 0 {
        return None;
    }
    let x = u32::try_from(x).ok()?;
    line.click_spans
        .iter()
        .find(|span| x >= span.x && x < span.x.saturating_add(span.width))
        .map(|span| span.event.clone())
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
            block_model: block_item_3d_model(
                world,
                &stack,
                Some(item_runtime),
                terrain_textures,
                None,
                false,
                false,
                false,
                false,
                local_state.shift_down,
                local_state.keybind_context,
                0,
            ),
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
            scrolling: None,
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
    let tooltip = hud_item_tooltip(
        world,
        item_runtime,
        stack,
        advanced,
        world.gameplay().game_type == GameType::Creative.id(),
    )?;
    Some(HudInventoryTooltip {
        slot_id: u16::try_from(block_index).unwrap_or(u16::MAX),
        x: i32::try_from(i64::from(cursor_x) - origin_x).ok()?,
        y: i32::try_from(i64::from(cursor_y) - origin_y).ok()?,
        style: tooltip.style,
        lines: tooltip.lines,
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
        scrolling: None,
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
        scrolling: None,
        shadow: false,
        runs,
    });
}

fn push_centered_dialog_control_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    action: DialogControlAction<'_>,
    center_x: i32,
    y: i32,
) {
    if let Some(label) = action.styled_label() {
        push_centered_text_label(labels, label, center_x, y);
    } else {
        push_centered_runs_text_label(
            labels,
            vec![HudStyledTextRun::plain(action.label_text())],
            center_x,
            y,
            None,
        );
    }
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
        scrolling: None,
        shadow: false,
        runs,
    });
}

fn push_scrolling_widget_text_label(
    labels: &mut Vec<HudInventoryTextLabel>,
    runs: Vec<HudStyledTextRun>,
    layout: DialogInputLayout,
) {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    let bounds = HudInventoryScrollingText {
        x: layout
            .x
            .saturating_add(i32::try_from(WIDGET_TEXT_MARGIN).unwrap_or(i32::MAX)),
        y: layout.y,
        width: u32::try_from(layout.width)
            .unwrap_or(1)
            .saturating_sub(WIDGET_TEXT_MARGIN.saturating_mul(2))
            .max(1),
        height: u32::try_from(layout.height).unwrap_or(1).max(1),
    };
    let measured_width = runs_approx_width(&runs).max(1);
    let width = measured_width.min(bounds.width);
    labels.push(HudInventoryTextLabel {
        x: layout
            .x
            .saturating_add(layout.width / 2)
            .saturating_sub(i32::try_from(width).unwrap_or(i32::MAX) / 2),
        y: layout.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
        width,
        text,
        tint: TEXT_COLOR,
        background: None,
        input: None,
        scrolling: Some(bounds),
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
        scrolling: None,
        shadow: false,
        runs,
    });
}

fn wrapped_text_lines(text: &DialogTextState, max_width: u32) -> Vec<DialogTextLine> {
    let mut lines = Vec::new();
    let mut current = empty_text_line();
    let mut ended_with_newline = false;
    for source_run in dialog_source_runs(text) {
        let run = hud_dialog_text_run(&source_run);
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
            if let Some(event) = &source_run.style.click_event {
                if let Some(last) = current.click_spans.last_mut().filter(|last| {
                    last.event == *event && last.x.saturating_add(last.width) == current.width
                }) {
                    last.width = last.width.saturating_add(advance);
                } else {
                    current.click_spans.push(DialogTextClickSpan {
                        x: current.width,
                        width: advance,
                        event: event.clone(),
                    });
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

fn plain_dialog_text_lines(text: &str, max_width: u32) -> Vec<DialogTextLine> {
    wrapped_text_lines(
        &DialogTextState {
            text: text.to_string(),
            runs: Vec::new(),
            component: NbtSummaryValue::String(text.to_string()),
        },
        max_width,
    )
}

fn dialog_warning_message_lines(surface_size: PhysicalSize<u32>) -> Vec<DialogTextLine> {
    let message_width = surface_size
        .width
        .clamp(1, 512)
        .saturating_sub(CONFIRM_MESSAGE_HORIZONTAL_PADDING)
        .max(1);
    plain_dialog_text_lines(DIALOG_WARNING_CONTENTS, message_width)
        .into_iter()
        .take(CONFIRM_MESSAGE_MAX_ROWS)
        .collect()
}

fn empty_text_line() -> DialogTextLine {
    DialogTextLine {
        text: String::new(),
        width: 0,
        runs: Vec::new(),
        click_spans: Vec::new(),
    }
}

fn dialog_text_runs(text: &DialogTextState) -> Vec<HudStyledTextRun> {
    dialog_source_runs(text)
        .iter()
        .map(hud_dialog_text_run)
        .collect()
}

fn dialog_source_runs(text: &DialogTextState) -> Vec<StyledTextRun> {
    if text.runs.is_empty() {
        vec![StyledTextRun {
            text: text.text.clone(),
            style: Default::default(),
        }]
    } else {
        text.runs.clone()
    }
}

fn hud_dialog_text_run(run: &StyledTextRun) -> HudStyledTextRun {
    HudStyledTextRun {
        text: run.text.clone(),
        style: HudTextStyle {
            bold: run.style.bold == Some(true),
            italic: run.style.italic == Some(true),
            underlined: run.style.underlined == Some(true),
            strikethrough: run.style.strikethrough == Some(true),
            obfuscated: run.style.obfuscated == Some(true),
        },
        color: run.style.color,
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
        packets::{
            DialogHolder, ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks,
            ShowDialog,
        },
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
    fn dialog_warning_button_opens_confirm_screen_and_dispatches_back_or_disconnect() {
        let mut world = world_with_notice(Vec::new());
        let surface_size = PhysicalSize::new(320, 240);
        let terrain_textures = TerrainTextureState::default();
        let warning_button = DialogWarningButtonLayout::new(
            &world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap()
                .common
                .title,
            surface_size,
        )
        .unwrap();
        assert_eq!(
            warning_button,
            DialogWarningButtonLayout {
                title_x: 127,
                title_y: 13,
                x: 173,
                y: 7,
            }
        );
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &terrain_textures,
            &InventoryHudLocalState {
                cursor_position: Some((174, 8)),
                ..Default::default()
            },
            surface_size,
            0.0,
        )
        .unwrap();
        assert_eq!(
            (screen.text_labels[0].x, screen.text_labels[0].y),
            (127, 13)
        );
        assert_eq!(
            screen.background_layers[1].texture,
            HudInventoryBackgroundTexture::DialogWarningButtonHighlighted
        );
        assert!(screen.tooltip.as_ref().unwrap().lines[0]
            .text
            .starts_with("This is a custom screen"));

        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(1);
        let net_commands = Some(tx);
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(174.0, 8.0)),
            surface_size,
        ));
        assert!(world.dialog_warning_is_open());

        let warning_screen = hud_dialog_warning_screen(None, None, surface_size);
        assert_eq!(warning_screen.text_labels[0].text, DIALOG_WARNING_TITLE);
        assert!(warning_screen
            .text_labels
            .iter()
            .any(|label| label.text == DIALOG_WARNING_DISCONNECT));
        assert!(warning_screen
            .text_labels
            .iter()
            .any(|label| label.text == DIALOG_WARNING_BACK));
        let message_line_count = dialog_warning_message_lines(surface_size).len();
        let warning_layout = DialogWarningScreenLayout::new(message_line_count, surface_size);

        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(world.dialog_warning_focused_button(), Some(true));
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::ArrowRight),
            ElementState::Pressed,
        ));
        assert_eq!(world.dialog_warning_focused_button(), Some(false));
        let keyboard_focused =
            hud_dialog_warning_screen(world.dialog_warning_focused_button(), None, surface_size);
        assert_eq!(
            keyboard_focused.background_layers[2].texture,
            HudInventoryBackgroundTexture::WidgetButtonHighlighted
        );
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Space),
            ElementState::Pressed,
        ));
        assert!(!world.dialog_warning_is_open());
        assert!(world.current_dialog().is_some());

        assert!(world.open_dialog_warning());
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
            true,
            false,
        ));
        assert_eq!(world.dialog_warning_focused_button(), Some(false));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(
                f64::from(warning_layout.yes.x + 1),
                f64::from(warning_layout.yes.y + 1),
            )),
            surface_size,
        ));
        assert_eq!(rx.try_recv().unwrap(), NetCommand::Disconnect);
        assert!(world.dialog_warning_is_open());
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(!world.dialog_warning_is_open());
    }

    #[test]
    fn dialog_focus_graph_matches_warning_body_input_and_footer_hierarchy() {
        assert_eq!(dialog_content_y(240, 205, FOOTER_HEIGHT), 33);
        let plain = NbtSummaryValue::Compound(vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:plain_message".to_string()),
            ),
            entry("contents", NbtSummaryValue::String("Plain".to_string())),
            entry("width", NbtSummaryValue::Int(80)),
        ]);
        let item = NbtSummaryValue::Compound(vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:item".to_string()),
            ),
            entry(
                "item",
                NbtSummaryValue::Compound(vec![entry(
                    "id",
                    NbtSummaryValue::String("minecraft:stick".to_string()),
                )]),
            ),
            entry(
                "description",
                NbtSummaryValue::Compound(vec![
                    entry("contents", NbtSummaryValue::String("Info".to_string())),
                    entry("width", NbtSummaryValue::Int(50)),
                ]),
            ),
            entry("width", NbtSummaryValue::Int(32)),
            entry("height", NbtSummaryValue::Int(20)),
        ]);
        let options = vec![
            NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::String("a".to_string())),
                entry("initial", NbtSummaryValue::Byte(1)),
            ]),
            NbtSummaryValue::Compound(vec![entry("id", NbtSummaryValue::String("b".to_string()))]),
        ];
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry("body", NbtSummaryValue::List(vec![plain, item])),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![
                    boolean_input("enabled", "Enabled", false, "yes", "no"),
                    number_range_input("amount", "Amount", 100, 0.0, 10.0, None, None),
                    single_option_input("mode", "Mode", 100, true, options),
                    text_input("name", "Name", 100, false, "ab", 16),
                ]),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Done".to_string())),
                    entry(
                        "tooltip",
                        NbtSummaryValue::String("Finish dialog".to_string()),
                    ),
                    entry("width", NbtSummaryValue::Int(150)),
                ]),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        let graph = dialog_focus_graph(&controls, surface_size).unwrap();
        let expected = vec![
            DialogFocusState::WarningButton,
            DialogFocusState::Body {
                index: 0,
                part: DialogBodyFocusPartState::Primary,
            },
            DialogFocusState::Body {
                index: 1,
                part: DialogBodyFocusPartState::Primary,
            },
            DialogFocusState::Body {
                index: 1,
                part: DialogBodyFocusPartState::Description,
            },
            DialogFocusState::Input { index: 0 },
            DialogFocusState::Input { index: 1 },
            DialogFocusState::Input { index: 2 },
            DialogFocusState::Input { index: 3 },
            DialogFocusState::Action { index: 0 },
        ];
        assert_eq!(graph.tab, expected);
        assert_eq!(
            next_dialog_focus_in_direction(
                &graph,
                Some(DialogFocusState::WarningButton),
                DialogFocusDirection::Down,
            ),
            Some(expected[1])
        );
        assert_eq!(
            next_dialog_focus_in_direction(&graph, Some(expected[1]), DialogFocusDirection::Down,),
            Some(expected[2])
        );
        assert_eq!(
            next_dialog_focus_in_direction(&graph, Some(expected[2]), DialogFocusDirection::Right,),
            Some(expected[3])
        );
        assert_eq!(
            next_dialog_focus_in_direction(&graph, Some(expected[7]), DialogFocusDirection::Down,),
            Some(expected[8])
        );
        assert_eq!(
            next_dialog_focus_in_direction(&graph, Some(expected[8]), DialogFocusDirection::Up,),
            Some(expected[7])
        );

        let mut counters = NetCounters::default();
        for target in &expected {
            assert!(handle_dialog_key_input_with_modifiers_for_surface(
                &mut counters,
                &mut world,
                &None,
                None,
                PhysicalKey::Code(KeyCode::Tab),
                ElementState::Pressed,
                false,
                false,
                surface_size,
            ));
            assert_eq!(world.dialog_focus(), Some(*target));
            if *target == (DialogFocusState::Input { index: 1 }) {
                assert!(world.dialog_number_range_keyboard_can_change(1));
            }
        }
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
            false,
            false,
            surface_size,
        ));
        assert_eq!(world.dialog_focus(), Some(expected[0]));

        let focused_warning = hud_dialog_screen(
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
            focused_warning.background_layers[1].texture,
            HudInventoryBackgroundTexture::DialogWarningButtonHighlighted
        );
        assert!(focused_warning.tooltip.is_some());
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Space),
            ElementState::Pressed,
            false,
            false,
            surface_size,
        ));
        assert!(world.dialog_warning_is_open());
        assert_eq!(world.dialog_focus(), Some(expected[0]));
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
            false,
            false,
            surface_size,
        ));
        assert!(!world.dialog_warning_is_open());

        assert!(world.focus_dialog_navigation_target(expected[1], false));
        let focused_plain = hud_dialog_screen(
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
            focused_plain
                .scroll_area
                .as_ref()
                .unwrap()
                .fill_layers
                .len(),
            4
        );

        let description = dialog_focus_node(&graph.body, expected[3]).unwrap();
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(
                f64::from(description.rectangle.x + 1),
                f64::from(description.rectangle.y + 1),
            )),
            surface_size,
        ));
        assert_eq!(world.dialog_focus(), Some(expected[3]));

        assert!(world.focus_dialog_navigation_target(expected[7], false));
        assert_eq!(world.dialog_text_edit_state().unwrap().cursor, 2);
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::ArrowLeft),
            ElementState::Pressed,
            false,
            false,
            surface_size,
        ));
        assert_eq!(world.dialog_text_edit_state().unwrap().cursor, 1);
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
            false,
            false,
            surface_size,
        ));
        assert_eq!(world.dialog_focus(), Some(expected[8]));
        assert_eq!(world.dialog_text_edit_state(), None);
        let focused_action = hud_dialog_screen(
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
            focused_action.background_layers.last().unwrap().texture,
            HudInventoryBackgroundTexture::WidgetButtonHighlighted
        );
        assert_eq!(
            focused_action.tooltip.unwrap().lines[0].text,
            "Finish dialog"
        );
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
            false,
            false,
            surface_size,
        ));
        assert!(world.current_dialog().is_some());
        assert_eq!(world.dialog_focus(), Some(expected[8]));
        assert!(handle_dialog_key_input_with_modifiers_for_surface(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
            true,
            false,
            surface_size,
        ));
        assert_eq!(world.dialog_focus(), Some(expected[7]));
        assert_eq!(world.dialog_text_edit_state().unwrap().cursor, 1);
    }

    #[test]
    fn overflowing_dialog_body_scrolls_clips_and_reveals_keyboard_focus() {
        let body = (0..10)
            .map(|index| {
                NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:plain_message".to_string()),
                    ),
                    entry("contents", NbtSummaryValue::String(format!("Line {index}"))),
                    entry("width", NbtSummaryValue::Int(80)),
                ])
            })
            .collect::<Vec<_>>();
        let options = vec![
            NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::String("a".to_string())),
                entry("initial", NbtSummaryValue::Byte(1)),
            ]),
            NbtSummaryValue::Compound(vec![entry("id", NbtSummaryValue::String("b".to_string()))]),
        ];
        let mut world = world_with_notice(vec![
            entry("body", NbtSummaryValue::List(body)),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![single_option_input(
                    "mode", "Mode", 100, true, options,
                )]),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        let layout = dialog_body_scroll_layout(&controls, surface_size, 0.0).unwrap();
        assert_eq!(
            layout.rectangle,
            DialogFocusRectangle {
                x: 100,
                y: 33,
                width: 120,
                height: 174,
            }
        );
        assert_eq!((layout.content_height, layout.max_scroll), (290, 116));
        assert_eq!(
            dialog_scrollbar_layout(layout),
            Some(DialogScrollbarLayout {
                x: 214,
                y: 33,
                width: 6,
                height: 174,
                thumb_y: 33,
                thumb_height: 104,
            })
        );
        let _ = controls;

        assert!(world.focus_dialog_navigation_target(DialogFocusState::Input { index: 0 }, true,));
        assert!(reveal_dialog_keyboard_focus(
            &mut world,
            DialogFocusState::Input { index: 0 },
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_amount(), 116.0);
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
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        assert_eq!(
            scroll_area.scissor,
            HudInventoryItemScissor {
                x: 100,
                y: 33,
                width: 120,
                height: 174,
            }
        );
        assert_eq!(scroll_area.scrollbar.unwrap().thumb_y, 103);
        assert!(scroll_area
            .text_labels
            .iter()
            .any(|label| label.text == "Mode: a" && label.y == 193));

        let input_cursor = Some(PhysicalPosition::new(160.0, 190.0));
        assert!(handle_dialog_mouse_wheel(
            &mut world,
            MouseScrollDelta::LineDelta(0.0, 1.0),
            input_cursor,
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_amount(), 106.0);
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::SingleOption {
                selected_index: 0,
            })]
        );

        world.set_dialog_body_scroll_amount(0.0, 116.0);
        let mut counters = NetCounters::default();
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(215.0, 40.0)),
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_drag_y(), Some(40));
        assert!(handle_dialog_cursor_moved(
            &mut world,
            Some(PhysicalPosition::new(215.0, 220.0)),
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_amount(), 116.0);
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Released,
            None,
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_drag_y(), None);
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(220.0, 40.0)),
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_drag_y(), Some(40));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Released,
            None,
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_drag_y(), None);
    }

    #[test]
    fn dialog_item_body_mouse_focus_uses_only_the_declared_widget_rect() {
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "body",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:item".to_string()),
                    ),
                    entry(
                        "item",
                        NbtSummaryValue::Compound(vec![entry(
                            "id",
                            NbtSummaryValue::String("minecraft:stick".to_string()),
                        )]),
                    ),
                    entry(
                        "description",
                        NbtSummaryValue::Compound(vec![
                            entry("contents", NbtSummaryValue::String("Info".to_string())),
                            entry("width", NbtSummaryValue::Int(50)),
                        ]),
                    ),
                    entry("width", NbtSummaryValue::Int(32)),
                    entry("height", NbtSummaryValue::Int(48)),
                ]),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let dialog = world.current_dialog().unwrap();
        let controls = dialog
            .data
            .as_ref()
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap_or_else(|| panic!("dialog decode failed: {:?}", dialog.semantic_error));
        let item_focus = dialog_focus_node(
            &dialog_focus_graph(&controls, surface_size).unwrap().body,
            DialogFocusState::Body {
                index: 0,
                part: DialogBodyFocusPartState::Primary,
            },
        )
        .unwrap();
        assert_eq!(
            item_focus.rectangle,
            DialogFocusRectangle {
                x: 118,
                y: 63,
                width: 32,
                height: 48,
            }
        );
        let mut counters = NetCounters::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(149.0, 110.0)),
            surface_size,
        ));
        assert_eq!(world.dialog_focused_body(), Some(0));

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(150.0, 64.0)),
            surface_size,
        ));
        assert_eq!(world.dialog_focused_body(), Some(0));

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(160.0, 220.0)),
            surface_size,
        ));
        assert_eq!(world.dialog_focused_body(), None);
        assert!(world.current_dialog().is_some());
    }

    #[test]
    fn plain_message_component_click_requires_a_clickable_glyph() {
        let event = ComponentClickEvent::CopyToClipboard {
            value: "copied value".to_string(),
        };
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "body",
                plain_message_body(
                    clickable_component(
                        "Copy",
                        "copy_to_clipboard",
                        vec![entry(
                            "value",
                            NbtSummaryValue::String("copied value".to_string()),
                        )],
                    ),
                    80,
                ),
            ),
        ]);
        let DialogBodyState::PlainMessage { message } = &world
            .current_dialog()
            .unwrap()
            .data
            .as_ref()
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap()
            .common
            .body_data[0]
        else {
            panic!("expected plain-message body");
        };
        assert_eq!(
            dialog_text_block(message).unwrap().lines[0].click_spans,
            [DialogTextClickSpan {
                x: 0,
                width: 24,
                event,
            }]
        );
        let mut clipboard = MockDialogClipboard::default();
        let mut counters = NetCounters::default();
        let surface_size = PhysicalSize::new(320, 240);

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            Some(&mut clipboard),
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(121.0, 64.0)),
            surface_size,
        ));
        assert_eq!(clipboard.text, None);
        assert_eq!(
            world.dialog_focus(),
            Some(DialogFocusState::Body {
                index: 0,
                part: DialogBodyFocusPartState::Primary,
            })
        );

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            Some(&mut clipboard),
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(149.0, 68.0)),
            surface_size,
        ));
        assert_eq!(clipboard.text.as_deref(), Some("copied value"));
        assert!(world.current_dialog().is_some());
    }

    #[test]
    fn item_description_component_click_opens_untrusted_url_prompt() {
        let mut world = world_with_notice(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "body",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:item".to_string()),
                    ),
                    entry(
                        "item",
                        NbtSummaryValue::Compound(vec![entry(
                            "id",
                            NbtSummaryValue::String("minecraft:stick".to_string()),
                        )]),
                    ),
                    entry(
                        "description",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "contents",
                                clickable_component(
                                    "Info",
                                    "open_url",
                                    vec![entry(
                                        "url",
                                        NbtSummaryValue::String(
                                            "https://example.invalid/info".to_string(),
                                        ),
                                    )],
                                ),
                            ),
                            entry("width", NbtSummaryValue::Int(50)),
                        ]),
                    ),
                    entry("width", NbtSummaryValue::Int(32)),
                    entry("height", NbtSummaryValue::Int(48)),
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
            Some(PhysicalPosition::new(166.0, 84.0)),
            PhysicalSize::new(320, 240),
        ));

        assert_eq!(
            world.dialog_link_confirmation(),
            Some(&DialogLinkConfirmationState {
                url: "https://example.invalid/info".to_string(),
                after_action: "none".to_string(),
            })
        );
    }

    #[test]
    fn plain_message_run_command_click_applies_parent_after_action() {
        let mut world = world_with_notice(vec![
            entry(
                "after_action",
                NbtSummaryValue::String("wait_for_response".to_string()),
            ),
            entry(
                "body",
                plain_message_body(
                    clickable_component(
                        "Run",
                        "run_command",
                        vec![entry(
                            "command",
                            NbtSummaryValue::String("/say component".to_string()),
                        )],
                    ),
                    80,
                ),
            ),
        ]);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(1);

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &Some(tx),
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(152.0, 68.0)),
            PhysicalSize::new(320, 240),
        ));

        assert_eq!(counters.chat_command_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::ChatCommand(command) if command.command == "say component"
        ));
        assert_eq!(world.dialog_waiting_ticks(), Some(0));
    }

    #[test]
    fn plain_message_show_dialog_click_replaces_parent_without_packet_count() {
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
                    "body",
                    plain_message_body(
                        clickable_component(
                            "Show",
                            "show_dialog",
                            vec![entry("dialog", notice_dialog("Inner", None))],
                        ),
                        80,
                    ),
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
            Some(PhysicalPosition::new(149.0, 68.0)),
            PhysicalSize::new(320, 240),
        ));

        assert!(matches!(
            world.current_dialog().and_then(|dialog| dialog.data.as_ref()),
            Some(DialogDataState::Notice { common, .. }) if common.title.text == "Inner"
        ));
        assert_eq!(world.counters().show_dialog_packets, 1);
        assert!(world.close_dialog());
        assert!(world.advancements_screen_is_open());
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

        assert_eq!(screen.background_layers.len(), 4);
        assert_eq!(screen.background_layers[0].x, 0);
        assert_eq!(screen.background_layers[0].width, 320);
        assert_eq!(
            screen.background_layers[0].texture,
            HudInventoryBackgroundTexture::InworldMenuBackground
        );
        assert_eq!(screen.background_layers[0].uv.max, [10.0, 7.5]);
        assert_eq!(screen.background_layers[1].x, 176);
        assert_eq!(screen.background_layers[1].width, 20);
        assert_eq!(
            screen.background_layers[1].texture,
            HudInventoryBackgroundTexture::DialogWarningButton
        );
        assert_eq!(screen.background_layers[2].x, 66);
        assert_eq!(screen.background_layers[2].width, 80);
        assert_eq!(
            screen.background_layers[2].texture,
            HudInventoryBackgroundTexture::WidgetButtonHighlighted
        );
        assert_eq!(screen.background_layers[3].x, 154);
        assert_eq!(screen.background_layers[3].width, 100);
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
    fn multi_action_projects_scrolled_button_grid_focus_tooltips_and_exit() {
        let dynamic_button = NbtSummaryValue::Compound(vec![
            entry("label", NbtSummaryValue::String("One".to_string())),
            entry("tooltip", NbtSummaryValue::String("First tip".to_string())),
            entry("width", NbtSummaryValue::Int(80)),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:dynamic/custom".to_string()),
                    ),
                    entry("id", NbtSummaryValue::String("example:one".to_string())),
                ]),
            ),
        ]);
        let mut world = world_with_multi_action(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "inputs",
                NbtSummaryValue::List(vec![boolean_input(
                    "enabled", "Enabled", false, "yes", "no",
                )]),
            ),
            entry(
                "actions",
                NbtSummaryValue::List(vec![
                    dynamic_button,
                    custom_button("Two", None, 100, "example:two", None),
                    custom_button("Three", None, 60, "example:three", None),
                    custom_button("Four", Some("Fourth tip"), 60, "example:four", None),
                    custom_button("Five", None, 120, "example:five", None),
                ]),
            ),
            entry("columns", NbtSummaryValue::Int(3)),
            entry(
                "exit_action",
                custom_button("Exit", Some("Exit tip"), 150, "example:exit", None),
            ),
        ]);
        let surface_size = PhysicalSize::new(320, 240);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        assert_eq!(controls.body_action_count, 5);
        assert_eq!(controls.footer_height, FOOTER_HEIGHT);
        let grid = dialog_body_action_grid_layout(&controls).unwrap();
        assert_eq!((grid.width, grid.height), (244, 42));
        assert_eq!(
            grid.buttons,
            vec![
                DialogButtonLayout {
                    x: 0,
                    y: 0,
                    width: 80,
                },
                DialogButtonLayout {
                    x: 82,
                    y: 0,
                    width: 100,
                },
                DialogButtonLayout {
                    x: 184,
                    y: 0,
                    width: 60,
                },
                DialogButtonLayout {
                    x: 31,
                    y: 22,
                    width: 60,
                },
                DialogButtonLayout {
                    x: 93,
                    y: 22,
                    width: 120,
                },
            ]
        );
        let graph = dialog_focus_graph(&controls, surface_size).unwrap();
        assert_eq!(
            graph.tab,
            vec![
                DialogFocusState::WarningButton,
                DialogFocusState::Input { index: 0 },
                DialogFocusState::Action { index: 0 },
                DialogFocusState::Action { index: 1 },
                DialogFocusState::Action { index: 2 },
                DialogFocusState::Action { index: 3 },
                DialogFocusState::Action { index: 4 },
                DialogFocusState::Action { index: 5 },
            ]
        );

        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState {
                cursor_position: Some((70, 113)),
                ..Default::default()
            },
            surface_size,
            0.0,
        )
        .unwrap();
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        assert_eq!(
            scroll_area
                .background_layers
                .iter()
                .filter(|layer| matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::WidgetButton
                        | HudInventoryBackgroundTexture::WidgetButtonHighlighted
                ))
                .count(),
            15
        );
        let four = scroll_area
            .text_labels
            .iter()
            .find(|label| label.text == "Four")
            .unwrap();
        assert_eq!(four.y, 118);
        assert_eq!(four.x + i32::try_from(four.width).unwrap() / 2, 99);
        assert_eq!(screen.background_layers[2].x, 85);
        assert_eq!(screen.background_layers[2].y, 213);
        assert_eq!(screen.background_layers[2].width, 150);
        assert_eq!(screen.tooltip.as_ref().unwrap().lines[0].text, "Fourth tip");

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
            Some(PhysicalPosition::new(130.0, 64.0)),
            surface_size,
        ));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &net_commands,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(39.0, 91.0)),
            surface_size,
        ));
        assert_eq!(
            world.dialog_focus(),
            Some(DialogFocusState::Action { index: 0 })
        );
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::CustomClickAction(CustomClickAction {
                id,
                payload: Some(NbtSummaryValue::Compound(entries)),
            }) if id == "example:one"
                && entries == vec![entry("enabled", NbtSummaryValue::Byte(1))]
        ));
        assert!(world.current_dialog().is_some());

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
            NetCommand::CustomClickAction(CustomClickAction { id, .. }) if id == "example:exit"
        ));
        assert_eq!(counters.custom_click_action_commands_queued, 2);
    }

    #[test]
    fn multi_action_without_exit_uses_five_pixel_footer_and_reveals_grid_focus() {
        let actions = (0..9)
            .map(|index| {
                custom_button(
                    &format!("Action {index}"),
                    None,
                    100,
                    &format!("example:{index}"),
                    None,
                )
            })
            .collect::<Vec<_>>();
        let mut world = world_with_multi_action(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry("actions", NbtSummaryValue::List(actions)),
            entry("columns", NbtSummaryValue::Int(1)),
        ]);
        let surface_size = PhysicalSize::new(320, 120);
        let dialog = world.current_dialog().unwrap();
        let controls = dialog
            .data
            .as_ref()
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap_or_else(|| panic!("invalid multi-action dialog: {:?}", dialog.semantic_error));
        assert_eq!(controls.footer_height, 5);
        assert!(controls.footer_actions().is_empty());
        let scroll = dialog_body_scroll_layout(&controls, surface_size, 0.0).unwrap();
        assert_eq!(scroll.rectangle.height, 82);
        assert_eq!(scroll.content_height, 196);
        assert_eq!(scroll.max_scroll, 114);

        assert!(world.focus_dialog_navigation_target(DialogFocusState::Action { index: 8 }, true));
        assert!(reveal_dialog_keyboard_focus(
            &mut world,
            DialogFocusState::Action { index: 8 },
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_amount(), 114.0);

        let mut counters = NetCounters::default();
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(world.current_dialog().is_none());
    }

    #[test]
    fn multi_action_large_column_count_preserves_partial_row_without_large_allocation() {
        let world = world_with_multi_action(vec![
            entry("pause", NbtSummaryValue::Byte(0)),
            entry("after_action", NbtSummaryValue::String("none".to_string())),
            entry(
                "actions",
                NbtSummaryValue::List(vec![
                    custom_button("A", None, 1, "example:a", None),
                    custom_button("B", None, 1, "example:b", None),
                ]),
            ),
            entry("columns", NbtSummaryValue::Int(100)),
        ]);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        assert_eq!(
            dialog_body_action_grid_layout(&controls).unwrap(),
            DialogBodyActionGridLayout {
                width: 198,
                height: 20,
                buttons: vec![
                    DialogButtonLayout {
                        x: 97,
                        y: 0,
                        width: 1,
                    },
                    DialogButtonLayout {
                        x: 100,
                        y: 0,
                        width: 1,
                    },
                ],
            }
        );
    }

    #[test]
    fn dialog_list_projects_external_titles_uniform_grid_and_opens_direct_entry() {
        let targets = (0..5)
            .map(|index| {
                notice_dialog(
                    &format!("Internal {index}"),
                    Some(&format!("External {index}")),
                )
            })
            .collect::<Vec<_>>();
        let mut world = world_with_dialog_list(
            vec![
                entry("dialogs", NbtSummaryValue::List(targets)),
                entry("columns", NbtSummaryValue::Int(3)),
                entry("button_width", NbtSummaryValue::Int(70)),
                entry(
                    "exit_action",
                    custom_button("Back", Some("Exit tip"), 90, "example:back", None),
                ),
            ],
            false,
        );
        let surface_size = PhysicalSize::new(320, 240);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        assert_eq!(controls.body_action_count, 5);
        assert_eq!(controls.footer_height, FOOTER_HEIGHT);
        assert!(controls
            .body_actions()
            .iter()
            .all(|action| action.width() == 70 && action.tooltip().is_none()));
        assert_eq!(controls.body_actions()[3].label_text(), "External 3");
        assert_eq!(
            dialog_body_action_grid_layout(&controls).unwrap(),
            DialogBodyActionGridLayout {
                width: 214,
                height: 42,
                buttons: vec![
                    DialogButtonLayout {
                        x: 0,
                        y: 0,
                        width: 70,
                    },
                    DialogButtonLayout {
                        x: 72,
                        y: 0,
                        width: 70,
                    },
                    DialogButtonLayout {
                        x: 144,
                        y: 0,
                        width: 70,
                    },
                    DialogButtonLayout {
                        x: 36,
                        y: 22,
                        width: 70,
                    },
                    DialogButtonLayout {
                        x: 108,
                        y: 22,
                        width: 70,
                    },
                ],
            }
        );
        assert_eq!(
            dialog_focus_graph(&controls, surface_size).unwrap().tab,
            vec![
                DialogFocusState::WarningButton,
                DialogFocusState::Action { index: 0 },
                DialogFocusState::Action { index: 1 },
                DialogFocusState::Action { index: 2 },
                DialogFocusState::Action { index: 3 },
                DialogFocusState::Action { index: 4 },
                DialogFocusState::Action { index: 5 },
            ]
        );

        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState {
                cursor_position: Some((54, 64)),
                ..Default::default()
            },
            surface_size,
            0.0,
        )
        .unwrap();
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        assert!(scroll_area
            .text_labels
            .iter()
            .any(|label| label.text == "External 0"));
        assert_eq!(
            scroll_area
                .background_layers
                .iter()
                .filter(|layer| matches!(
                    layer.texture,
                    HudInventoryBackgroundTexture::WidgetButton
                        | HudInventoryBackgroundTexture::WidgetButtonHighlighted
                ))
                .count(),
            15
        );
        assert_eq!(screen.tooltip, None);

        let mut counters = NetCounters::default();
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(54.0, 64.0)),
            surface_size,
        ));
        let target = world.current_dialog().unwrap();
        assert_eq!(target.holder_kind, "direct");
        assert!(matches!(
            target.data,
            Some(DialogDataState::Notice { ref common, .. }) if common.title.text == "Internal 0"
        ));
        assert_eq!(world.counters().show_dialog_packets, 1);
        assert_eq!(world.dialog_focus(), None);
        assert_eq!(world.dialog_body_scroll_amount(), 0.0);
    }

    #[test]
    fn context_free_dialog_list_without_exit_reveals_and_opens_entry() {
        let targets = (0..9)
            .map(|index| notice_dialog(&format!("Target {index}"), None))
            .collect::<Vec<_>>();
        let mut world = world_with_dialog_list(
            vec![
                entry("dialogs", NbtSummaryValue::List(targets)),
                entry("columns", NbtSummaryValue::Int(1)),
                entry("button_width", NbtSummaryValue::Int(100)),
            ],
            true,
        );
        let surface_size = PhysicalSize::new(320, 120);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        assert_eq!(controls.footer_height, 5);
        assert!(controls.footer_actions().is_empty());
        let scroll = dialog_body_scroll_layout(&controls, surface_size, 0.0).unwrap();
        assert_eq!(scroll.rectangle.height, 82);
        assert_eq!(scroll.content_height, 196);
        assert_eq!(scroll.max_scroll, 114);
        let graph = dialog_focus_graph(&controls, surface_size).unwrap();
        let DialogTopFocusNode::Body(container) = graph.top[0] else {
            panic!("body container must lead the top-level focus graph");
        };
        assert_eq!(container.rectangle.height, 82);

        let target = DialogFocusState::Action { index: 8 };
        assert!(world.focus_dialog_navigation_target(target, true));
        assert!(reveal_dialog_keyboard_focus(
            &mut world,
            target,
            surface_size,
        ));
        assert_eq!(world.dialog_body_scroll_amount(), 114.0);
        let mut counters = NetCounters::default();
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
        ));
        let target = world.current_dialog().unwrap();
        assert_eq!(target.holder_kind, "context_free");
        assert!(matches!(
            target.data,
            Some(DialogDataState::Notice { ref common, .. }) if common.title.text == "Target 8"
        ));
    }

    #[test]
    fn server_links_dialog_projects_canonical_links_and_generated_open_url_actions() {
        let mut world = world_with_server_links_dialog(
            vec![
                ServerLinkEntry {
                    link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                    url: "https://example.invalid/support".to_string(),
                },
                ServerLinkEntry {
                    link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                    url: "https://example.invalid/bugs".to_string(),
                },
                ServerLinkEntry {
                    link_type: ServerLinkType::Custom {
                        label: "Rules".to_string(),
                    },
                    url: "http://example.invalid/rules".to_string(),
                },
                ServerLinkEntry {
                    link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                    url: "ftp://example.invalid/rejected".to_string(),
                },
            ],
            vec![
                entry("columns", NbtSummaryValue::Int(2)),
                entry("button_width", NbtSummaryValue::Int(90)),
                entry(
                    "exit_action",
                    custom_button("Back", Some("Exit tip"), 100, "example:back", None),
                ),
            ],
        );
        assert_eq!(world.counters().server_link_invalid_entries, 1);
        let surface_size = PhysicalSize::new(320, 240);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        assert_eq!(controls.body_action_count, 3);
        assert_eq!(controls.footer_height, FOOTER_HEIGHT);
        assert_eq!(
            controls
                .body_actions()
                .iter()
                .map(|action| action.label_text())
                .collect::<Vec<_>>(),
            vec!["Support", "Report Server Bug", "Rules"]
        );
        assert!(controls
            .body_actions()
            .iter()
            .all(|action| action.width() == 90 && action.tooltip().is_none()));
        assert!(matches!(
            controls.body_actions()[0].activation(),
            DialogControlActivation::Action(Some(DialogActionState::OpenUrl { url }))
                if url == "https://example.invalid/support"
        ));
        assert_eq!(
            dialog_body_action_grid_layout(&controls).unwrap(),
            DialogBodyActionGridLayout {
                width: 182,
                height: 42,
                buttons: vec![
                    DialogButtonLayout {
                        x: 0,
                        y: 0,
                        width: 90,
                    },
                    DialogButtonLayout {
                        x: 92,
                        y: 0,
                        width: 90,
                    },
                    DialogButtonLayout {
                        x: 46,
                        y: 22,
                        width: 90,
                    },
                ],
            }
        );
        assert_eq!(
            dialog_focus_graph(&controls, surface_size).unwrap().tab,
            vec![
                DialogFocusState::WarningButton,
                DialogFocusState::Action { index: 0 },
                DialogFocusState::Action { index: 1 },
                DialogFocusState::Action { index: 2 },
                DialogFocusState::Action { index: 3 },
            ]
        );

        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState {
                cursor_position: Some((70, 64)),
                ..Default::default()
            },
            surface_size,
            0.0,
        )
        .unwrap();
        assert!(screen
            .scroll_area
            .as_ref()
            .unwrap()
            .text_labels
            .iter()
            .any(|label| label.text == "Report Server Bug"));
        assert_eq!(screen.tooltip, None);

        let mut counters = NetCounters::default();
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(70.0, 64.0)),
            surface_size,
        ));
        assert_eq!(
            world.dialog_focus(),
            Some(DialogFocusState::Action { index: 0 })
        );
        assert!(world.current_dialog().is_some());
        assert_eq!(
            world.dialog_link_confirmation(),
            Some(&DialogLinkConfirmationState {
                url: "https://example.invalid/support".to_string(),
                after_action: "close".to_string(),
            })
        );
    }

    #[test]
    fn confirm_link_screen_projects_vanilla_text_layout_and_three_buttons() {
        let mut world = world_with_notice(vec![entry(
            "after_action",
            NbtSummaryValue::String("none".to_string()),
        )]);
        assert!(world.open_dialog_url(
            "https://example.invalid/docs".to_string(),
            "none".to_string(),
        ));
        assert!(world.focus_dialog_link_button(1));
        let surface_size = PhysicalSize::new(320, 240);

        let screen = hud_dialog_confirm_link_screen(
            world.dialog_link_confirmation().unwrap(),
            world.dialog_link_focused_button(),
            None,
            surface_size,
        );

        assert_eq!((screen.width, screen.height), (320, 240));
        assert!(screen.text_labels.iter().any(|label| {
            label.text == CONFIRM_LINK_TITLE && label.y == 76 && label.tint == TEXT_COLOR
        }));
        assert!(screen
            .text_labels
            .iter()
            .any(|label| { label.text == "https://example.invalid/docs" && label.y == 93 }));
        assert!(screen.text_labels.iter().any(|label| {
            label.text == CONFIRM_LINK_WARNING
                && label.y == 110
                && label.tint == CONFIRM_LINK_WARNING_COLOR
        }));
        assert_eq!(
            screen
                .text_labels
                .iter()
                .filter(|label| matches!(label.text.as_str(), "Yes" | "Copy to Clipboard" | "No"))
                .map(|label| (label.text.as_str(), label.y))
                .collect::<Vec<_>>(),
            [("Yes", 149), ("Copy to Clipboard", 149), ("No", 149)]
        );
        assert_eq!(
            screen.background_layers[1..]
                .iter()
                .map(|layer| (layer.x, layer.y, layer.width, layer.texture))
                .collect::<Vec<_>>(),
            [
                (6, 143, 100, HudInventoryBackgroundTexture::WidgetButton),
                (
                    110,
                    143,
                    100,
                    HudInventoryBackgroundTexture::WidgetButtonHighlighted,
                ),
                (214, 143, 100, HudInventoryBackgroundTexture::WidgetButton),
            ]
        );
    }

    #[test]
    fn confirm_link_keyboard_copies_or_cancels_without_opening_uri() {
        let mut world = world_with_notice(vec![entry(
            "after_action",
            NbtSummaryValue::String("none".to_string()),
        )]);
        let mut counters = NetCounters::default();
        assert!(run_dialog_action(
            &mut counters,
            &mut world,
            &None,
            None,
            Some(&DialogActionState::OpenUrl {
                url: "https://example.invalid/copy".to_string(),
            }),
            "none",
        ));
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
        ));
        assert_eq!(world.dialog_link_focused_button(), Some(0));
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
        ));
        assert_eq!(world.dialog_link_focused_button(), Some(1));
        let mut clipboard = MockDialogClipboard::default();
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            Some(&mut clipboard),
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
        ));
        assert_eq!(
            clipboard.text.as_deref(),
            Some("https://example.invalid/copy")
        );
        assert_eq!(world.dialog_link_confirmation(), None);
        assert!(world.current_dialog().is_some());
        assert!(world.take_dialog_open_url_requests().is_empty());

        assert!(world.open_dialog_url(
            "https://example.invalid/cancel".to_string(),
            "none".to_string(),
        ));
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert_eq!(world.dialog_link_confirmation(), None);
        assert!(world.current_dialog().is_some());
        assert!(world.take_dialog_open_url_requests().is_empty());
    }

    #[test]
    fn confirm_link_mouse_yes_queues_uri_and_applies_close_action() {
        let mut world = world_with_notice(Vec::new());
        assert!(world.open_dialog_url(
            "https://example.invalid/open".to_string(),
            "close".to_string(),
        ));
        let mut counters = NetCounters::default();

        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(7.0, 144.0)),
            PhysicalSize::new(320, 240),
        ));

        assert_eq!(world.dialog_link_confirmation(), None);
        assert!(world.current_dialog().is_none());
        assert_eq!(
            world.take_dialog_open_url_requests(),
            ["https://example.invalid/open"]
        );
    }

    #[test]
    fn empty_server_links_dialog_uses_five_pixel_footer_and_escape_close() {
        let mut world = world_with_server_links_dialog(Vec::new(), Vec::new());
        let surface_size = PhysicalSize::new(320, 120);
        let controls = world
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .unwrap();
        assert_eq!(controls.body_action_count, 0);
        assert_eq!(controls.footer_height, 5);
        assert!(controls.actions.is_empty());
        assert_eq!(
            dialog_focus_graph(&controls, surface_size).unwrap().tab,
            vec![DialogFocusState::WarningButton]
        );
        let mut counters = NetCounters::default();
        assert!(handle_dialog_key_input(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
        ));
        assert!(world.current_dialog().is_none());
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
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap();
            dialog_input_layouts(&controls, surface_size).unwrap()[0]
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
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        assert_eq!(scroll_area.text_labels[0].text, "Enabled");
        assert!(scroll_area.background_layers.iter().any(|layer| {
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
        assert!(focused_screen
            .scroll_area
            .as_ref()
            .unwrap()
            .background_layers
            .iter()
            .any(|layer| {
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
                    "A very long speed control",
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
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap_or_else(|| panic!("invalid range dialog: {:?}", dialog.semantic_error));
            dialog_input_layouts(&controls, surface_size).unwrap()[0]
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
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        let range_label = scroll_area
            .text_labels
            .iter()
            .find(|label| label.text == "A very long speed control: 2")
            .unwrap();
        assert_eq!(
            range_label.scrolling,
            Some(HudInventoryScrollingText {
                x: input_layout.x + 2,
                y: input_layout.y,
                width: 96,
                height: 20,
            })
        );
        assert_eq!(
            scroll_area
                .background_layers
                .iter()
                .filter(|layer| layer.texture == HudInventoryBackgroundTexture::WidgetSlider)
                .count(),
            3
        );
        assert!(scroll_area.background_layers.iter().any(|layer| {
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
                .scroll_area
                .as_ref()
                .unwrap()
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
                .scroll_area
                .as_ref()
                .unwrap()
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
                    "A very long mode label",
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
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap();
            dialog_input_layouts(&controls, surface_size).unwrap()[0]
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
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        let option_label = scroll_area
            .text_labels
            .iter()
            .find(|label| label.text == "A very long mode label: Second")
            .unwrap();
        assert_eq!(
            option_label.scrolling,
            Some(HudInventoryScrollingText {
                x: input_layout.x + 2,
                y: input_layout.y,
                width: 96,
                height: 20,
            })
        );
        assert_eq!(
            scroll_area
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
                .scroll_area
                .as_ref()
                .unwrap()
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
                selected_index: 1,
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
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        assert!(scroll_area
            .text_labels
            .iter()
            .any(|label| label.text == "Alpha Display"));
        assert!(!scroll_area
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
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap();
            dialog_input_layouts(&controls, surface_size).unwrap()[0]
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
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        assert!(scroll_area.text_labels.iter().any(|label| {
            label.text == "Name" && (label.x, label.y) == (input_layout.x, input_layout.y)
        }));
        let text = scroll_area
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
            scroll_area
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
                scroll_amount: 0,
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
                scroll_amount: 0,
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
                .scroll_area
                .as_ref()
                .unwrap()
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
            .scroll_area
            .as_ref()
            .unwrap()
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
    fn dialog_ime_preedit_projects_from_the_focused_text_cursor() {
        let mut world = world_with_notice(vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input("name", "Name", 100, true, "ab", 32)]),
        )]);
        assert!(world.focus_dialog_text_input(0, 2, false));
        let surface_size = PhysicalSize::new(320, 240);
        let input_layout = {
            let controls = world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap();
            dialog_input_layouts(&controls, surface_size).unwrap()[0]
        };
        let box_layout = dialog_single_line_text_box_layout(input_layout, true);
        let preedit = DialogImePreeditHudState {
            text: "xy".to_string(),
            caret: Some(1),
            focused_start: 0,
            focused_end: 1,
            screen_revision: world.dialog_screen_revision(),
            caret_visible: true,
        };
        let screen = hud_dialog_screen(
            &world,
            world.current_dialog().unwrap(),
            None,
            &TerrainTextureState::default(),
            &InventoryHudLocalState {
                dialog_ime_preedit: Some(preedit.clone()),
                ..Default::default()
            },
            surface_size,
            0.0,
        )
        .unwrap();
        let overlay = screen.preedit_overlay.unwrap();
        let input_left = box_layout.x
            + TEXT_INPUT_INSET
            + i32::try_from(ApproximateDialogTextMeasurer.width("ab")).unwrap()
            + 1;
        let input_top = box_layout.y + TEXT_INPUT_Y_OFFSET;
        assert_eq!(overlay.text, "xy");
        assert_eq!((overlay.focused_start, overlay.focused_end), (0, 1));
        assert_eq!(overlay.caret, Some(1));
        assert!(overlay.caret_visible);
        assert_eq!(
            (overlay.text_x, overlay.text_y),
            (input_left, input_top + 14)
        );
        assert_eq!(
            (
                overlay.background_x,
                overlay.background_y,
                overlay.background_width,
                overlay.background_height,
            ),
            (input_left - 5, input_top + 9, 22, 19)
        );
        assert_eq!(
            dialog_ime_cursor_area(
                &world,
                Some(&preedit),
                surface_size,
                &ApproximateDialogTextMeasurer,
            ),
            Some((
                PhysicalPosition::new(input_left - 2, input_top - 2),
                PhysicalSize::new(16, 27),
            ))
        );

        let mut stale = preedit.clone();
        stale.screen_revision = stale.screen_revision.saturating_add(1);
        assert_eq!(
            dialog_ime_cursor_area(
                &world,
                Some(&stale),
                surface_size,
                &ApproximateDialogTextMeasurer,
            ),
            None
        );

        let mut overflowing = preedit;
        overflowing.text = "x".repeat(60);
        let overflow = dialog_preedit_projection(
            &world,
            &overflowing,
            surface_size,
            &ApproximateDialogTextMeasurer,
        )
        .unwrap();
        assert_eq!(
            overflow.overlay.text_x,
            i32::try_from(surface_size.width).unwrap()
                - i32::try_from(ApproximateDialogTextMeasurer.width(&overflowing.text)).unwrap()
        );
    }

    #[test]
    fn dialog_ime_preedit_uses_the_multiline_cursor_row() {
        let mut world = world_with_notice(vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![multiline_text_input(
                "notes",
                "Notes",
                100,
                false,
                "a\nb",
                32,
                None,
                Some(30),
            )]),
        )]);
        assert!(world.focus_dialog_text_input(0, 3, false));
        let surface_size = PhysicalSize::new(320, 240);
        let box_layout = {
            let controls = world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap();
            let input_layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
            dialog_multiline_text_box_layout(input_layout, false)
        };
        let preedit = DialogImePreeditHudState {
            text: "候选".to_string(),
            caret: None,
            focused_start: 0,
            focused_end: 2,
            screen_revision: world.dialog_screen_revision(),
            caret_visible: true,
        };
        let projection = dialog_preedit_projection(
            &world,
            &preedit,
            surface_size,
            &ApproximateDialogTextMeasurer,
        )
        .unwrap();

        assert_eq!(
            projection.overlay.text_x,
            box_layout.x
                + TEXT_INPUT_INSET
                + i32::try_from(ApproximateDialogTextMeasurer.width("b")).unwrap()
        );
        assert_eq!(
            projection.overlay.text_y,
            box_layout.y + TEXT_INPUT_INSET + TEXT_LINE_HEIGHT + 14
        );
    }

    #[test]
    fn multiline_text_renders_wraps_edits_scrolls_and_enforces_line_limit() {
        let mut world = world_with_notice(vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![multiline_text_input(
                "notes",
                "Notes",
                44,
                true,
                "ab cd\nx",
                64,
                Some(3),
                Some(17),
            )]),
        )]);
        let surface_size = PhysicalSize::new(320, 240);
        let (input_layout, box_layout) = {
            let controls = world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap();
            let input_layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
            (
                input_layout,
                dialog_multiline_text_box_layout(input_layout, true),
            )
        };
        let graph = dialog_focus_graph(
            &world
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(|data| dialog_controls(data, world.server_links()))
                .unwrap(),
            surface_size,
        )
        .unwrap();
        assert!(graph.tab.contains(&DialogFocusState::Input { index: 0 }));

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
        let scroll_area = screen.scroll_area.as_ref().unwrap();
        assert!(scroll_area.text_labels.iter().any(|label| {
            label.text == "Notes" && (label.x, label.y) == (input_layout.x, input_layout.y)
        }));
        assert_eq!(scroll_area.text_areas.len(), 1);
        let area = &scroll_area.text_areas[0];
        assert_eq!(
            (area.x, area.y, area.width, area.height, area.text.as_str()),
            (box_layout.x, box_layout.y, 44, 17, "ab cd\nx")
        );
        assert_eq!(area.scroll_amount, i32::MAX);
        assert_eq!(area.max_length, 64);
        assert_eq!(
            scroll_area
                .background_layers
                .iter()
                .filter(|layer| {
                    layer.texture == HudInventoryBackgroundTexture::WidgetTextField
                        && layer.x >= box_layout.x
                        && layer.y >= box_layout.y
                })
                .count(),
            9
        );

        assert!(world.focus_dialog_navigation_target(DialogFocusState::Input { index: 0 }, true));
        let mut counters = NetCounters::default();
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Home),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(world.dialog_text_edit_state().unwrap().cursor, 6);
        assert!(handle_dialog_text_input(&mut world, "§Y"));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "ab cd\nYx".to_string(),
            })]
        );
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "ab cd\nY\nx".to_string(),
            })]
        );
        assert!(world.dialog_text_edit_state().unwrap().scroll_amount > 0);

        let before = world.current_dialog_input_values().to_vec();
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(world.current_dialog_input_values(), before);
        let scrollbar_cursor = Some(PhysicalPosition::new(
            f64::from(box_layout.x + box_layout.width + 2),
            f64::from(box_layout.y + 4),
        ));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            scrollbar_cursor,
            surface_size,
        ));
        assert_eq!(world.dialog_dragging_multiline_scroll_input(), Some(0));
        assert!(handle_dialog_cursor_moved(
            &mut world,
            Some(PhysicalPosition::new(
                f64::from(box_layout.x + box_layout.width + 2),
                f64::from(box_layout.y + 14),
            )),
            surface_size,
        ));
        assert!(handle_dialog_mouse_input(
            &mut counters,
            &mut world,
            &None,
            None,
            MouseButton::Left,
            ElementState::Released,
            scrollbar_cursor,
            surface_size,
        ));
        assert_eq!(world.dialog_dragging_multiline_scroll_input(), None);
        assert!(handle_dialog_key_input_with_modifiers(
            &mut counters,
            &mut world,
            &None,
            None,
            PhysicalKey::Code(KeyCode::Backspace),
            ElementState::Pressed,
            false,
            false,
        ));
        assert_eq!(
            world.current_dialog_input_values(),
            [Some(DialogInputValueState::Text {
                value: "ab cd\nYx".to_string(),
            })]
        );
    }

    #[test]
    fn text_double_click_uses_vanilla_single_and_multiline_word_selection() {
        let surface_size = PhysicalSize::new(320, 240);
        let mut counters = NetCounters::default();
        let mut single = world_with_notice(vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![text_input(
                "name",
                "Name",
                120,
                false,
                "one two three",
                32,
            )]),
        )]);
        let single_layout = {
            let controls = single
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(|data| dialog_controls(data, single.server_links()))
                .unwrap();
            let layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
            dialog_single_line_text_box_layout(layout, false)
        };
        let single_first = Some(PhysicalPosition::new(
            f64::from(
                single_layout.x
                    + TEXT_INPUT_INSET
                    + i32::try_from(ApproximateDialogTextMeasurer.width("one t")).unwrap(),
            ),
            f64::from(single_layout.y + TEXT_INPUT_INSET),
        ));
        let single_second = Some(PhysicalPosition::new(
            f64::from(
                single_layout.x
                    + TEXT_INPUT_INSET
                    + i32::try_from(ApproximateDialogTextMeasurer.width("one two th")).unwrap(),
            ),
            f64::from(single_layout.y + TEXT_INPUT_INSET),
        ));
        assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
            &mut counters,
            &mut single,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            single_first,
            surface_size,
            false,
            false,
            &ApproximateDialogTextMeasurer,
        ));
        assert_eq!(single.dialog_text_edit_state().unwrap().cursor, 5);
        assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
            &mut counters,
            &mut single,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            single_second,
            surface_size,
            true,
            true,
            &ApproximateDialogTextMeasurer,
        ));
        assert_eq!(
            single.dialog_text_edit_state(),
            Some(bbb_world::DialogTextEditState {
                input_index: 0,
                cursor: 13,
                selection: 8,
                scroll_amount: 0,
            })
        );
        assert_eq!(single.dialog_dragging_text_input(), Some(0));

        let mut multiline = world_with_notice(vec![entry(
            "inputs",
            NbtSummaryValue::List(vec![multiline_text_input(
                "notes",
                "Notes",
                120,
                false,
                "one two\nthree",
                64,
                None,
                Some(30),
            )]),
        )]);
        let multiline_layout = {
            let controls = multiline
                .current_dialog()
                .and_then(|dialog| dialog.data.as_ref())
                .and_then(|data| dialog_controls(data, multiline.server_links()))
                .unwrap();
            let layout = dialog_input_layouts(&controls, surface_size).unwrap()[0];
            dialog_multiline_text_box_layout(layout, false)
        };
        let multiline_first = Some(PhysicalPosition::new(
            f64::from(
                multiline_layout.x
                    + TEXT_INPUT_INSET
                    + i32::try_from(ApproximateDialogTextMeasurer.width("one t")).unwrap(),
            ),
            f64::from(multiline_layout.y + TEXT_INPUT_INSET),
        ));
        let multiline_second = Some(PhysicalPosition::new(
            f64::from(
                multiline_layout.x
                    + TEXT_INPUT_INSET
                    + i32::try_from(ApproximateDialogTextMeasurer.width("thr")).unwrap(),
            ),
            f64::from(multiline_layout.y + TEXT_INPUT_INSET + TEXT_LINE_HEIGHT),
        ));
        assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
            &mut counters,
            &mut multiline,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            multiline_first,
            surface_size,
            false,
            false,
            &ApproximateDialogTextMeasurer,
        ));
        assert_eq!(multiline.dialog_text_edit_state().unwrap().cursor, 5);
        assert!(handle_dialog_mouse_input_with_modifiers_and_measurer(
            &mut counters,
            &mut multiline,
            &None,
            None,
            MouseButton::Left,
            ElementState::Pressed,
            multiline_second,
            surface_size,
            true,
            true,
            &ApproximateDialogTextMeasurer,
        ));
        assert_eq!(
            multiline.dialog_text_edit_state(),
            Some(bbb_world::DialogTextEditState {
                input_index: 0,
                cursor: 7,
                selection: 4,
                scroll_amount: 0,
            })
        );
        assert_eq!(multiline.dialog_dragging_text_input(), Some(0));
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

    fn plain_message_body(contents: NbtSummaryValue, width: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:plain_message".to_string()),
            ),
            entry("contents", contents),
            entry("width", NbtSummaryValue::Int(width)),
        ])
    }

    fn clickable_component(
        text: &str,
        action: &str,
        mut action_fields: Vec<NbtSummaryEntry>,
    ) -> NbtSummaryValue {
        let mut click_event = vec![entry("action", NbtSummaryValue::String(action.to_string()))];
        click_event.append(&mut action_fields);
        NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::String(text.to_string())),
            entry("click_event", NbtSummaryValue::Compound(click_event)),
        ])
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

    #[allow(clippy::too_many_arguments)]
    fn multiline_text_input(
        key: &str,
        label: &str,
        width: i32,
        label_visible: bool,
        initial: &str,
        max_length: i32,
        max_lines: Option<i32>,
        height: Option<i32>,
    ) -> NbtSummaryValue {
        let mut multiline = Vec::new();
        if let Some(max_lines) = max_lines {
            multiline.push(entry("max_lines", NbtSummaryValue::Int(max_lines)));
        }
        if let Some(height) = height {
            multiline.push(entry("height", NbtSummaryValue::Int(height)));
        }
        let NbtSummaryValue::Compound(mut fields) =
            text_input(key, label, width, label_visible, initial, max_length)
        else {
            unreachable!("text input helper always returns a compound")
        };
        fields.push(entry("multiline", NbtSummaryValue::Compound(multiline)));
        NbtSummaryValue::Compound(fields)
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

    fn world_with_multi_action(mut fields: Vec<NbtSummaryEntry>) -> WorldStore {
        let mut dialog = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:multi_action".to_string()),
            ),
            entry("title", NbtSummaryValue::String("Actions".to_string())),
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

    fn world_with_dialog_list(mut fields: Vec<NbtSummaryEntry>, context_free: bool) -> WorldStore {
        let mut dialog = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:dialog_list".to_string()),
            ),
            entry("title", NbtSummaryValue::String("Menu".to_string())),
        ];
        dialog.append(&mut fields);
        let dialog = NbtSummaryValue::Compound(dialog);
        let mut world = WorldStore::new();
        world.apply_show_dialog(ShowDialog {
            dialog: if context_free {
                DialogHolder::ContextFree { dialog }
            } else {
                DialogHolder::Direct { dialog }
            },
        });
        world
    }

    fn world_with_server_links_dialog(
        links: Vec<ServerLinkEntry>,
        mut fields: Vec<NbtSummaryEntry>,
    ) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_server_links(ServerLinks { links });
        let mut dialog = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:server_links".to_string()),
            ),
            entry("title", NbtSummaryValue::String("Links".to_string())),
        ];
        dialog.append(&mut fields);
        world.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(dialog),
            },
        });
        world
    }

    fn notice_dialog(title: &str, external_title: Option<&str>) -> NbtSummaryValue {
        let mut dialog = vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:notice".to_string()),
            ),
            entry("title", NbtSummaryValue::String(title.to_string())),
        ];
        if let Some(external_title) = external_title {
            dialog.push(entry(
                "external_title",
                NbtSummaryValue::String(external_title.to_string()),
            ));
        }
        NbtSummaryValue::Compound(dialog)
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }
}
