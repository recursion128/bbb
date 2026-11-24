use crate::input::dialog::DialogFocusState;
use std::collections::BTreeMap;

mod actions;
mod input;
mod layout;
mod render;
mod text_raw;
mod widgets;
use actions::*;
pub(crate) use input::*;
use layout::*;
pub(crate) use render::*;
use text_raw::*;
use widgets::*;

use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::item_model::NativeItemRuntime;
#[cfg(test)]
use bbb_client_render::HudTextStyle;
use bbb_client_render::{
    HudComponentTextRun, HudInventoryBackgroundLayer, HudInventoryBackgroundTexture,
    HudInventoryFillLayer, HudInventoryFillStage, HudInventoryItem, HudInventoryItemScissor,
    HudInventoryPreeditOverlay, HudInventoryScreen, HudInventoryScrollArea, HudInventoryScrollbar,
    HudInventoryScrollingText, HudInventoryTextArea, HudInventoryTextInputDecoration,
    HudInventoryTextLabel, HudInventoryTooltip, HudInventoryTooltipLine, HudStyledTextRun,
    HudUvRect, Renderer,
};
use bbb_client_world::{
    CommonDialogDataState, DialogActionState, DialogBodyState, DialogButtonState,
    DialogCommandTemplateState, DialogDataState, DialogInputControlState,
    DialogItemStackTemplateState, DialogNumberRangeState, DialogPlainMessageState, DialogState,
    DialogTextMultilineState, DialogTextState, ServerLinkState, WorldStore,
};
use bbb_protocol::{
    java_float_to_string,
    packets::{CustomClickAction, GameType, ItemStackSummary},
    ComponentClickEvent, ComponentStyle, NbtSummaryValue, StyledTextRun,
};
use tokio::sync::mpsc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::input::{
    text_field::{
        caret_visible, clock_millis, MultilineTextField, TextField, TextFieldModifiers,
        TextLineFont,
    },
    DebugClipboard, DialogImePreeditHudState,
};
use bbb_client_render::scene::terrain_textures::TerrainTextureState;

use super::{hud_ascii_approx_char_width, hud_item_tooltip, HudItemModelContext, HudItemSlotState};

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
    fn glyphs(&self) -> &bbb_client_render::HudFontGlyphMap;

    fn width(&self, text: &str) -> i32;

    fn display_start(&self, text: &str, scroll_to: usize, width: i32) -> usize;

    fn cursor_for_width(&self, text: &str, display_start: usize, width: i32) -> usize;
}

impl DialogTextMeasurer for Renderer {
    fn glyphs(&self) -> &bbb_client_render::HudFontGlyphMap {
        self.hud_text_measurements_ref()
    }

    fn width(&self, text: &str) -> i32 {
        self.hud_raw_string_width(text)
    }

    fn display_start(&self, text: &str, scroll_to: usize, width: i32) -> usize {
        self.hud_plain_text_display_start_for_width(text, scroll_to, width)
            .unwrap_or_else(|| approximate_dialog_text_display_start(text, scroll_to, width))
    }

    fn cursor_for_width(&self, text: &str, display_start: usize, width: i32) -> usize {
        self.hud_plain_text_cursor_for_width_from(text, display_start, width)
            .unwrap_or_else(|| approximate_dialog_text_cursor(text, display_start, width))
    }
}

#[derive(Clone, Copy)]
pub(super) struct DialogTextContext<'a> {
    pub(super) item_runtime: Option<&'a NativeItemRuntime>,
    pub(super) glyphs: &'a bbb_client_render::HudFontGlyphMap,
}

impl<'a> DialogTextContext<'a> {
    pub(super) fn from_measurer(
        item_runtime: Option<&'a NativeItemRuntime>,
        text_measurer: &'a dyn DialogTextMeasurer,
    ) -> Self {
        Self {
            item_runtime,
            glyphs: text_measurer.glyphs(),
        }
    }

    pub(super) fn approximate(item_runtime: Option<&'a NativeItemRuntime>) -> Self {
        Self {
            item_runtime,
            glyphs: ApproximateDialogTextMeasurer.glyphs(),
        }
    }
}

pub(super) struct ApproximateDialogTextMeasurer;

impl DialogTextMeasurer for ApproximateDialogTextMeasurer {
    fn glyphs(&self) -> &bbb_client_render::HudFontGlyphMap {
        static GLYPHS: std::sync::OnceLock<bbb_client_render::HudFontGlyphMap> =
            std::sync::OnceLock::new();
        GLYPHS.get_or_init(|| {
            let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
            glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(6.0));
            for ch in ' '..='~' {
                glyphs.insert_first_wins(
                    ch,
                    bbb_client_render::HudAsciiGlyph::empty(hud_ascii_approx_char_width(ch) as f32),
                );
            }
            glyphs
        })
    }

    fn width(&self, text: &str) -> i32 {
        text.chars().fold(0, |width, ch| {
            width.saturating_add(hud_ascii_approx_char_width(ch) as i32)
        })
    }

    fn display_start(&self, text: &str, scroll_to: usize, width: i32) -> usize {
        approximate_dialog_text_display_start(text, scroll_to, width)
    }

    fn cursor_for_width(&self, text: &str, display_start: usize, width: i32) -> usize {
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
    stack: std::sync::Arc<ItemStackSummary>,
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

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct DialogHudLocalState {
    pub(super) dialog_ime_preedit: Option<crate::input::DialogImePreeditHudState>,
    pub(super) cursor_position: Option<(i32, i32)>,
    pub(super) shift_down: bool,
    pub(super) keybind_context: bbb_client_render::item_model::ItemModelKeybindContext,
    pub(super) advanced_item_tooltips: bool,
}
