use super::*;
use bbb_protocol::game_types::GameType;
use std::sync::Arc;

/// One accepted chat entry projected from the immutable world generation.
/// The renderer keeps these stable source runs and rebuilds its bounded
/// wrapped-line cache only when native reports a new delivery/resource key.
#[derive(Debug, Clone, PartialEq)]
pub struct HudChatMessage {
    pub display_sequence: u64,
    pub added_tick: u64,
    pub runs: Vec<HudComponentTextRun>,
    pub interactions: Vec<HudChatInteraction>,
    pub tag: Option<HudChatTag>,
}

/// Reusable Component/item hover tooltip content. Lines retain their own base
/// tint and per-run font selection; the renderer owns wrapping, placement, and
/// the advanced-tooltip visibility gate.
#[derive(Debug, Clone, PartialEq)]
pub struct HudRichTooltip {
    pub style: Option<String>,
    pub lines: Vec<HudRichTooltipLine>,
    /// Optional replacement lines selected while advanced item tooltips are
    /// enabled. Item hovers use this for advanced details; entity hovers keep
    /// `lines` empty and expose their entire tooltip here.
    pub advanced_lines: Option<Vec<HudRichTooltipLine>>,
    pub wrap_text: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudRichTooltipLine {
    pub runs: Vec<HudComponentTextRun>,
    pub tint: [f32; 4],
}

impl HudRichTooltipLine {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            runs: vec![HudComponentTextRun::plain(text)],
            tint: HUD_TINT_WHITE,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudChatInteraction {
    pub start_char: usize,
    pub end_char: usize,
    pub target_index: usize,
    /// The canonical component run carries a `Style.click_event`; only such a
    /// run answers an unmodified click (`ChatScreen.handleComponentClicked`
    /// ignores insertions unless shift is held).
    pub clickable: bool,
    /// The canonical component run carries a `Style.insertion`; the renderer
    /// only needs targetability and never retains the insertion payload.
    pub insertion: bool,
    pub tooltip: Option<HudRichTooltip>,
}

/// Renderer vocabulary for Vanilla `GuiMessageTag`. The color is the exact
/// two-pixel indicator color; only Modified carries the 9x9 sprite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudChatTag {
    pub kind: HudChatTagKind,
    pub indicator_color: u32,
    pub tooltip_runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudChatTagKind {
    System,
    SystemSinglePlayer,
    NotSecure,
    Modified,
    Error,
}

/// Per-frame values which do not invalidate stable rich-message wrapping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudChatContext {
    pub level_generation: u64,
    pub delivery_revision: u64,
    pub chat_screen_revision: u64,
    pub resource_generation: u64,
    pub resources_loaded: bool,
    pub gui_ticks: u64,
    pub focused: bool,
    pub advanced_item_tooltips: bool,
    pub scroll: HudChatScroll,
    pub cursor_position: Option<(i32, i32)>,
    pub queued_messages: usize,
    pub queue_text: Option<String>,
    pub queue_tooltip_text: Option<String>,
    pub restricted_text: Option<String>,
}

/// Vanilla `RestrictionsScreen` projection reached from the focused chat
/// restriction prompt. The canonical permission decision remains native;
/// renderer owns only bounded text, layout, and hit testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudChatRestrictionsScreen {
    pub generation: u64,
    pub title: String,
    pub reason: HudStyledTextRun,
    pub permissions: Vec<HudStyledTextRun>,
    pub done: String,
    pub cursor_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HudChatLine {
    pub(super) display_sequence: u64,
    pub(super) added_tick: u64,
    pub(super) runs: Vec<HudComponentTextRun>,
    pub(super) cached_geometry: Option<HudChatLineGeometry>,
    pub(super) tag: Option<HudChatTag>,
    /// Interaction target per logical character of [`Self::runs`]; the glyph
    /// geometry indexes the same flattened character sequence.
    pub(super) targets: Vec<Option<usize>>,
    pub(super) interactions: Vec<HudChatLineInteraction>,
    pub(super) end_of_entry: bool,
}

/// One line's interaction targets, in the order the source message lists them.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HudChatLineInteraction {
    pub(super) target_index: usize,
    pub(super) clickable: bool,
    pub(super) insertion: bool,
    pub(super) tooltip: Option<HudRichTooltip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudChatHitTarget {
    pub level_generation: u64,
    pub delivery_revision: u64,
    pub chat_screen_revision: u64,
    pub resource_generation: u64,
    pub resources_loaded: bool,
    pub kind: HudChatHitTargetKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudChatHitTargetKind {
    Queue,
    Restrictions,
    RestrictionsDone,
    Message {
        display_sequence: u64,
        /// The last hit target with a click event or an insertion, the scan
        /// `ClickableStyleFinder.includeInsertions(true)` performs while shift
        /// is held.
        target_index: usize,
        /// The last hit target with a click event, the scan an unmodified click
        /// consumes (`includeInsertions(false)`); `None` when that scan found
        /// nothing, so the click belongs to the edit box.
        click_target_index: Option<usize>,
    },
}

/// UTF-16 positions may point between the units of a surrogate pair.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudChatInputView {
    pub value: Vec<u16>,
    pub cursor: usize,
    pub selection: usize,
    pub display_start: usize,
}

/// One frame of the unbordered ChatScreen edit box and its Brigadier
/// completion popup. Coordinates are absolute GUI pixels, matching vanilla's
/// `CommandSuggestions.SuggestionsList` extraction.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudCommandSuggestions {
    pub preedit: Option<HudImePreedit>,
    pub input_view: Option<HudChatInputView>,
    pub input_text: String,
    pub draft: bool,
    /// `Some` only while the input is a command the local parser accepted or
    /// rejected, matching `CommandSuggestions.currentParse`; `None` draws the
    /// plain edit-box colour.
    pub syntax: Option<HudCommandInputSyntax>,
    pub cursor: usize,
    pub selection: usize,
    pub cursor_visible: bool,
    pub suffix: Option<String>,
    pub list_x: i32,
    pub list_y: i32,
    pub list_width: u32,
    pub list_height: u32,
    pub entries: Vec<HudCommandSuggestionEntry>,
    pub has_previous: bool,
    pub has_next: bool,
    pub tooltip: Option<HudCommandSuggestionTooltip>,
    pub layout: HudCommandSuggestionsLayout,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HudCommandSuggestionsLayout {
    #[default]
    Chat,
    CommandBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudCommandSuggestionTooltip {
    pub runs: Vec<HudStyledTextRun>,
    pub x: i32,
    pub y: i32,
}

/// One frame's F3 debug overlay text columns. Vanilla `DebugScreenOverlay`
/// resolves enabled entries into left/right line lists, then draws each
/// non-empty line with a translucent black backdrop at 2px margins and 9px
/// row stride.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudDebugOverlay {
    pub left_lines: Vec<String>,
    pub right_lines: Vec<String>,
    pub debug_crosshair: Option<HudDebugCrosshair>,
    pub game_mode_switcher: Option<HudDebugGameModeSwitcher>,
    pub profiler_chart: Option<HudDebugProfilerChart>,
    pub fps_chart: Option<HudDebugFrameTimeChart>,
    pub tps_chart: Option<HudDebugTpsChart>,
    pub network_charts: Option<HudDebugNetworkCharts>,
    pub show_lightmap_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudPauseScreenTooltip {
    pub text: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudStatsRow {
    pub label: String,
    pub value: String,
    pub secondary_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudStatsItemRow {
    pub item_id: i32,
    pub resource_id: String,
    pub label: String,
    pub values: [Option<String>; 6],
    pub icon: Option<HudItemIcon>,
    pub block_model: Option<HudBlockItemModel>,
}

/// Vanilla `StatsScreen` GUI-atlas sprites. Keeping the asset identity in one typed registry lets
/// pack loading and draw dispatch share the same fixed 26.1 catalog without stringly-typed slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudStatsSprite {
    Slot,
    Header,
    SortUp,
    SortDown,
    BlockMined,
    ItemBroken,
    ItemCrafted,
    ItemUsed,
    ItemPickedUp,
    ItemDropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudStatsScrollbarSprite {
    Scroller,
    Background,
}

impl HudStatsScrollbarSprite {
    pub(crate) const COUNT: usize = 2;
    pub const ALL: [Self; Self::COUNT] = [Self::Scroller, Self::Background];

    pub fn sprite_path(self) -> &'static str {
        match self {
            Self::Scroller => "widget/scroller",
            Self::Background => "widget/scroller_background",
        }
    }
}

impl HudStatsSprite {
    pub(crate) const COUNT: usize = 10;

    pub const ALL: [Self; Self::COUNT] = [
        Self::Slot,
        Self::Header,
        Self::SortUp,
        Self::SortDown,
        Self::BlockMined,
        Self::ItemBroken,
        Self::ItemCrafted,
        Self::ItemUsed,
        Self::ItemPickedUp,
        Self::ItemDropped,
    ];

    pub fn sprite_path(self) -> &'static str {
        match self {
            Self::Slot => "container/slot",
            Self::Header => "statistics/header",
            Self::SortUp => "statistics/sort_up",
            Self::SortDown => "statistics/sort_down",
            Self::BlockMined => "statistics/block_mined",
            Self::ItemBroken => "statistics/item_broken",
            Self::ItemCrafted => "statistics/item_crafted",
            Self::ItemUsed => "statistics/item_used",
            Self::ItemPickedUp => "statistics/item_picked_up",
            Self::ItemDropped => "statistics/item_dropped",
        }
    }

    pub(super) const fn column(column: usize) -> Option<Self> {
        match column {
            0 => Some(Self::BlockMined),
            1 => Some(Self::ItemBroken),
            2 => Some(Self::ItemCrafted),
            3 => Some(Self::ItemUsed),
            4 => Some(Self::ItemPickedUp),
            5 => Some(Self::ItemDropped),
            _ => None,
        }
    }
}

/// Vanilla `StatsScreen` projection. Lists become visible only after the requested AwardStats
/// response; each empty tab remains disabled like vanilla.
#[derive(Debug, Clone, PartialEq)]
pub struct HudStatsScreen {
    pub title: String,
    pub loading_text: String,
    pub is_loading: bool,
    pub general_tab: String,
    pub items_tab: String,
    pub mobs_tab: String,
    pub active_tab: usize,
    pub items_tab_enabled: bool,
    pub mobs_tab_enabled: bool,
    pub rows: Vec<HudStatsRow>,
    pub item_rows: Vec<HudStatsItemRow>,
    pub item_column_labels: [String; 6],
    pub item_no_value: String,
    pub item_sort_column: Option<usize>,
    pub item_sort_order: i8,
    pub item_hovered_column: Option<usize>,
    pub item_focused_column: Option<usize>,
    /// First projected data-row index (the Items header is a separate scrollable entry).
    pub scroll_row: usize,
    /// Integer-pixel list scroll after vanilla's `(int) scrollAmount` truncation.
    pub scroll_amount: i32,
    /// Maximum integer-pixel scroll for the current content and viewport.
    pub max_scroll_amount: i32,
    /// Vanilla scrollbar thumb height, or zero when the list is not scrollable.
    pub scroller_height: i32,
    pub total_rows: usize,
    pub visible_rows: usize,
    pub row_height: i32,
    pub tooltip: Option<HudDebugOptionsTooltip>,
    pub done: String,
    pub done_hovered: bool,
    pub done_focused: bool,
}

/// One system toast admitted to Vanilla's shared five-slot toast area.
#[derive(Debug, Clone, PartialEq)]
pub struct HudSystemToastInstance {
    pub toast: HudSystemToast,
    pub slot: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudNowPlayingToastPlacement {
    SlidingToast,
    PauseScreen,
}

/// One visible Vanilla advancement toast. Native owns queue/slot/timing and
/// item resolution; the renderer owns exact font wrapping and draw order.
#[derive(Debug, Clone, PartialEq)]
pub struct HudAdvancementToast {
    pub frame_label: String,
    pub frame_label_color: u32,
    pub title_runs: Vec<HudStyledTextRun>,
    pub icon: HudItemIcon,
    pub block_model: Option<HudBlockItemModel>,
    pub slot: u8,
    pub visible_portion: f32,
    pub fully_visible_ms: u64,
}

/// One visible Vanilla recipe toast. Native owns singleton coalescing,
/// item cycling, queue/slot/timing, and item resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct HudRecipeToast {
    pub title: String,
    pub description: String,
    pub category_icon: Option<HudItemIcon>,
    pub category_block_model: Option<HudBlockItemModel>,
    pub unlocked_icon: Option<HudItemIcon>,
    pub unlocked_block_model: Option<HudBlockItemModel>,
    pub slot: u8,
    pub visible_portion: f32,
}

/// Sanitize wrapped content once so native can share it across frame projections.
#[derive(Debug, Clone, PartialEq)]
pub struct HudTutorialToastLines(Arc<[Vec<HudStyledTextRun>]>);

impl HudTutorialToastLines {
    pub fn from_wrapped(lines: Vec<Vec<HudStyledTextRun>>) -> Self {
        Self(super::sanitize::sanitize_hud_tutorial_toast_lines(lines).into())
    }

    pub fn as_slice(&self) -> &[Vec<HudStyledTextRun>] {
        &self.0
    }

    pub fn height(&self) -> u32 {
        tutorial_toast_height(self.0.len())
    }

    pub fn occupied_slots(&self) -> usize {
        usize::try_from(self.height().div_ceil(HUD_ADVANCEMENT_TOAST_HEIGHT)).unwrap_or(usize::MAX)
    }
}

/// One visible Vanilla tutorial toast. Native owns the shared queue, slot
/// allocation, localization, and progress smoothing; the renderer owns the
/// sprite-backed variable-height draw.
#[derive(Debug, Clone, PartialEq)]
pub struct HudTutorialToast {
    pub icon: HudTutorialToastIcon,
    pub lines: HudTutorialToastLines,
    pub slot: u8,
    pub visible_portion: f32,
    pub progress: Option<f32>,
    pub progress_increasing: bool,
}

impl HudTutorialToastIcon {
    pub(super) const fn sprite_path(self) -> &'static str {
        match self {
            Self::MovementKeys => "toast/movement_keys",
            Self::Mouse => "toast/mouse",
            Self::Tree => "toast/tree",
            Self::RecipeBook => "toast/recipe_book",
            Self::WoodenPlanks => "toast/wooden_planks",
            Self::SocialInteractions => "toast/social_interactions",
        }
    }
}

pub const HUD_TOAST_SLOTS: usize = 5;

/// Vanilla `DebugOptionsScreen` render-state shell: the searchable option list
/// in vanilla category/path order, three status buttons per entry, and the
/// default/performance/done footer buttons. The renderer derives layout from
/// the fixed vanilla row metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudDebugOptionsScreen {
    pub title: String,
    pub warning: String,
    pub search: HudTextField,
    pub search_hint: String,
    pub never_label: String,
    pub overlay_label: String,
    pub always_label: String,
    pub rows: Vec<HudDebugOptionsRow>,
    pub tooltip: Option<HudDebugOptionsTooltip>,
    pub scroll_row: usize,
    pub total_rows: usize,
    pub visible_rows: usize,
    pub default_profile_active: bool,
    pub default_profile_hovered: bool,
    pub default_profile_focused: bool,
    pub default_profile_label: String,
    pub performance_profile_active: bool,
    pub performance_profile_hovered: bool,
    pub performance_profile_focused: bool,
    pub performance_profile_label: String,
    pub done_hovered: bool,
    pub done_focused: bool,
    pub done_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudDebugOptionsTooltip {
    pub text: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy)]
pub(super) struct HudDebugOptionsButtonSprites<'a> {
    pub(super) normal: Option<&'a HudSpriteGpu>,
    pub(super) disabled: Option<&'a HudSpriteGpu>,
    pub(super) highlighted: Option<&'a HudSpriteGpu>,
}

#[derive(Clone, Copy)]
pub(super) struct HudJigsawSliderSprites<'a> {
    pub(super) normal: Option<&'a HudSpriteGpu>,
    pub(super) highlighted: Option<&'a HudSpriteGpu>,
    pub(super) handle: Option<&'a HudSpriteGpu>,
    pub(super) handle_highlighted: Option<&'a HudSpriteGpu>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HudDebugOptionsButtonSpriteSlot {
    Normal,
    Disabled,
    Highlighted,
}

impl<'a> HudDebugOptionsButtonSprites<'a> {
    pub(super) fn widget(sprites: &'a GuiSprites) -> Self {
        Self {
            normal: sprites.get("widget/button"),
            disabled: sprites.get("widget/button_disabled"),
            highlighted: sprites.get("widget/button_highlighted"),
        }
    }

    pub(super) fn get(self, active: bool, highlighted: bool) -> Option<&'a HudSpriteGpu> {
        match hud_debug_options_button_sprite_slot(active, highlighted) {
            HudDebugOptionsButtonSpriteSlot::Normal => self.normal,
            HudDebugOptionsButtonSpriteSlot::Disabled => self.disabled.or(self.normal),
            HudDebugOptionsButtonSpriteSlot::Highlighted => self.highlighted.or(self.normal),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudDebugOptionsScrollbarRect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HudDebugOptionsRow {
    Category {
        label: String,
    },
    Entry {
        path: String,
        status: HudDebugOptionsEntryStatus,
        hovered_status: Option<HudDebugOptionsEntryStatus>,
        focused_status: Option<HudDebugOptionsEntryStatus>,
        allowed: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudDebugOptionsEntryStatus {
    AlwaysOn,
    InOverlay,
    Never,
}

/// Vanilla `GameModeSwitcherScreen` render-state shell: background bounds, four
/// 26px slots in vanilla order, selected mode, and the two centered text rows.
///
/// The hint row is a Component whose one argument carries its own colour
/// so it arrives as resolved runs.
#[derive(Debug, Clone, PartialEq)]
pub struct HudDebugGameModeSwitcher {
    pub selected: GameType,
    pub title: String,
    pub help_runs: Vec<HudStyledTextRun>,
    pub background_x: i32,
    pub background_y: i32,
    pub background_width: i32,
    pub background_height: i32,
    pub slots: Vec<HudDebugGameModeSwitcherSlot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudDebugGameModeSwitcherSlot {
    pub mode: GameType,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub selected: bool,
    pub icon: Option<HudItemIcon>,
    pub block_model: Option<HudBlockItemModel>,
}

/// Vanilla `DebugScreenEntries.THREE_DIMENSIONAL_CROSSHAIR`: a camera-relative
/// 3-axis debug gizmo rendered at the screen center while the entry is enabled.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HudDebugCrosshair {
    pub x_rot_degrees: f32,
    pub y_rot_degrees: f32,
    pub gui_scale: u32,
}

/// Vanilla `FpsDebugChart` sample stream: frame durations in nanoseconds,
/// oldest first, capped to `LocalSampleLogger.CAPACITY`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudDebugFrameTimeChart {
    pub frame_time_nanos: Vec<u64>,
    pub configured_framerate_limit: Option<u32>,
}

/// Vanilla `TpsDebugChart` sample stream. Each sample stores the
/// `TpsDebugDimensions` values in nanoseconds: full tick, server tick method,
/// scheduled tasks, and idle time. The renderer derives the aggregate
/// full-minus-idle value for labels, matching vanilla.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudDebugTpsChart {
    pub samples: Vec<HudDebugTpsSample>,
    pub milliseconds_per_tick: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudDebugTpsSample {
    pub full_tick_nanos: u64,
    pub tick_server_method_nanos: u64,
    pub scheduled_tasks_nanos: u64,
    pub idle_nanos: u64,
}

/// Vanilla `PingDebugChart` + `BandwidthDebugChart` sample streams. Bandwidth
/// samples are received bytes per 50ms client tick and are displayed as
/// bytes/second by the renderer, matching vanilla's `BandwidthDebugChart`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudDebugNetworkCharts {
    pub ping_millis: Vec<u64>,
    pub bandwidth_bytes_per_tick: Vec<u64>,
    pub show_bandwidth: bool,
}

/// Vanilla `ProfilerPieChart` render state after `ProfileResults.getTimes(path)`:
/// the current node is the first `ResultField`, and `slices` are the remaining
/// direct children. Slice colors are derived from the names with vanilla
/// `ResultField.getColor()`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudDebugProfilerChart {
    pub current_node_name: String,
    pub current_global_percentage: f64,
    pub slices: Vec<HudDebugProfilerSlice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudDebugProfilerSlice {
    pub name: String,
    pub percentage: f64,
    pub global_percentage: f64,
}

/// One pitch cue beside a locator-bar waypoint dot
/// (`TrackedWaypoint.PitchDirection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudLocatorPitch {
    None,
    Up,
    Down,
}

/// One visible tracked-waypoint projection. Native extraction owns camera and
/// canonical-world math; the renderer owns style/sprite selection and exact
/// pixel geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct HudLocatorMarker {
    pub angle_degrees: f32,
    pub pitch: HudLocatorPitch,
    pub distance: f32,
    pub style_id: String,
    pub color_argb: u32,
}

/// The farthest-first visible marker list for Vanilla's locator contextual bar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudLocatorBar {
    pub markers: Vec<HudLocatorMarker>,
    pub client_tick: u64,
}

impl HudLocatorStyle {
    pub fn sprite_id_for_distance(&self, distance: f32) -> Option<&str> {
        let first = self.sprite_ids.first()?;
        if distance < self.near_distance as f32 {
            return Some(first);
        }
        let last = self.sprite_ids.last()?;
        if distance >= self.far_distance as f32 {
            return Some(last);
        }
        if self.sprite_ids.len() == 1 {
            return Some(first);
        }
        if self.sprite_ids.len() == 3 {
            return self.sprite_ids.get(1).map(String::as_str);
        }
        let range = self.far_distance.saturating_sub(self.near_distance);
        if range == 0 {
            return Some(last);
        }
        let alpha = (distance - self.near_distance as f32) / range as f32;
        let index = 1
            + (alpha * (self.sprite_ids.len() as f32 - 2.0))
                .floor()
                .max(0.0) as usize;
        self.sprite_ids
            .get(index.min(self.sprite_ids.len() - 1))
            .map(String::as_str)
    }
}

pub(super) const HUD_INVENTORY_EFFECT_SPACING: i32 = 7;

pub(super) const HUD_INVENTORY_EFFECT_SQUARE_SIZE: u32 = 32;

pub(super) const HUD_INVENTORY_EFFECT_TEXT_X_OFFSET: i32 = 32;

pub(super) const HUD_INVENTORY_EFFECT_DEFAULT_Y_STEP: i32 = 33;

pub(super) const HUD_INVENTORY_EFFECT_TOTAL_HEIGHT: i32 = 132;

pub(super) const HUD_INVENTORY_EFFECT_WIDE_THRESHOLD: i32 = 120;

pub(super) const HUD_INVENTORY_EFFECT_TEXT_GRAY: [f32; 4] =
    [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0];

pub(super) const HUD_COMMAND_INPUT_TINT: [f32; 4] =
    [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0];

/// `CommandSuggestions` styles, as the `ChatFormatting` colours they
/// resolve to: five cycling argument colours plus
/// the literal and unparsed ones.
pub(super) const HUD_COMMAND_ARGUMENT_COLORS: [u32; 5] =
    [0x55FFFF, 0xFFFF55, 0x55FF55, 0xFF55FF, 0xFFAA00];

pub(super) const HUD_COMMAND_UNPARSED_COLOR: u32 = 0xFF5555;

pub(super) const HUD_COMMAND_LITERAL_COLOR: u32 = 0xAAAAAA;

pub(super) const HUD_COMMAND_SUFFIX_TINT: [f32; 4] =
    [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0];

pub(super) const HUD_COMMAND_UNSELECTED_TINT: [f32; 4] =
    [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0];

pub(super) const HUD_COMMAND_SELECTED_TINT: [f32; 4] = [1.0, 1.0, 0.0, 1.0];

pub(super) const HUD_COMMAND_INPUT_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 128.0 / 255.0];

pub(super) const HUD_COMMAND_LIST_BACKGROUND_TINT: [f32; 4] = [0.0, 0.0, 0.0, 208.0 / 255.0];

pub(super) const HUD_COMMAND_SELECTION_TINT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

pub(super) const HUD_COMMAND_LINE_HEIGHT: i32 = 12;

pub(super) const HUD_DEBUG_OVERLAY_TEXT_TINT: [f32; 4] =
    [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0];

pub(super) const HUD_DEBUG_OVERLAY_BACKGROUND_TINT: [f32; 4] =
    [80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 144.0 / 255.0];

pub(super) const HUD_DEBUG_OVERLAY_MARGIN_X: i32 = 2;

pub(super) const HUD_DEBUG_OVERLAY_MARGIN_Y: i32 = 2;

pub(super) const HUD_DEBUG_OVERLAY_LINE_HEIGHT: i32 = 9;

pub(super) const HUD_DEBUG_CHART_SAMPLE_CAPACITY: usize = 240;

pub(super) const HUD_DEBUG_CHART_HEIGHT: i32 = 60;

pub(super) const HUD_DEBUG_CHART_LABEL_HEIGHT: i32 = 9;

pub(super) const HUD_DEBUG_FPS_CHART_TOP_VALUE_MS: f64 = 33.333333333333336;

pub(super) const HUD_DEBUG_PROFILER_WIDTH: i32 = 260;

pub(super) const HUD_DEBUG_PROFILER_HALF_WIDTH: i32 = HUD_DEBUG_PROFILER_WIDTH / 2;

pub(super) const HUD_DEBUG_PROFILER_RADIUS: f32 = 105.0;

pub(super) const HUD_DEBUG_PROFILER_VERTICAL_RADIUS_SCALE: f32 = 0.5;

pub(super) const HUD_DEBUG_PROFILER_THICKNESS: f32 = 10.0;

pub(super) const HUD_DEBUG_PROFILER_MARGIN: i32 = 5;

pub(super) const HUD_DEBUG_PROFILER_RIGHT_MARGIN: i32 = 10;

pub(super) const HUD_DEBUG_PROFILER_BOTTOM_OFFSET: i32 = 10;

pub(super) const HUD_DEBUG_PROFILER_HALF_HEIGHT: i32 = 62;

pub(super) const HUD_DEBUG_PROFILER_TEXT_INDENT: i32 = 10;

pub(super) const HUD_DEBUG_PROFILER_SLICE_CAPACITY: usize = 64;

pub(super) const HUD_DEBUG_LIGHTMAP_PREVIEW_SIZE: u32 = 64;

pub(super) const HUD_DEBUG_LIGHTMAP_PREVIEW_MARGIN: i32 = 2;

pub(super) const HUD_DEBUG_LIGHTMAP_PREVIEW_BORDER: i32 = 1;

pub(super) const HUD_DEBUG_GAME_MODE_SWITCHER_TEXTURE_WIDTH: f32 = 128.0;

pub(super) const HUD_DEBUG_GAME_MODE_SWITCHER_TEXTURE_HEIGHT: f32 = 128.0;

pub(super) const HUD_DEBUG_GAME_MODE_SWITCHER_BACKGROUND_U_WIDTH: f32 = 125.0;

pub(super) const HUD_DEBUG_GAME_MODE_SWITCHER_BACKGROUND_V_HEIGHT: f32 = 75.0;

pub(super) const HUD_DEBUG_GAME_MODE_SWITCHER_CENTER_X_OFFSET: i32 = 62;

pub(super) const HUD_DEBUG_GAME_MODE_SWITCHER_TITLE_Y_OFFSET: i32 = 7;

pub(super) const HUD_DEBUG_GAME_MODE_SWITCHER_HELP_Y_OFFSET: i32 = 63;

pub(super) const HUD_DEBUG_OPTIONS_HEADER_HEIGHT: i32 = 61;

pub(super) const HUD_DEBUG_OPTIONS_FOOTER_HEIGHT: i32 = 33;

pub(super) const HUD_DEBUG_OPTIONS_ROW_WIDTH: i32 = 350;

pub(super) const HUD_DEBUG_OPTIONS_ROW_HEIGHT: i32 = 20;

pub(super) const HUD_DEBUG_OPTIONS_STATUS_BUTTON_WIDTH: i32 = 60;

pub(super) const HUD_DEBUG_OPTIONS_STATUS_BUTTON_HEIGHT: i32 = 16;

pub(super) const HUD_DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH: i32 = 120;

pub(super) const HUD_DEBUG_OPTIONS_DONE_BUTTON_WIDTH: i32 = 60;

pub(super) const HUD_DEBUG_OPTIONS_FOOTER_BUTTON_HEIGHT: i32 = 20;

pub(super) const HUD_DEBUG_OPTIONS_FOOTER_BUTTON_SPACING: i32 = 8;

pub(super) const HUD_DEBUG_OPTIONS_SEARCH_WIDTH: i32 = HUD_DEBUG_OPTIONS_ROW_WIDTH / 3;

pub(super) const HUD_DEBUG_OPTIONS_SEARCH_HEIGHT: i32 = 20;

pub(super) const HUD_DEBUG_OPTIONS_SCROLLBAR_WIDTH: i32 = 6;

pub(super) const HUD_DEBUG_OPTIONS_SCROLLBAR_MIN_HEIGHT: i32 = 32;

pub(super) const HUD_PAUSE_RETURN_TO_GAME_BUTTON_WIDTH: i32 = 204;

pub(super) const HUD_PAUSE_HALF_BUTTON_WIDTH: i32 = 98;

pub(super) const HUD_PAUSE_RETURN_TO_GAME_BUTTON_HEIGHT: i32 = 20;

pub(super) const HUD_PAUSE_RETURN_TO_GAME_TOP_OFFSET: i32 = 8;

pub(super) const HUD_PAUSE_SECOND_ROW_TOP_OFFSET: i32 = 32;

pub(super) const HUD_PAUSE_THIRD_ROW_TOP_OFFSET: i32 = 56;

pub(super) const HUD_PAUSE_FOURTH_ROW_TOP_OFFSET: i32 = 80;

pub(super) const HUD_PAUSE_DISCONNECT_ROW_TOP_OFFSET: i32 = 104;

pub(super) const HUD_PAUSE_FEEDBACK_HEADER_HEIGHT: i32 = 33;

pub(super) const HUD_PAUSE_FEEDBACK_FOOTER_HEIGHT: i32 = 33;

pub(super) const HUD_PAUSE_FEEDBACK_CONTENT_MARGIN_TOP: i32 = 30;

pub(super) const HUD_PAUSE_FEEDBACK_CELL_PADDING: i32 = 4;

pub(super) const HUD_PAUSE_FEEDBACK_BACK_BUTTON_WIDTH: i32 = 200;

pub(super) const HUD_PAUSE_BACKGROUND_TOP_TINT: [f32; 4] =
    [16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0, 192.0 / 255.0];

pub(super) const HUD_PAUSE_BACKGROUND_BOTTOM_TINT: [f32; 4] =
    [16.0 / 255.0, 16.0 / 255.0, 16.0 / 255.0, 208.0 / 255.0];

pub(super) const HUD_STATS_DONE_BUTTON_WIDTH: i32 = 200;

pub(super) const HUD_STATS_DONE_BUTTON_HEIGHT: i32 = 20;

pub(super) const HUD_STATS_FOOTER_HEIGHT: i32 = 33;

pub(super) const HUD_STATS_LIST_WIDTH: i32 = 280;

pub const HUD_STATS_LIST_TOP: i32 = 24;

pub(super) const HUD_STATS_LIST_ENTRY_PADDING: i32 = 2;

pub(super) const HUD_DEBUG_CROSSHAIR_SCALE: f32 = 0.01;

pub(super) const HUD_DEBUG_CROSSHAIR_FOV_DEGREES: f32 = 70.0;

pub(super) const HUD_DEBUG_CROSSHAIR_OUTLINE_WIDTH: f32 = 4.0;

pub(super) const HUD_DEBUG_CROSSHAIR_AXIS_WIDTH: f32 = 2.0;

pub(super) const HUD_DEBUG_CROSSHAIR_OUTLINE_ARGB: u32 = 0xFF000000;

pub(super) const HUD_DEBUG_CROSSHAIR_AXIS_ARGB: [u32; 3] = [0xFFFF0000, 0xFF00FF00, 0xFF7F7FFF];

pub(super) const HUD_TOOLTIP_BACKGROUND_TINT: [f32; 4] = [0.0625, 0.0, 0.0625, 0.94];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAdvancementTabSprite {
    AboveLeftSelected,
    AboveMiddleSelected,
    AboveRightSelected,
    AboveLeft,
    AboveMiddle,
    AboveRight,
    BelowLeftSelected,
    BelowMiddleSelected,
    BelowRightSelected,
    BelowLeft,
    BelowMiddle,
    BelowRight,
    LeftTopSelected,
    LeftMiddleSelected,
    LeftBottomSelected,
    LeftTop,
    LeftMiddle,
    LeftBottom,
    RightTopSelected,
    RightMiddleSelected,
    RightBottomSelected,
    RightTop,
    RightMiddle,
    RightBottom,
}

impl HudAdvancementTabSprite {
    pub(crate) const COUNT: usize = 24;

    pub const ALL: [Self; Self::COUNT] = [
        Self::AboveLeftSelected,
        Self::AboveMiddleSelected,
        Self::AboveRightSelected,
        Self::AboveLeft,
        Self::AboveMiddle,
        Self::AboveRight,
        Self::BelowLeftSelected,
        Self::BelowMiddleSelected,
        Self::BelowRightSelected,
        Self::BelowLeft,
        Self::BelowMiddle,
        Self::BelowRight,
        Self::LeftTopSelected,
        Self::LeftMiddleSelected,
        Self::LeftBottomSelected,
        Self::LeftTop,
        Self::LeftMiddle,
        Self::LeftBottom,
        Self::RightTopSelected,
        Self::RightMiddleSelected,
        Self::RightBottomSelected,
        Self::RightTop,
        Self::RightMiddle,
        Self::RightBottom,
    ];

    pub fn sprite_path(self) -> &'static str {
        match self {
            Self::AboveLeftSelected => "advancements/tab_above_left_selected",
            Self::AboveMiddleSelected => "advancements/tab_above_middle_selected",
            Self::AboveRightSelected => "advancements/tab_above_right_selected",
            Self::AboveLeft => "advancements/tab_above_left",
            Self::AboveMiddle => "advancements/tab_above_middle",
            Self::AboveRight => "advancements/tab_above_right",
            Self::BelowLeftSelected => "advancements/tab_below_left_selected",
            Self::BelowMiddleSelected => "advancements/tab_below_middle_selected",
            Self::BelowRightSelected => "advancements/tab_below_right_selected",
            Self::BelowLeft => "advancements/tab_below_left",
            Self::BelowMiddle => "advancements/tab_below_middle",
            Self::BelowRight => "advancements/tab_below_right",
            Self::LeftTopSelected => "advancements/tab_left_top_selected",
            Self::LeftMiddleSelected => "advancements/tab_left_middle_selected",
            Self::LeftBottomSelected => "advancements/tab_left_bottom_selected",
            Self::LeftTop => "advancements/tab_left_top",
            Self::LeftMiddle => "advancements/tab_left_middle",
            Self::LeftBottom => "advancements/tab_left_bottom",
            Self::RightTopSelected => "advancements/tab_right_top_selected",
            Self::RightMiddleSelected => "advancements/tab_right_middle_selected",
            Self::RightBottomSelected => "advancements/tab_right_bottom_selected",
            Self::RightTop => "advancements/tab_right_top",
            Self::RightMiddle => "advancements/tab_right_middle",
            Self::RightBottom => "advancements/tab_right_bottom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAdvancementWidgetFrameSprite {
    TaskObtained,
    TaskUnobtained,
    ChallengeObtained,
    ChallengeUnobtained,
    GoalObtained,
    GoalUnobtained,
}

impl HudAdvancementWidgetFrameSprite {
    pub(crate) const COUNT: usize = 6;

    pub const ALL: [Self; Self::COUNT] = [
        Self::TaskObtained,
        Self::TaskUnobtained,
        Self::ChallengeObtained,
        Self::ChallengeUnobtained,
        Self::GoalObtained,
        Self::GoalUnobtained,
    ];

    pub fn sprite_path(self) -> &'static str {
        match self {
            Self::TaskObtained => "advancements/task_frame_obtained",
            Self::TaskUnobtained => "advancements/task_frame_unobtained",
            Self::ChallengeObtained => "advancements/challenge_frame_obtained",
            Self::ChallengeUnobtained => "advancements/challenge_frame_unobtained",
            Self::GoalObtained => "advancements/goal_frame_obtained",
            Self::GoalUnobtained => "advancements/goal_frame_unobtained",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAdvancementHoverBoxSprite {
    Title,
    Obtained,
    Unobtained,
}

impl HudAdvancementHoverBoxSprite {
    pub(crate) const COUNT: usize = 3;

    pub const ALL: [Self; Self::COUNT] = [Self::Title, Self::Obtained, Self::Unobtained];

    pub fn sprite_path(self) -> &'static str {
        match self {
            Self::Title => "advancements/title_box",
            Self::Obtained => "advancements/box_obtained",
            Self::Unobtained => "advancements/box_unobtained",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAdvancementBackgroundTexture {
    Stone,
    Adventure,
    Nether,
    End,
    Husbandry,
    Missing,
}

impl HudAdvancementBackgroundTexture {
    pub const VANILLA: [Self; 5] = [
        Self::Stone,
        Self::Adventure,
        Self::Nether,
        Self::End,
        Self::Husbandry,
    ];

    pub fn texture_path(self) -> Option<&'static str> {
        match self {
            Self::Stone => Some("textures/gui/advancements/backgrounds/stone.png"),
            Self::Adventure => Some("textures/gui/advancements/backgrounds/adventure.png"),
            Self::Nether => Some("textures/gui/advancements/backgrounds/nether.png"),
            Self::End => Some("textures/gui/advancements/backgrounds/end.png"),
            Self::Husbandry => Some("textures/gui/advancements/backgrounds/husbandry.png"),
            Self::Missing => None,
        }
    }

    pub fn texture_resource_id(self) -> Option<&'static str> {
        match self {
            Self::Stone => Some("minecraft:textures/gui/advancements/backgrounds/stone"),
            Self::Adventure => Some("minecraft:textures/gui/advancements/backgrounds/adventure"),
            Self::Nether => Some("minecraft:textures/gui/advancements/backgrounds/nether"),
            Self::End => Some("minecraft:textures/gui/advancements/backgrounds/end"),
            Self::Husbandry => Some("minecraft:textures/gui/advancements/backgrounds/husbandry"),
            Self::Missing => None,
        }
    }

    pub fn from_resource_id(id: &str) -> Option<Self> {
        let path = id.strip_prefix("minecraft:").unwrap_or(id);
        let path = path.strip_suffix(".png").unwrap_or(path);
        let path = path.strip_prefix("textures/").unwrap_or(path);
        match path {
            "gui/advancements/backgrounds/stone" => Some(Self::Stone),
            "gui/advancements/backgrounds/adventure" => Some(Self::Adventure),
            "gui/advancements/backgrounds/nether" => Some(Self::Nether),
            "gui/advancements/backgrounds/end" => Some(Self::End),
            "gui/advancements/backgrounds/husbandry" => Some(Self::Husbandry),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAdvancementLineTexture {
    Background,
    Foreground,
}

/// What one inventory background layer samples, keyed like Vanilla's
/// `blitSprite`/`blit` calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudInventoryBackgroundTexture {
    /// A `gui` atlas sprite (`container/loom/pattern`).
    Sprite(&'static str),
    /// A standalone texture (`textures/gui/container/loom.png`).
    Texture(&'static str),
    /// A sprite blitted with an alpha (`SmithingScreen`'s fading slot icons).
    FadedSprite {
        id: &'static str,
        alpha_bits: u32,
    },
    /// `Gui.getMobEffectSprite` of one effect id.
    MobEffect(i32),
    AdvancementLine(HudAdvancementLineTexture),
}

impl HudInventoryBackgroundTexture {
    pub fn smithing_slot_icon(icon: HudSmithingSlotIcon, alpha: f32) -> Self {
        Self::FadedSprite {
            id: icon.sprite_path(),
            alpha_bits: alpha.clamp(0.0, 1.0).to_bits(),
        }
    }

    pub(super) fn smithing_slot_icon_alpha(self) -> Option<f32> {
        match self {
            Self::FadedSprite { alpha_bits, .. } => Some(f32::from_bits(alpha_bits)),
            _ => None,
        }
    }

    /// A datapack background outside the Vanilla set draws the missing texture.
    pub fn advancement_background(texture: HudAdvancementBackgroundTexture) -> Self {
        texture
            .texture_path()
            .map_or(Self::Sprite("missingno"), Self::Texture)
    }

    /// `CreativeModeInventoryScreen` tab sprites `container/creative_inventory/tab_<row>_<state>_<n>`.
    pub fn creative_tab(row: HudCreativeInventoryTabRow, selected: bool, column: u8) -> Self {
        const TABS: [[[&str; 7]; 2]; 2] = [
            [
                [
                    "container/creative_inventory/tab_top_unselected_1",
                    "container/creative_inventory/tab_top_unselected_2",
                    "container/creative_inventory/tab_top_unselected_3",
                    "container/creative_inventory/tab_top_unselected_4",
                    "container/creative_inventory/tab_top_unselected_5",
                    "container/creative_inventory/tab_top_unselected_6",
                    "container/creative_inventory/tab_top_unselected_7",
                ],
                [
                    "container/creative_inventory/tab_top_selected_1",
                    "container/creative_inventory/tab_top_selected_2",
                    "container/creative_inventory/tab_top_selected_3",
                    "container/creative_inventory/tab_top_selected_4",
                    "container/creative_inventory/tab_top_selected_5",
                    "container/creative_inventory/tab_top_selected_6",
                    "container/creative_inventory/tab_top_selected_7",
                ],
            ],
            [
                [
                    "container/creative_inventory/tab_bottom_unselected_1",
                    "container/creative_inventory/tab_bottom_unselected_2",
                    "container/creative_inventory/tab_bottom_unselected_3",
                    "container/creative_inventory/tab_bottom_unselected_4",
                    "container/creative_inventory/tab_bottom_unselected_5",
                    "container/creative_inventory/tab_bottom_unselected_6",
                    "container/creative_inventory/tab_bottom_unselected_7",
                ],
                [
                    "container/creative_inventory/tab_bottom_selected_1",
                    "container/creative_inventory/tab_bottom_selected_2",
                    "container/creative_inventory/tab_bottom_selected_3",
                    "container/creative_inventory/tab_bottom_selected_4",
                    "container/creative_inventory/tab_bottom_selected_5",
                    "container/creative_inventory/tab_bottom_selected_6",
                    "container/creative_inventory/tab_bottom_selected_7",
                ],
            ],
        ];
        let row = match row {
            HudCreativeInventoryTabRow::Top => 0,
            HudCreativeInventoryTabRow::Bottom => 1,
        };
        Self::Sprite(TABS[row][usize::from(selected)][usize::from(column.min(6))])
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryBackgroundLayer {
    pub texture: HudInventoryBackgroundTexture,
    /// Layer x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Layer y position relative to the centered inventory screen origin.
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub uv: HudUvRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudInventoryFillStage {
    BeforeGhostItem,
    AfterGhostItem,
    Foreground,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryFillLayer {
    /// Fill x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Fill y position relative to the centered inventory screen origin.
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub tint: [f32; 4],
    pub stage: HudInventoryFillStage,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventorySlot {
    /// Slot id in the currently open inventory container.
    pub slot_id: u16,
    /// Slot x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Slot y position relative to the centered inventory screen origin.
    pub y: i32,
    pub icon: Option<HudItemIcon>,
    /// The slot's 3D block-item model (vanilla 3D inventory icon), when the item is a block. Drawn in
    /// the GUI item pass at the slot's pixel rect; when present, the 2D `icon`'s flat sprite layers are
    /// suppressed (the 3D model replaces them) while its count / durability overlays still draw.
    pub block_model: Option<HudBlockItemModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryItem {
    /// Item icon x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Item icon y position relative to the centered inventory screen origin.
    pub y: i32,
    /// Horizontal pose scale applied around the item's top-left GUI rect.
    pub scale: f32,
    /// Vertical pose scale applied around the item's top-left GUI rect.
    pub scale_y: f32,
    pub icon: HudItemIcon,
    /// Optional clip rectangle relative to the centered inventory screen origin. This is used for
    /// vanilla advancement fake items, which are rendered inside the advancement contents scissor.
    pub scissor: Option<HudInventoryItemScissor>,
    /// Whether count, durability, and cooldown overlays should be drawn for this floating item.
    pub draw_decorations: bool,
    /// The item's 3D block-item model (vanilla 3D inventory icon), when it is a block. See
    /// [`HudInventorySlot::block_model`].
    pub block_model: Option<HudBlockItemModel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudInventoryItemScissor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryGhostItem {
    /// Fake item x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Fake item y position relative to the centered inventory screen origin.
    pub y: i32,
    pub icon: HudItemIcon,
    pub draw_decorations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSignEditorKind {
    Standing {
        wood: SignModelWood,
        attachment: SignModelAttachment,
    },
    Hanging {
        wood: SignModelWood,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudSignEditorScreen {
    pub kind: HudSignEditorKind,
    pub sign_preview: Option<HudEntityPreview>,
    pub title: String,
    pub lines: [String; 4],
    pub line: usize,
    pub cursor: usize,
    pub selection: usize,
    pub cursor_visible: bool,
    pub text_tint: [f32; 4],
    pub done: String,
    pub done_hovered: bool,
    pub preedit: Option<HudImePreedit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudImePreedit {
    pub text: String,
    pub focused_start: usize,
    pub focused_end: usize,
    pub caret: Option<usize>,
    pub caret_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudCommandBlockEditorKind {
    Block,
    Minecart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudCommandBlockEditorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl HudCommandBlockEditorRect {
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add_unsigned(self.width)
            && y < self.y.saturating_add_unsigned(self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudCommandBlockEditorLayout {
    pub command_input: HudCommandBlockEditorRect,
    pub previous_output: HudCommandBlockEditorRect,
    pub track_output: HudCommandBlockEditorRect,
    pub mode: HudCommandBlockEditorRect,
    pub conditional: HudCommandBlockEditorRect,
    pub automatic: HudCommandBlockEditorRect,
    pub done: HudCommandBlockEditorRect,
    pub cancel: HudCommandBlockEditorRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudStructureBlockMode {
    Save,
    Load,
    Corner,
    Data,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudStructureBlockEditorScreen {
    pub title: String,
    pub name_label: String,
    pub position_label: String,
    pub size_label: String,
    pub integrity_label: String,
    pub data_label: String,
    pub include_entities_label: String,
    pub strict_label: String,
    pub detect_label: String,
    pub option_label: String,
    pub mode_description: String,
    pub fields: [HudTextField; 10],
    pub mode: HudStructureBlockMode,
    pub mode_value: String,
    pub mirror_value: String,
    pub rotation: String,
    pub include_entities: String,
    pub strict: String,
    pub option: String,
    pub action: String,
    pub detect: String,
    pub done: String,
    pub cancel: String,
    pub mode_hovered: bool,
    pub mirror_hovered: bool,
    pub rotation_hovered: [bool; 4],
    pub include_entities_hovered: bool,
    pub strict_hovered: bool,
    pub option_hovered: bool,
    pub action_hovered: bool,
    pub detect_hovered: bool,
    pub done_hovered: bool,
    pub cancel_hovered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudStructureBlockEditorLayout {
    pub fields: [HudCommandBlockEditorRect; 10],
    pub mode: HudCommandBlockEditorRect,
    pub mirror: HudCommandBlockEditorRect,
    pub rotations: [HudCommandBlockEditorRect; 4],
    pub option: HudCommandBlockEditorRect,
    pub detect: HudCommandBlockEditorRect,
    pub strict: HudCommandBlockEditorRect,
    pub include_entities: HudCommandBlockEditorRect,
    pub action: HudCommandBlockEditorRect,
    pub done: HudCommandBlockEditorRect,
    pub cancel: HudCommandBlockEditorRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudJigsawBlockEditorScreen {
    pub labels: [String; 6],
    pub joint_label: String,
    pub fields: [HudTextField; 6],
    pub joint_visible: bool,
    pub joint: String,
    pub levels: String,
    pub slider_value_bits: u64,
    pub keep_jigsaws: String,
    pub generate: String,
    pub done: String,
    pub cancel: String,
    pub controls_enabled: bool,
    pub joint_hovered: bool,
    pub levels_hovered: bool,
    pub keep_jigsaws_hovered: bool,
    pub generate_hovered: bool,
    pub done_hovered: bool,
    pub cancel_hovered: bool,
    pub tooltip: Option<HudDebugOptionsTooltip>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudJigsawBlockEditorLayout {
    pub fields: [HudCommandBlockEditorRect; 6],
    pub joint: HudCommandBlockEditorRect,
    pub levels: HudCommandBlockEditorRect,
    pub keep_jigsaws: HudCommandBlockEditorRect,
    pub generate: HudCommandBlockEditorRect,
    pub done: HudCommandBlockEditorRect,
    pub cancel: HudCommandBlockEditorRect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudTestBlockEditorScreen {
    pub title: String,
    pub message_label: String,
    pub message: HudTextField,
    pub message_visible: bool,
    pub mode: String,
    pub mode_info: String,
    pub done: String,
    pub cancel: String,
    pub mode_hovered: bool,
    pub done_hovered: bool,
    pub cancel_hovered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudTestBlockEditorLayout {
    pub message: HudCommandBlockEditorRect,
    pub mode: HudCommandBlockEditorRect,
    pub done: HudCommandBlockEditorRect,
    pub cancel: HudCommandBlockEditorRect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudTestInstanceBlockEditorScreen {
    pub title: String,
    pub id_label: String,
    pub size_label: String,
    pub rotation_label: String,
    pub entities_label: String,
    pub id: HudTextField,
    pub size: [HudTextField; 3],
    pub info_runs: Vec<HudStyledTextRun>,
    pub rotation: String,
    pub include_entities: String,
    pub reset: String,
    pub save: String,
    pub run: String,
    pub done: String,
    pub cancel: String,
    pub save_enabled: bool,
    pub rotation_hovered: bool,
    pub include_entities_hovered: bool,
    pub reset_hovered: bool,
    pub save_hovered: bool,
    pub run_hovered: bool,
    pub done_hovered: bool,
    pub cancel_hovered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudTestInstanceBlockEditorLayout {
    pub id: HudCommandBlockEditorRect,
    pub info: HudCommandBlockEditorRect,
    pub size: [HudCommandBlockEditorRect; 3],
    pub rotation: HudCommandBlockEditorRect,
    pub include_entities: HudCommandBlockEditorRect,
    pub reset: HudCommandBlockEditorRect,
    pub save: HudCommandBlockEditorRect,
    pub run: HudCommandBlockEditorRect,
    pub done: HudCommandBlockEditorRect,
    pub cancel: HudCommandBlockEditorRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryTextBackground {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub tint: [f32; 4],
}

/// One `EditBox` value as a screen hands it over. `text` decodes the Java
/// value one unit per unit (a lone surrogate becomes U+FFFD), so `cursor`,
/// `selection`, `scroll_to` and `max_length` are UTF-16 units of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudTextField {
    pub text: String,
    pub cursor: usize,
    pub selection: usize,
    /// The last `EditBox.scrollTo` target; the box is drawn scrolled from
    /// the start to it.
    pub scroll_to: usize,
    pub max_length: usize,
    pub focused: bool,
    pub hovered: bool,
    /// Focused and inside the blink's visible phase.
    pub cursor_visible: bool,
    pub preedit: Option<HudImePreedit>,
}

impl HudTextField {
    pub fn decoration(&self, cursor_tint: [f32; 4]) -> HudInventoryTextInputDecoration {
        HudInventoryTextInputDecoration {
            cursor: self.cursor,
            selection: self.selection,
            scroll_to: self.scroll_to,
            max_length: self.max_length,
            cursor_visible: self.cursor_visible,
            cursor_tint,
        }
    }
}

/// Draw-time edit box over a label's text; positions are UTF-16 units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryTextInputDecoration {
    pub cursor: usize,
    pub selection: usize,
    pub scroll_to: usize,
    pub max_length: usize,
    pub cursor_visible: bool,
    pub cursor_tint: [f32; 4],
}

/// Vanilla `ActiveTextCollector.acceptScrollingWithDefaultCenter` bounds for a
/// single-line widget label, relative to the centered inventory screen origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudInventoryScrollingText {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryTextLabel {
    /// Nonempty component runs are canonical, already wrapped display text.
    pub component_runs: Vec<HudComponentTextRun>,
    /// Text x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Text y position relative to the centered inventory screen origin.
    pub y: i32,
    pub width: u32,
    pub text: String,
    pub tint: [f32; 4],
    pub background: Option<HudInventoryTextBackground>,
    pub input: Option<HudInventoryTextInputDecoration>,
    pub scrolling: Option<HudInventoryScrollingText>,
    pub shadow: bool,
    /// Whole-label font id. Missing resource-pack ids fall back to the default
    /// font through `HudFontGlyphBook`.
    pub font: Option<String>,
    /// Styled draw runs; concatenated run text matches `text`. Leave empty
    /// for plain labels — sanitization synthesizes a single default-style run
    /// from `text`.
    pub runs: Vec<HudStyledTextRun>,
}

#[derive(Debug, Clone, PartialEq)]
/// `MultiLineEditBox`; `cursor` and `selection` are UTF-16 units of `text`.
pub struct HudInventoryTextArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub text: String,
    pub cursor: usize,
    pub selection: usize,
    /// Lays out the cursor line even while the caret blinks off, as an IME
    /// overlay anchored at the caret needs.
    pub needs_cursor_position: bool,
    pub scroll_amount: i32,
    pub max_length: usize,
    pub cursor_visible: bool,
    pub text_tint: [f32; 4],
    pub cursor_tint: [f32; 4],
    pub counter_tint: [f32; 4],
    pub shadow: bool,
}

/// Deferred IME composition candidate rendered above the owning inventory-style
/// screen. Coordinates are absolute HUD pixels because vanilla clamps the
/// overlay against the full GUI surface rather than the centered dialog frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudInventoryPreeditOverlay {
    pub text: String,
    pub focused_start: usize,
    pub focused_end: usize,
    pub caret: Option<usize>,
    pub caret_visible: bool,
    pub text_x: i32,
    pub text_y: i32,
    pub background_x: i32,
    pub background_y: i32,
    pub background_width: u32,
    pub background_height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryTooltipLine {
    pub text: String,
    pub tint: [f32; 4],
    /// Component draw runs; concatenated visual text matches `text`. Leave
    /// empty for plain lines — sanitization synthesizes one default-font run
    /// from `text`.
    pub runs: Vec<HudComponentTextRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryTooltip {
    pub slot_id: u16,
    /// Tooltip anchor x position relative to the centered inventory screen origin.
    pub x: i32,
    /// Tooltip anchor y position relative to the centered inventory screen origin.
    pub y: i32,
    /// ItemStack `DataComponents.TOOLTIP_STYLE`; non-item tooltips leave this absent.
    pub style: Option<String>,
    pub lines: Vec<HudInventoryTooltipLine>,
}

/// Visual bundle-contents tooltip component matching Vanilla's
/// `ClientBundleTooltip` (`ClientBundleTooltip.java:19-222`).
#[derive(Debug, Clone, PartialEq)]
pub struct HudBundleTooltip {
    /// Displayed bundle items in visual order (at most 12 entries; index 0 is
    /// the top/first-shown entry matching `BundleContents.items()`).
    pub items: Vec<HudItemIcon>,
    /// Count of hidden items beyond the 11 shown slots when the bundle holds
    /// more than 12 distinct stacks (`+N` surplus cell).
    pub hidden_item_count: u32,
    /// Index into `items` currently highlighted by scroll selection.
    pub selected_index: Option<usize>,
    /// Selected item's styled hover name drawn above the grid.
    pub selected_item_name: Option<HudInventoryTooltipLine>,
    /// Selected item's `DataComponents.TOOLTIP_STYLE` override, if any.
    pub selected_item_tooltip_style: Option<String>,
    /// Normalized occupancy fraction (`weight`), where `>= 1.0` marks the
    /// bundle as full.
    pub occupancy: f32,
    /// Localized description drawn when the bundle is empty
    /// (`item.minecraft.bundle.empty.description`).
    pub empty_description: String,
    /// Localized progress-bar label (`item.minecraft.bundle.empty` when empty,
    /// `item.minecraft.bundle.full` when full, `None` when partially filled).
    pub progress_label: Option<String>,
}

/// A clipped child surface inside a centered inventory-style screen. Coordinates
/// use the same screen-relative space as the parent layers.
#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryScrollArea {
    pub scissor: HudInventoryItemScissor,
    pub background_layers: Vec<HudInventoryBackgroundLayer>,
    pub fill_layers: Vec<HudInventoryFillLayer>,
    pub text_labels: Vec<HudInventoryTextLabel>,
    pub text_areas: Vec<HudInventoryTextArea>,
    pub scrollbar: Option<HudInventoryScrollbar>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudInventoryScrollbar {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub thumb_y: i32,
    pub thumb_height: u32,
}

/// One dynamic map rendered inside an inventory screen. Coordinates are
/// relative to the centered screen origin. `after_background_layer` preserves
/// Vanilla GUI strata by inserting the complete pixels/decorations/name stack
/// immediately after the indexed background layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryMapPreview {
    pub map_id: i32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub after_background_layer: usize,
}

/// One Loom pattern thumbnail interleaved immediately after its 14x14 button.
/// The 5x10 gray banner base is always retained; `texture = None` represents a
/// synchronized pattern whose bounded dynamic atlas entry is not resident yet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudInventoryBannerPatternPreview {
    pub texture: Option<BannerPatternTexture>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Normalized crop inside the pattern's 64x64 sprite.
    pub uv: HudUvRect,
    pub after_background_layer: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudInventoryScreen {
    pub width: u32,
    pub height: u32,
    pub background_layers: Vec<HudInventoryBackgroundLayer>,
    /// Foreground GUI sprite layers drawn after fake item icons and before text labels/tooltips.
    pub foreground_layers: Vec<HudInventoryBackgroundLayer>,
    /// Solid GUI fills drawn around recipe-book ghost fake items.
    pub fill_layers: Vec<HudInventoryFillLayer>,
    /// Slots for the currently open inventory container.
    pub slots: Vec<HudInventorySlot>,
    /// Item icons drawn by the inventory screen that are not container slots.
    pub floating_items: Vec<HudInventoryItem>,
    /// Foreground item icons drawn after inventory/screen text, for vanilla fake-item overlays.
    pub foreground_items: Vec<HudInventoryItem>,
    /// Recipe-book ghost fake items drawn above normal slots and below the foreground slot highlight.
    pub ghost_items: Vec<HudInventoryGhostItem>,
    /// Entity previews drawn through vanilla GUI picture-in-picture renderers.
    pub entity_previews: Vec<HudEntityPreview>,
    /// Dynamic cartography previews interleaved with the static background
    /// layers according to each entry's insertion point.
    pub map_previews: Vec<HudInventoryMapPreview>,
    /// Loom pattern thumbnails interleaved with their button sprites.
    pub banner_pattern_previews: Vec<HudInventoryBannerPatternPreview>,
    pub text_labels: Vec<HudInventoryTextLabel>,
    pub scroll_area: Option<HudInventoryScrollArea>,
    pub preedit_overlay: Option<HudInventoryPreeditOverlay>,
    pub hovered_slot_id: Option<u16>,
    pub tooltip: Option<HudInventoryTooltip>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HudScreenBlurDraws {
    pub(crate) radius: f32,
    pub(crate) first_base_command: usize,
    pub(crate) first_overlay_command: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HudScreenStratum {
    All,
    BeforeBlur,
    AfterBlur,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HudInventoryMobEffectInstance {
    pub(super) effect_id: i32,
    pub(super) ambient: bool,
    pub(super) background_x: i32,
    pub(super) background_y: i32,
    pub(super) background_width: u32,
    pub(super) icon_x: i32,
    pub(super) icon_y: i32,
    /// `ComponentRenderUtils.clipText` result over the name's resolved runs.
    pub(super) name: Option<Vec<HudStyledTextRun>>,
    /// `MobEffectUtil.formatDuration` resolved once for the panel width and the
    /// glyph pass.
    pub(super) duration: Option<Vec<HudStyledTextRun>>,
    pub(super) text_x: i32,
    pub(super) text_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudScrollingTextLayout {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) clipped: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct HudInventoryTextInputLayout {
    pub(super) split_text: Option<(String, String)>,
    pub(super) displayed_text: String,
    pub(super) cursor_x: f32,
    pub(super) cursor_on_screen: bool,
    pub(super) insert_cursor: bool,
    pub(super) selection_rect: Option<(f32, u32)>,
}

pub(super) const HUD_ADVANCEMENT_TOAST_WIDTH: u32 = 160;

pub(super) const HUD_ADVANCEMENT_TOAST_HEIGHT: u32 = 32;

pub(super) const HUD_ADVANCEMENT_TOAST_TITLE_WIDTH: u32 = 125;

pub(super) const HUD_TUTORIAL_TOAST_TEXT_WIDTH: u32 = 126;

pub(super) const HUD_IME_PREEDIT_TEXT_HEIGHT: i32 = 9;

pub(super) const HUD_IME_PREEDIT_INPUT_SEPARATION: i32 = 4;

pub(super) const HUD_IME_PREEDIT_BACKGROUND_OFFSET: i32 = 5;

pub(super) const HUD_IME_PREEDIT_HOT_AREA_MARGIN: i32 = 2;

pub(super) struct HudImePreeditProjection {
    pub(super) overlay: HudInventoryPreeditOverlay,
    pub(super) ime_position: PhysicalPosition<i32>,
    pub(super) ime_size: PhysicalSize<u32>,
}

/// One centered HUD overlay text line resolved for this frame: the styled
/// runs, the pen origin in HUD pixels, the vanilla pose scale, and the
/// fade-resolved base tint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudScreenTextDraw<'a> {
    pub(super) runs: &'a [HudStyledTextRun],
    pub(super) origin: (f32, f32),
    pub(super) scale: f32,
    pub(super) tint: [f32; 4],
    pub(super) backdrop_opacity: f32,
}

/// One action-bar/title draw whose runs retain exact Component font identity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HudComponentScreenTextDraw<'a> {
    pub(super) runs: &'a [HudComponentTextRun],
    pub(super) origin: (f32, f32),
    pub(super) scale: f32,
    pub(super) tint: [f32; 4],
    pub(super) backdrop_opacity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HudDebugProfilerChartLayout {
    pub(super) left: i32,
    pub(super) right: i32,
    pub(super) chart_center_x: i32,
    pub(super) chart_center_y: i32,
    pub(super) text_start_y: i32,
    pub(super) current_node_top: i32,
    pub(super) bottom: i32,
}

/// Stable font-space geometry for one wrapped chat line. Chat delivery and
/// font-resource changes already rebuild `hud_chat_lines`, while ordinary
/// render frames only change the line origin and fade alpha. Obfuscated text
/// intentionally bypasses this cache because vanilla chooses a fresh glyph
/// substitution from the render-frame seed. Explicit shadows also bypass it
/// because their alpha is quantized after the current text fade.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HudChatLineGeometry {
    pub(super) geometry: HudStyledTextGeometry,
}

pub(super) const HUD_CHAT_MAX_WRAPPED_LINES: usize = 100;

pub(super) const HUD_CHAT_MAX_MESSAGES: usize = 100;

pub(super) const HUD_CHAT_MODIFIED_ICON_WIDTH_WITH_GAP: u32 = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HudChatStyledChar {
    pub(super) ch: char,
    pub(super) style: HudTextStyle,
    pub(super) color: Option<u32>,
    pub(super) shadow_color: Option<u32>,
    pub(super) font: Option<String>,
    pub(super) target_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct HudWrappedChatLine {
    pub(super) runs: Vec<HudStyledTextRun>,
    pub(super) targets: Vec<Option<usize>>,
    pub(super) interactions: Vec<HudChatLineInteraction>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct HudWrappedComponentChatLine {
    pub(super) runs: Vec<HudComponentTextRun>,
    pub(super) targets: Vec<Option<usize>>,
    pub(super) interactions: Vec<HudChatLineInteraction>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum HudChatFontResolver<'a> {
    Default(&'a HudFontGlyphMap),
    Book(&'a HudFontGlyphBook),
}

impl<'a> HudChatFontResolver<'a> {
    pub(super) fn glyphs(self, font: Option<&str>) -> &'a HudFontGlyphMap {
        match self {
            Self::Default(glyphs) => glyphs,
            Self::Book(fonts) => fonts.glyphs(font),
        }
    }
}

/// Localized button labels for Vanilla's `PauseScreen` and `FeedbackSubScreen`,
/// defaulting to the Vanilla `en_us` English strings when no translation pack is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudPauseScreenLabels {
    pub return_to_game: String,
    pub advancements: String,
    pub stats: String,
    pub send_feedback: String,
    pub feedback: String,
    pub report_bugs: String,
    pub back: String,
    pub options: String,
    pub disconnect: String,
}

impl Default for HudPauseScreenLabels {
    fn default() -> Self {
        Self {
            return_to_game: "Back to Game".to_string(),
            advancements: "Advancements".to_string(),
            stats: "Statistics".to_string(),
            send_feedback: "Give Feedback".to_string(),
            feedback: "Feedback...".to_string(),
            report_bugs: "Report Bugs".to_string(),
            back: "Back".to_string(),
            options: "Options...".to_string(),
            disconnect: "Disconnect".to_string(),
        }
    }
}

/// Vanilla `PauseScreen` render-state shell: no-menu debug pause screens only submit the centered
/// title row, while menu pause screens project the transparent in-game background, title, and the
/// implemented menu buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudPauseScreen {
    pub title: String,
    pub labels: HudPauseScreenLabels,
    pub show_pause_menu: bool,
    pub feedback_subscreen: bool,
    pub return_to_game_hovered: bool,
    pub advancements_hovered: bool,
    pub stats_hovered: bool,
    pub send_feedback_hovered: bool,
    pub report_bugs_hovered: bool,
    pub report_bugs_enabled: bool,
    pub custom_dialog_label: Option<String>,
    pub custom_dialog_hovered: bool,
    pub custom_dialog_tooltip: Option<HudPauseScreenTooltip>,
    pub feedback_back_hovered: bool,
    pub options_hovered: bool,
    pub options_enabled: bool,
    pub options_tooltip: Option<HudPauseScreenTooltip>,
    pub disconnect_hovered: bool,
    pub disconnect_enabled: bool,
}

/// One visible vanilla system toast. Native freezes the construction-time
/// width and message lines, matching `SystemToast`; the renderer owns the
/// nine-slice layout and right-edge slide transform.
#[derive(Debug, Clone, PartialEq)]
pub struct HudSystemToast {
    pub title: String,
    pub message_lines: Vec<String>,
    pub width: u32,
    pub visible_portion: f32,
}

/// ToastManager extraction order across every toast family.
#[derive(Debug, Clone, PartialEq)]
pub enum HudToast {
    System(HudSystemToastInstance),
    Advancement(HudAdvancementToast),
    Recipe(HudRecipeToast),
    Tutorial(HudTutorialToast),
}

/// Vanilla's independent `NowPlayingToast`; unlike `HudToast`, this never
/// occupies one of the shared five slots.
#[derive(Debug, Clone, PartialEq)]
pub struct HudNowPlayingToast {
    pub text: String,
    pub visible_portion: f32,
    pub note_color_argb: u32,
    pub client_tick: u64,
    pub placement: HudNowPlayingToastPlacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudTutorialToastIcon {
    MovementKeys,
    Mouse,
    Tree,
    RecipeBook,
    WoodenPlanks,
    SocialInteractions,
}

/// Renderer-owned projection of one accepted waypoint-style JSON resource.
/// Sprite ids are the GUI-atlas locations selected by Vanilla's distance
/// thresholds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudLocatorStyle {
    pub near_distance: u32,
    pub far_distance: u32,
    pub sprite_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudCommandBlockEditorScreen {
    pub kind: HudCommandBlockEditorKind,
    pub title: String,
    pub command_label: String,
    pub previous_output_label: String,
    pub previous_output: String,
    pub mode: String,
    pub conditional: String,
    pub automatic: String,
    pub track_output: String,
    pub done: String,
    pub cancel: String,
    pub controls_enabled: bool,
    pub mode_hovered: bool,
    pub conditional_hovered: bool,
    pub automatic_hovered: bool,
    pub track_output_hovered: bool,
    pub done_hovered: bool,
    pub cancel_hovered: bool,
}

/// Vanilla `AbstractSignEditScreen.init`: one 200x20 Done button centred at
/// `height / 4 + 144`.
pub fn hud_sign_editor_done_button_rect(
    surface_size: PhysicalSize<u32>,
) -> HudCommandBlockEditorRect {
    HudCommandBlockEditorRect {
        x: i32::try_from(surface_size.width / 2)
            .unwrap_or(i32::MAX)
            .saturating_sub(100),
        y: i32::try_from(surface_size.height / 4)
            .unwrap_or(i32::MAX)
            .saturating_add(144),
        width: 200,
        height: 20,
    }
}
