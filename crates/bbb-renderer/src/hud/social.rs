//! Vanilla 26.1 Social Interactions screen presentation.
//!
//! The value surface stays renderer-only and the layout below is shared by
//! drawing and generation-tagged hit testing. Native input therefore never
//! reconstructs widget coordinates or trusts a UUID/action from an older
//! frame.

use bbb_render_types::EntityPlayerSkin;
use winit::dpi::PhysicalSize;

use crate::Renderer;

use super::{
    absolute_hud_rect, hud_argb_to_tint, hud_plain_text_width, hud_wrap_plain_text,
    nine_slice_segments, push_hud_debug_options_button, push_hud_debug_options_centered_text,
    push_hud_debug_options_centered_text_in_width, push_hud_debug_options_text,
    push_hud_debug_options_tooltip, push_hud_draw, push_hud_draw_with_uv_and_tint,
    push_hud_rect_scissor, push_hud_screen_text_draw, HudDebugOptionsButtonSprites,
    HudDebugOptionsTooltip, HudDrawCommand, HudFontGlyphMap, HudNineSliceScaling,
    HudNineSliceSprite, HudObfuscatedGlyphPool, HudScreenTextDraw, HudSpriteGpu,
    HudStatsScrollbarSprite, HudStyledTextRun, HudTextStyle, HudUvRect, HudVertex, HUD_TINT_WHITE,
};

const BACKGROUND_WIDTH: i32 = 236;
const IMAGE_WIDTH: i32 = 238;
const BACKGROUND_Y: i32 = 64;
const TAB_Y: i32 = 45;
const BUTTON_HEIGHT: i32 = 20;
const SEARCH_Y: i32 = 74;
const SEARCH_HEIGHT: i32 = 15;
const SEARCH_WIDTH: i32 = 200;
const LIST_Y: i32 = 88;
const ROW_WIDTH: i32 = 220;
const ROW_HEIGHT: i32 = 36;
const FACE_SIZE: u32 = 24;
const ACTION_SIZE: i32 = 20;
const SCROLLBAR_WIDTH: i32 = 6;
const SCROLLBAR_MIN_HEIGHT: i32 = 32;
const FOOTER_HEIGHT: i32 = 33;
const MAX_ROWS: usize = 4_096;
const MAX_TEXT_CHARS: usize = 65_536;
const MENU_BACKGROUND_TILE_SIZE: f32 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsPage {
    All,
    Hidden,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsPlayerAction {
    Hide,
    Show,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsFocus {
    Search,
    Tab(HudSocialInteractionsPage),
    Player {
        profile_id: [u8; 16],
        action: HudSocialInteractionsPlayerAction,
    },
    BlockingHelp,
    Done,
    LinkOpen,
    LinkCopy,
    LinkCancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudSocialInteractionsHitTargetKind {
    Focus(HudSocialInteractionsFocus),
    Scrollbar { scroll_amount: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSocialInteractionsHitTarget {
    pub generation: u64,
    pub kind: HudSocialInteractionsHitTargetKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudSocialInteractionsPlayer {
    pub profile_id: [u8; 16],
    pub name: String,
    pub skin: EntityPlayerSkin,
    pub removed: bool,
    pub hidden: bool,
    pub blocked: bool,
    /// Local-player rows have no hide/show control, matching Vanilla.
    pub action: Option<HudSocialInteractionsPlayerAction>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudSocialInteractionsMainScreen {
    pub generation: u64,
    pub title: String,
    pub server_label: String,
    pub page: HudSocialInteractionsPage,
    pub tab_labels: [String; 3],
    pub search_text: String,
    pub search_hint: String,
    pub search_cursor: usize,
    pub search_selection: usize,
    pub search_cursor_visible: bool,
    pub rows: Vec<HudSocialInteractionsPlayer>,
    pub scroll_amount: i32,
    pub empty_text: Option<String>,
    pub blocking_help_label: String,
    pub done_label: String,
    pub focus: HudSocialInteractionsFocus,
    pub hovered: Option<HudSocialInteractionsFocus>,
    pub tooltip: Option<HudDebugOptionsTooltip>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudSocialInteractionsLinkConfirmationScreen {
    pub generation: u64,
    pub title: String,
    pub message: String,
    pub open_label: String,
    pub copy_label: String,
    pub cancel_label: String,
    pub focus: HudSocialInteractionsFocus,
    pub hovered: Option<HudSocialInteractionsFocus>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HudSocialInteractionsScreen {
    Main(HudSocialInteractionsMainScreen),
    LinkConfirmation(HudSocialInteractionsLinkConfirmationScreen),
}

impl HudSocialInteractionsScreen {
    pub fn generation(&self) -> u64 {
        match self {
            Self::Main(screen) => screen.generation,
            Self::LinkConfirmation(screen) => screen.generation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSocialInteractionsRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl HudSocialInteractionsRect {
    fn contains(self, position: (f64, f64)) -> bool {
        position.0 >= f64::from(self.x)
            && position.1 >= f64::from(self.y)
            && position.0 < f64::from(self.x.saturating_add(self.width))
            && position.1 < f64::from(self.y.saturating_add(self.height))
    }

    fn intersects(self, other: Self) -> bool {
        self.x < other.x.saturating_add(other.width)
            && self.x.saturating_add(self.width) > other.x
            && self.y < other.y.saturating_add(other.height)
            && self.y.saturating_add(self.height) > other.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSocialInteractionsLayout {
    pub background: HudSocialInteractionsRect,
    pub tabs: [HudSocialInteractionsRect; 3],
    pub search: HudSocialInteractionsRect,
    pub search_icon: HudSocialInteractionsRect,
    pub list: HudSocialInteractionsRect,
    pub list_scissor: HudSocialInteractionsRect,
    pub scrollbar: Option<HudSocialInteractionsRect>,
    pub scrollbar_thumb: Option<HudSocialInteractionsRect>,
    pub blocking_help: Option<HudSocialInteractionsRect>,
    pub done: HudSocialInteractionsRect,
    pub max_scroll_amount: i32,
}

pub fn hud_social_interactions_max_scroll_amount(
    row_count: usize,
    surface_size: PhysicalSize<u32>,
) -> i32 {
    let list_height = hud_social_window_height(surface_size).saturating_sub(16);
    let content_height = i32::try_from(row_count)
        .unwrap_or(i32::MAX / ROW_HEIGHT)
        .saturating_mul(ROW_HEIGHT)
        .saturating_add(4);
    content_height.saturating_sub(list_height).max(0)
}

pub fn hud_social_interactions_layout(
    surface_size: PhysicalSize<u32>,
    row_count: usize,
    scroll_amount: i32,
    blocked_page: bool,
) -> HudSocialInteractionsLayout {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let margin_x = width.saturating_sub(IMAGE_WIDTH) / 2;
    let row_left = width / 2 - ROW_WIDTH / 2;
    let tab_width = ROW_WIDTH / 3;
    let window_height = hud_social_window_height(surface_size);
    let list_height = window_height.saturating_sub(16).max(0);
    let max_scroll_amount = hud_social_interactions_max_scroll_amount(row_count, surface_size);
    let scroll_amount = scroll_amount.clamp(0, max_scroll_amount);
    let (scrollbar, scrollbar_thumb) = if max_scroll_amount > 0 && list_height > 8 {
        let content_height = i32::try_from(row_count)
            .unwrap_or(i32::MAX / ROW_HEIGHT)
            .saturating_mul(ROW_HEIGHT)
            .saturating_add(4)
            .max(1);
        let maximum_thumb_height = (list_height - 8).max(1);
        let thumb_height = list_height
            .saturating_mul(list_height)
            .checked_div(content_height)
            .unwrap_or(maximum_thumb_height)
            .clamp(
                SCROLLBAR_MIN_HEIGHT.min(maximum_thumb_height),
                maximum_thumb_height,
            );
        let travel = (list_height - thumb_height).max(0);
        let thumb_y = LIST_Y
            + scroll_amount
                .saturating_mul(travel)
                .checked_div(max_scroll_amount)
                .unwrap_or(0);
        let x = row_left + ROW_WIDTH + SCROLLBAR_WIDTH + 2;
        (
            Some(HudSocialInteractionsRect {
                x,
                y: LIST_Y,
                width: SCROLLBAR_WIDTH,
                height: list_height,
            }),
            Some(HudSocialInteractionsRect {
                x,
                y: thumb_y,
                width: SCROLLBAR_WIDTH,
                height: thumb_height,
            }),
        )
    } else {
        (None, None)
    };
    HudSocialInteractionsLayout {
        background: HudSocialInteractionsRect {
            x: margin_x + 3,
            y: BACKGROUND_Y,
            width: BACKGROUND_WIDTH,
            height: window_height.saturating_add(16),
        },
        tabs: [
            HudSocialInteractionsRect {
                x: row_left,
                y: TAB_Y,
                width: tab_width,
                height: BUTTON_HEIGHT,
            },
            HudSocialInteractionsRect {
                x: row_left + (ROW_WIDTH - tab_width) / 2 + 1,
                y: TAB_Y,
                width: tab_width,
                height: BUTTON_HEIGHT,
            },
            HudSocialInteractionsRect {
                x: row_left + ROW_WIDTH - tab_width + 1,
                y: TAB_Y,
                width: tab_width,
                height: BUTTON_HEIGHT,
            },
        ],
        search: HudSocialInteractionsRect {
            x: margin_x + 28,
            y: SEARCH_Y,
            width: SEARCH_WIDTH,
            height: SEARCH_HEIGHT,
        },
        search_icon: HudSocialInteractionsRect {
            x: margin_x + 13,
            y: 76,
            width: 12,
            height: 12,
        },
        list: HudSocialInteractionsRect {
            x: 0,
            y: LIST_Y,
            width,
            height: list_height,
        },
        list_scissor: HudSocialInteractionsRect {
            x: 0,
            y: LIST_Y + 4,
            width,
            height: list_height.saturating_sub(4),
        },
        scrollbar,
        scrollbar_thumb,
        blocking_help: blocked_page.then_some(HudSocialInteractionsRect {
            x: width / 2 - 100,
            y: BACKGROUND_Y + window_height,
            width: 200,
            height: BUTTON_HEIGHT,
        }),
        done: HudSocialInteractionsRect {
            x: width / 2 - 100,
            y: height.saturating_sub(FOOTER_HEIGHT) + (FOOTER_HEIGHT - BUTTON_HEIGHT) / 2,
            width: 200,
            height: BUTTON_HEIGHT,
        },
        max_scroll_amount,
    }
}

fn hud_social_window_height(surface_size: PhysicalSize<u32>) -> i32 {
    i32::try_from(surface_size.height)
        .unwrap_or(i32::MAX)
        .saturating_sub(144)
        .max(52)
}

fn row_rect(
    index: usize,
    scroll_amount: i32,
    surface_size: PhysicalSize<u32>,
) -> HudSocialInteractionsRect {
    let width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    HudSocialInteractionsRect {
        x: width / 2 - ROW_WIDTH / 2,
        y: LIST_Y
            + 2
            + i32::try_from(index)
                .unwrap_or(i32::MAX / ROW_HEIGHT)
                .saturating_mul(ROW_HEIGHT)
            - scroll_amount,
        width: ROW_WIDTH,
        height: ROW_HEIGHT,
    }
}

fn row_action_rect(row: HudSocialInteractionsRect) -> HudSocialInteractionsRect {
    HudSocialInteractionsRect {
        x: row.x + row.width - ACTION_SIZE - 4 - ACTION_SIZE - 4,
        y: row.y + (row.height - ACTION_SIZE) / 2,
        width: ACTION_SIZE,
        height: ACTION_SIZE,
    }
}

fn scrollbar_scroll_amount(layout: HudSocialInteractionsLayout, position: (f64, f64)) -> i32 {
    let Some(scrollbar) = layout.scrollbar else {
        return 0;
    };
    let thumb_height = layout.scrollbar_thumb.map_or(0, |thumb| thumb.height);
    let travel = (scrollbar.height - thumb_height).max(1);
    let centered_y = position.1.floor() as i32 - scrollbar.y - thumb_height / 2;
    centered_y
        .clamp(0, travel)
        .saturating_mul(layout.max_scroll_amount)
        .checked_div(travel)
        .unwrap_or(0)
}

pub(super) fn sanitize_hud_social_interactions_screen(
    screen: HudSocialInteractionsScreen,
) -> Option<HudSocialInteractionsScreen> {
    Some(match screen {
        HudSocialInteractionsScreen::Main(mut screen) => {
            sanitize_string(&mut screen.title);
            sanitize_string(&mut screen.server_label);
            for label in &mut screen.tab_labels {
                sanitize_string(label);
            }
            screen.search_text = screen.search_text.chars().take(16).collect();
            sanitize_string(&mut screen.search_hint);
            let search_len = screen.search_text.chars().count();
            screen.search_cursor = screen.search_cursor.min(search_len);
            screen.search_selection = screen.search_selection.min(search_len);
            screen.rows.truncate(MAX_ROWS);
            for row in &mut screen.rows {
                sanitize_string(&mut row.name);
                if let Some(status) = &mut row.status {
                    sanitize_string(status);
                }
            }
            screen.scroll_amount = screen.scroll_amount.max(0);
            if let Some(empty_text) = &mut screen.empty_text {
                sanitize_string(empty_text);
            }
            sanitize_string(&mut screen.blocking_help_label);
            sanitize_string(&mut screen.done_label);
            if let Some(tooltip) = &mut screen.tooltip {
                sanitize_string(&mut tooltip.text);
            }
            HudSocialInteractionsScreen::Main(screen)
        }
        HudSocialInteractionsScreen::LinkConfirmation(mut screen) => {
            sanitize_string(&mut screen.title);
            sanitize_string(&mut screen.message);
            sanitize_string(&mut screen.open_label);
            sanitize_string(&mut screen.copy_label);
            sanitize_string(&mut screen.cancel_label);
            if !matches!(
                screen.focus,
                HudSocialInteractionsFocus::LinkOpen
                    | HudSocialInteractionsFocus::LinkCopy
                    | HudSocialInteractionsFocus::LinkCancel
            ) {
                screen.focus = HudSocialInteractionsFocus::LinkOpen;
            }
            screen.hovered = screen.hovered.filter(|focus| {
                matches!(
                    focus,
                    HudSocialInteractionsFocus::LinkOpen
                        | HudSocialInteractionsFocus::LinkCopy
                        | HudSocialInteractionsFocus::LinkCancel
                )
            });
            HudSocialInteractionsScreen::LinkConfirmation(screen)
        }
    })
}

fn sanitize_string(value: &mut String) {
    if value.chars().count() > MAX_TEXT_CHARS {
        *value = value.chars().take(MAX_TEXT_CHARS).collect();
    }
}

pub(super) fn hud_social_interactions_hit_target_at(
    screen: &HudSocialInteractionsScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
    position: (f64, f64),
) -> Option<HudSocialInteractionsHitTarget> {
    let generation = screen.generation();
    let focus = match screen {
        HudSocialInteractionsScreen::LinkConfirmation(screen) => {
            let layout = link_confirmation_layout(screen, glyphs, surface_size);
            [
                HudSocialInteractionsFocus::LinkOpen,
                HudSocialInteractionsFocus::LinkCopy,
                HudSocialInteractionsFocus::LinkCancel,
            ]
            .into_iter()
            .zip(layout.buttons)
            .find_map(|(focus, rect)| rect.contains(position).then_some(focus))?
        }
        HudSocialInteractionsScreen::Main(screen) => {
            let layout = hud_social_interactions_layout(
                surface_size,
                screen.rows.len(),
                screen.scroll_amount,
                screen.page == HudSocialInteractionsPage::Blocked,
            );
            let scroll_amount = screen.scroll_amount.clamp(0, layout.max_scroll_amount);
            if layout.search.contains(position) {
                HudSocialInteractionsFocus::Search
            } else if let Some((index, _)) = layout
                .tabs
                .iter()
                .enumerate()
                .find(|(_, rect)| rect.contains(position))
            {
                HudSocialInteractionsFocus::Tab(page_for_index(index))
            } else if let Some(rect) = layout.blocking_help.filter(|rect| rect.contains(position)) {
                let _ = rect;
                HudSocialInteractionsFocus::BlockingHelp
            } else if layout.done.contains(position) {
                HudSocialInteractionsFocus::Done
            } else if let Some(scrollbar) = layout
                .scrollbar
                .filter(|scrollbar| scrollbar.contains(position))
            {
                let _ = scrollbar;
                return Some(HudSocialInteractionsHitTarget {
                    generation,
                    kind: HudSocialInteractionsHitTargetKind::Scrollbar {
                        scroll_amount: scrollbar_scroll_amount(layout, position),
                    },
                });
            } else {
                screen.rows.iter().enumerate().find_map(|(index, player)| {
                    let action = player.action?;
                    let row = row_rect(index, scroll_amount, surface_size);
                    (row.intersects(layout.list_scissor)
                        && layout.list_scissor.contains(position)
                        && row_action_rect(row).contains(position))
                    .then_some(HudSocialInteractionsFocus::Player {
                        profile_id: player.profile_id,
                        action,
                    })
                })?
            }
        }
    };
    Some(HudSocialInteractionsHitTarget {
        generation,
        kind: HudSocialInteractionsHitTargetKind::Focus(focus),
    })
}

fn page_for_index(index: usize) -> HudSocialInteractionsPage {
    match index {
        1 => HudSocialInteractionsPage::Hidden,
        2 => HudSocialInteractionsPage::Blocked,
        _ => HudSocialInteractionsPage::All,
    }
}

#[derive(Debug, Clone)]
struct LinkConfirmationLayout {
    title_y: i32,
    message_y: i32,
    buttons: [HudSocialInteractionsRect; 3],
}

fn link_confirmation_layout(
    screen: &HudSocialInteractionsLinkConfirmationScreen,
    glyphs: &HudFontGlyphMap,
    surface_size: PhysicalSize<u32>,
) -> LinkConfirmationLayout {
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let max_width = surface_size.width.saturating_sub(50).max(1);
    let title_lines = hud_wrap_plain_text(&screen.title, max_width, glyphs).len();
    let message_lines = hud_wrap_plain_text(&screen.message, max_width, glyphs).len();
    let text_height = i32::try_from(title_lines.saturating_add(message_lines))
        .unwrap_or(i32::MAX / 9)
        .saturating_mul(9)
        .saturating_add(8);
    let total_height = text_height + 16 + BUTTON_HEIGHT;
    let title_y = (height - total_height) / 2;
    let message_y = title_y
        + i32::try_from(title_lines)
            .unwrap_or(i32::MAX / 9)
            .saturating_mul(9)
        + 8;
    let button_y = title_y + text_height + 16;
    let button_width = 100;
    let gap = 4;
    let total_width = button_width * 3 + gap * 2;
    let start_x = i32::try_from(surface_size.width).unwrap_or(i32::MAX) / 2 - total_width / 2;
    LinkConfirmationLayout {
        title_y,
        message_y,
        buttons: std::array::from_fn(|index| HudSocialInteractionsRect {
            x: start_x + i32::try_from(index).unwrap_or_default() * (button_width + gap),
            y: button_y,
            width: button_width,
            height: BUTTON_HEIGHT,
        }),
    }
}

pub(super) struct HudSocialInteractionsSprites<'a> {
    pub background: Option<&'a HudNineSliceSprite>,
    pub search: Option<&'a HudSpriteGpu>,
    pub mute: Option<&'a HudSpriteGpu>,
    pub mute_highlighted: Option<&'a HudSpriteGpu>,
    pub unmute: Option<&'a HudSpriteGpu>,
    pub unmute_highlighted: Option<&'a HudSpriteGpu>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_hud_social_interactions_screen<'a>(
    renderer: &'a Renderer,
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    inworld_menu_background: Option<&'a HudSpriteGpu>,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    text_field: Option<&'a HudSpriteGpu>,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    scrollbar_sprites: &'a [Option<HudNineSliceSprite>; HudStatsScrollbarSprite::COUNT],
    tooltip_background: Option<&'a HudNineSliceSprite>,
    tooltip_frame: Option<&'a HudNineSliceSprite>,
    social_sprites: HudSocialInteractionsSprites<'a>,
    font_atlas: Option<&'a HudSpriteGpu>,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSocialInteractionsScreen,
) {
    push_menu_background(
        vertices,
        commands,
        white_pixel,
        inworld_menu_background,
        surface_size,
    );
    let Some(font_atlas) = font_atlas else {
        return;
    };
    match screen {
        HudSocialInteractionsScreen::Main(screen) => push_main(
            renderer,
            vertices,
            commands,
            white_pixel,
            button_sprites,
            text_field,
            text_field_highlighted,
            scrollbar_sprites,
            tooltip_background,
            tooltip_frame,
            social_sprites,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            screen,
        ),
        HudSocialInteractionsScreen::LinkConfirmation(screen) => push_link_confirmation(
            vertices,
            commands,
            white_pixel,
            button_sprites,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            screen,
        ),
    }
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
            [0.12, 0.12, 0.12, 0.86],
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
        absolute_hud_rect(
            0.0,
            0.0,
            surface_size.width.max(1),
            surface_size.height.max(1),
        ),
        uv,
        tint,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_main<'a>(
    renderer: &'a Renderer,
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    text_field: Option<&'a HudSpriteGpu>,
    text_field_highlighted: Option<&'a HudSpriteGpu>,
    scrollbar_sprites: &'a [Option<HudNineSliceSprite>; HudStatsScrollbarSprite::COUNT],
    tooltip_background: Option<&'a HudNineSliceSprite>,
    tooltip_frame: Option<&'a HudNineSliceSprite>,
    social_sprites: HudSocialInteractionsSprites<'a>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSocialInteractionsMainScreen,
) {
    let layout = hud_social_interactions_layout(
        surface_size,
        screen.rows.len(),
        screen.scroll_amount,
        screen.page == HudSocialInteractionsPage::Blocked,
    );
    let scroll_amount = screen.scroll_amount.clamp(0, layout.max_scroll_amount);
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
        12,
        HUD_TINT_WHITE,
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
        &screen.server_label,
        ((layout.background.x + 5) as f32, 35.0),
        HUD_TINT_WHITE,
    );
    for (index, rect) in layout.tabs.iter().copied().enumerate() {
        let page = page_for_index(index);
        let highlighted = screen.focus == HudSocialInteractionsFocus::Tab(page)
            || screen.hovered == Some(HudSocialInteractionsFocus::Tab(page));
        push_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            rect,
            &screen.tab_labels[index],
            highlighted,
            button_sprites,
        );
        if screen.page == page {
            let text_width = hud_plain_text_width(&screen.tab_labels[index], glyphs) as i32;
            let underline_width = text_width.min(rect.width.saturating_sub(8)).max(1);
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                absolute_hud_rect(
                    (rect.x + (rect.width - underline_width) / 2) as f32,
                    (rect.y + 15) as f32,
                    underline_width as u32,
                    1,
                ),
                full_uv(),
                HUD_TINT_WHITE,
            );
        }
    }
    if let Some(background) = social_sprites.background {
        let rect = absolute_hud_rect(
            layout.background.x as f32,
            layout.background.y as f32,
            layout.background.width.max(0) as u32,
            layout.background.height.max(0) as u32,
        );
        for segment in nine_slice_segments(rect, background.scaling) {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                &background.gpu,
                surface_size,
                segment.rect,
                segment.uv,
                HUD_TINT_WHITE,
            );
        }
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(
                layout.background.x as f32,
                layout.background.y as f32,
                layout.background.width.max(0) as u32,
                layout.background.height.max(0) as u32,
            ),
            full_uv(),
            [0.08, 0.08, 0.08, 0.96],
        );
        push_rect_border(
            vertices,
            commands,
            white_pixel,
            surface_size,
            layout.background,
            [0.65, 0.65, 0.65, 1.0],
        );
    }
    if let Some(search) = social_sprites.search {
        push_hud_draw(
            vertices,
            commands,
            search,
            surface_size,
            rect_to_hud(layout.search_icon),
        );
    }
    let text_field = if screen.focus == HudSocialInteractionsFocus::Search {
        text_field_highlighted.or(text_field)
    } else {
        text_field
    };
    if let Some(text_field) = text_field {
        push_hud_draw(
            vertices,
            commands,
            text_field,
            surface_size,
            rect_to_hud(layout.search),
        );
    }
    push_search_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        screen,
        layout.search,
    );

    if push_hud_rect_scissor(commands, surface_size, rect_to_hud(layout.list_scissor)) {
        for (index, player) in screen.rows.iter().enumerate() {
            let row = row_rect(index, scroll_amount, surface_size);
            if !row.intersects(layout.list_scissor) {
                continue;
            }
            push_player_row(
                renderer,
                vertices,
                commands,
                white_pixel,
                social_sprites.mute,
                social_sprites.mute_highlighted,
                social_sprites.unmute,
                social_sprites.unmute_highlighted,
                font_atlas,
                glyphs,
                obfuscated_pool,
                frame_index,
                surface_size,
                screen,
                player,
                row,
            );
        }
        reset_scissor(commands, surface_size);
    }
    push_scrollbar(
        vertices,
        commands,
        white_pixel,
        scrollbar_sprites,
        surface_size,
        layout,
    );
    if let Some(rect) = layout.blocking_help {
        push_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            rect,
            &screen.blocking_help_label,
            screen.focus == HudSocialInteractionsFocus::BlockingHelp
                || screen.hovered == Some(HudSocialInteractionsFocus::BlockingHelp),
            button_sprites,
        );
    }
    push_button(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        layout.done,
        &screen.done_label,
        screen.focus == HudSocialInteractionsFocus::Done
            || screen.hovered == Some(HudSocialInteractionsFocus::Done),
        button_sprites,
    );
    if let Some(empty) = &screen.empty_text {
        let list_end = layout.list.y + layout.list.height;
        push_hud_debug_options_centered_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            empty,
            (72 + list_end) / 2,
            hud_argb_to_tint(0xFF_AA_AA_AA),
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
}

#[allow(clippy::too_many_arguments)]
fn push_search_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSocialInteractionsMainScreen,
    rect: HudSocialInteractionsRect,
) {
    let selection_start = screen.search_cursor.min(screen.search_selection);
    let selection_end = screen.search_cursor.max(screen.search_selection);
    if selection_start < selection_end {
        let prefix = screen
            .search_text
            .chars()
            .take(selection_start)
            .collect::<String>();
        let selection = screen
            .search_text
            .chars()
            .skip(selection_start)
            .take(selection_end - selection_start)
            .collect::<String>();
        let selection_x = rect.x + 4 + hud_plain_text_width(&prefix, glyphs) as i32;
        let selection_width = hud_plain_text_width(&selection, glyphs).max(1);
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            absolute_hud_rect(selection_x as f32, (rect.y + 3) as f32, selection_width, 9),
            full_uv(),
            [0.0, 0.0, 1.0, 0.65],
        );
    }
    let text = if screen.search_text.is_empty() {
        &screen.search_hint
    } else {
        &screen.search_text
    };
    let tint = if screen.search_text.is_empty() {
        hud_argb_to_tint(0xFF_80_80_80)
    } else {
        HUD_TINT_WHITE
    };
    push_hud_debug_options_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        text,
        ((rect.x + 4) as f32, (rect.y + 4) as f32),
        tint,
    );
    if screen.focus != HudSocialInteractionsFocus::Search
        || !screen.search_cursor_visible
        || screen.search_text.is_empty() && screen.search_cursor != 0
    {
        return;
    }
    let cursor_prefix = screen
        .search_text
        .chars()
        .take(screen.search_cursor)
        .collect::<String>();
    let cursor_x = rect.x + 4 + hud_plain_text_width(&cursor_prefix, glyphs) as i32;
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        absolute_hud_rect(cursor_x as f32, (rect.y + 3) as f32, 1, 9),
        full_uv(),
        HUD_TINT_WHITE,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_player_row<'a>(
    renderer: &'a Renderer,
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    mute: Option<&'a HudSpriteGpu>,
    mute_highlighted: Option<&'a HudSpriteGpu>,
    unmute: Option<&'a HudSpriteGpu>,
    unmute_highlighted: Option<&'a HudSpriteGpu>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSocialInteractionsMainScreen,
    player: &HudSocialInteractionsPlayer,
    row: HudSocialInteractionsRect,
) {
    push_hud_draw_with_uv_and_tint(
        vertices,
        commands,
        white_pixel,
        surface_size,
        rect_to_hud(row),
        full_uv(),
        if player.status.is_some() {
            hud_argb_to_tint(0xFF_30_30_30)
        } else {
            hud_argb_to_tint(0xFF_4A_4A_4A)
        },
    );
    let face = HudSocialInteractionsRect {
        x: row.x + 4,
        y: row.y + (row.height - FACE_SIZE as i32) / 2,
        width: FACE_SIZE as i32,
        height: FACE_SIZE as i32,
    };
    if let Some(sprite) = renderer.hud_player_face(player.skin) {
        push_hud_draw(vertices, commands, sprite, surface_size, rect_to_hud(face));
    }
    if player.removed {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            rect_to_hud(face),
            full_uv(),
            [0.0, 0.0, 0.0, 190.0 / 255.0],
        );
    }
    let text_x = face.x + face.width + 4;
    let text_y = if player.status.is_some() {
        row.y + (row.height - 18) / 2
    } else {
        row.y + (row.height - 9) / 2
    };
    push_hud_debug_options_text(
        vertices,
        commands,
        white_pixel,
        font_atlas,
        glyphs,
        obfuscated_pool,
        frame_index,
        surface_size,
        &player.name,
        (text_x as f32, text_y as f32),
        HUD_TINT_WHITE,
    );
    if let Some(status) = &player.status {
        push_italic_text(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            status,
            (text_x as f32, (text_y + 12) as f32),
            [1.0, 1.0, 1.0, 140.0 / 255.0],
        );
    }
    let Some(action) = player.action else {
        return;
    };
    let focus = HudSocialInteractionsFocus::Player {
        profile_id: player.profile_id,
        action,
    };
    let highlighted = screen.focus == focus || screen.hovered == Some(focus);
    let sprite = match (action, highlighted) {
        (HudSocialInteractionsPlayerAction::Hide, false) => mute,
        (HudSocialInteractionsPlayerAction::Hide, true) => mute_highlighted.or(mute),
        (HudSocialInteractionsPlayerAction::Show, false) => unmute,
        (HudSocialInteractionsPlayerAction::Show, true) => unmute_highlighted.or(unmute),
    };
    let action_rect = row_action_rect(row);
    if let Some(sprite) = sprite {
        push_hud_draw(
            vertices,
            commands,
            sprite,
            surface_size,
            rect_to_hud(action_rect),
        );
    } else {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            rect_to_hud(action_rect),
            full_uv(),
            if highlighted {
                [0.95, 0.95, 0.55, 1.0]
            } else {
                [0.32, 0.32, 0.32, 1.0]
            },
        );
        push_hud_debug_options_centered_text_in_width(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            match action {
                HudSocialInteractionsPlayerAction::Hide => "×",
                HudSocialInteractionsPlayerAction::Show => "✓",
            },
            action_rect.x,
            action_rect.width,
            action_rect.y + 6,
            HUD_TINT_WHITE,
        );
    }
}

fn push_scrollbar<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    sprites: &'a [Option<HudNineSliceSprite>; HudStatsScrollbarSprite::COUNT],
    surface_size: PhysicalSize<u32>,
    layout: HudSocialInteractionsLayout,
) {
    let (Some(background), Some(thumb)) = (layout.scrollbar, layout.scrollbar_thumb) else {
        return;
    };
    for (kind, rect, fallback) in [
        (
            HudStatsScrollbarSprite::Background,
            background,
            [0.0, 0.0, 0.0, 0.45],
        ),
        (
            HudStatsScrollbarSprite::Scroller,
            thumb,
            [0.55, 0.55, 0.55, 0.92],
        ),
    ] {
        if let Some(sprite) = sprites[kind.as_index()].as_ref() {
            for segment in nine_slice_segments(rect_to_hud(rect), sprite.scaling) {
                push_hud_draw_with_uv_and_tint(
                    vertices,
                    commands,
                    &sprite.gpu,
                    surface_size,
                    segment.rect,
                    segment.uv,
                    HUD_TINT_WHITE,
                );
            }
        } else {
            push_hud_draw_with_uv_and_tint(
                vertices,
                commands,
                white_pixel,
                surface_size,
                rect_to_hud(rect),
                full_uv(),
                fallback,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_link_confirmation<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    button_sprites: HudDebugOptionsButtonSprites<'a>,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    screen: &HudSocialInteractionsLinkConfirmationScreen,
) {
    let layout = link_confirmation_layout(screen, glyphs, surface_size);
    let max_width = surface_size.width.saturating_sub(50).max(1);
    for (text, start_y) in [
        (&screen.title, layout.title_y),
        (&screen.message, layout.message_y),
    ] {
        for (index, line) in hud_wrap_plain_text(text, max_width, glyphs)
            .iter()
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
                line,
                start_y + i32::try_from(index).unwrap_or(i32::MAX / 9) * 9,
                HUD_TINT_WHITE,
            );
        }
    }
    for (focus, rect, label) in [
        (
            HudSocialInteractionsFocus::LinkOpen,
            layout.buttons[0],
            &screen.open_label,
        ),
        (
            HudSocialInteractionsFocus::LinkCopy,
            layout.buttons[1],
            &screen.copy_label,
        ),
        (
            HudSocialInteractionsFocus::LinkCancel,
            layout.buttons[2],
            &screen.cancel_label,
        ),
    ] {
        push_button(
            vertices,
            commands,
            white_pixel,
            font_atlas,
            glyphs,
            obfuscated_pool,
            frame_index,
            surface_size,
            rect,
            label,
            screen.focus == focus || screen.hovered == Some(focus),
            button_sprites,
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
    rect: HudSocialInteractionsRect,
    label: &str,
    highlighted: bool,
    sprites: HudDebugOptionsButtonSprites<'a>,
) {
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
        label,
        true,
        highlighted,
        HUD_TINT_WHITE,
        sprites,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_italic_text<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    font_atlas: &'a HudSpriteGpu,
    glyphs: &HudFontGlyphMap,
    obfuscated_pool: &HudObfuscatedGlyphPool,
    frame_index: u64,
    surface_size: PhysicalSize<u32>,
    text: &str,
    origin: (f32, f32),
    tint: [f32; 4],
) {
    let runs = [HudStyledTextRun {
        text: text.to_string(),
        style: HudTextStyle {
            italic: true,
            ..HudTextStyle::default()
        },
        color: None,
    }];
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
            runs: &runs,
            origin,
            scale: 1.0,
            tint,
        },
    );
}

fn push_rect_border<'a>(
    vertices: &mut Vec<HudVertex>,
    commands: &mut Vec<HudDrawCommand<'a>>,
    white_pixel: &'a HudSpriteGpu,
    surface_size: PhysicalSize<u32>,
    rect: HudSocialInteractionsRect,
    tint: [f32; 4],
) {
    for border in [
        HudSocialInteractionsRect {
            width: rect.width,
            height: 1,
            ..rect
        },
        HudSocialInteractionsRect {
            y: rect.y + rect.height - 1,
            width: rect.width,
            height: 1,
            ..rect
        },
        HudSocialInteractionsRect {
            width: 1,
            height: rect.height,
            ..rect
        },
        HudSocialInteractionsRect {
            x: rect.x + rect.width - 1,
            width: 1,
            height: rect.height,
            ..rect
        },
    ] {
        push_hud_draw_with_uv_and_tint(
            vertices,
            commands,
            white_pixel,
            surface_size,
            rect_to_hud(border),
            full_uv(),
            tint,
        );
    }
}

fn rect_to_hud(rect: HudSocialInteractionsRect) -> super::HudRect {
    absolute_hud_rect(
        rect.x as f32,
        rect.y as f32,
        rect.width.max(0) as u32,
        rect.height.max(0) as u32,
    )
}

fn full_uv() -> HudUvRect {
    HudUvRect {
        min: [0.0, 0.0],
        max: [1.0, 1.0],
    }
}

fn reset_scissor(commands: &mut Vec<HudDrawCommand<'_>>, surface_size: PhysicalSize<u32>) {
    commands.push(HudDrawCommand::Scissor {
        x: 0,
        y: 0,
        width: surface_size.width.max(1),
        height: surface_size.height.max(1),
    });
}

impl Renderer {
    pub fn set_hud_social_interactions_screen(
        &mut self,
        screen: Option<HudSocialInteractionsScreen>,
    ) {
        self.hud_social_interactions_screen =
            screen.and_then(sanitize_hud_social_interactions_screen);
    }

    pub fn hud_social_interactions_hit_target_at(
        &self,
        position: (f64, f64),
    ) -> Option<HudSocialInteractionsHitTarget> {
        hud_social_interactions_hit_target_at(
            self.hud_social_interactions_screen.as_ref()?,
            &self.hud_font_glyphs,
            self.surface_size(),
            position,
        )
    }

    pub fn upload_hud_social_interactions_background(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
        scaling: HudNineSliceScaling,
    ) -> anyhow::Result<()> {
        self.hud_social_interactions_background = Some(HudNineSliceSprite {
            gpu: self.upload_hud_sprite(width, height, rgba)?,
            scaling,
        });
        Ok(())
    }

    pub fn upload_hud_social_interactions_search(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<()> {
        self.hud_social_interactions_search = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_social_interactions_mute(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<()> {
        self.hud_social_interactions_mute = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_social_interactions_mute_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<()> {
        self.hud_social_interactions_mute_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_social_interactions_unmute(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<()> {
        self.hud_social_interactions_unmute = Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }

    pub fn upload_hud_social_interactions_unmute_highlighted(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> anyhow::Result<()> {
        self.hud_social_interactions_unmute_highlighted =
            Some(self.upload_hud_sprite(width, height, rgba)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_render_types::EntityDefaultPlayerSkin;

    fn player(
        id: u128,
        action: Option<HudSocialInteractionsPlayerAction>,
    ) -> HudSocialInteractionsPlayer {
        HudSocialInteractionsPlayer {
            profile_id: id.to_be_bytes(),
            name: format!("player-{id}"),
            skin: EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve),
            removed: false,
            hidden: action == Some(HudSocialInteractionsPlayerAction::Show),
            blocked: false,
            action,
            status: None,
        }
    }

    fn screen(rows: usize) -> HudSocialInteractionsScreen {
        HudSocialInteractionsScreen::Main(HudSocialInteractionsMainScreen {
            generation: 7,
            title: "Social Interactions".to_string(),
            server_label: "Server - 2 players".to_string(),
            page: HudSocialInteractionsPage::All,
            tab_labels: [
                "All".to_string(),
                "Hidden".to_string(),
                "Blocked".to_string(),
            ],
            search_text: String::new(),
            search_hint: "Search".to_string(),
            search_cursor: 0,
            search_selection: 0,
            search_cursor_visible: true,
            rows: (0..rows)
                .map(|index| {
                    player(
                        index as u128 + 1,
                        Some(HudSocialInteractionsPlayerAction::Hide),
                    )
                })
                .collect(),
            scroll_amount: 0,
            empty_text: None,
            blocking_help_label: "Blocking help".to_string(),
            done_label: "Done".to_string(),
            focus: HudSocialInteractionsFocus::Search,
            hovered: None,
            tooltip: None,
        })
    }

    #[test]
    fn hud_social_interactions_layout_matches_vanilla_geometry_and_scroll_extent() {
        let size = PhysicalSize::new(320, 240);
        let layout = hud_social_interactions_layout(size, 8, 10_000, true);
        assert_eq!(
            layout.background,
            HudSocialInteractionsRect {
                x: 44,
                y: 64,
                width: 236,
                height: 112
            }
        );
        assert_eq!(
            layout.tabs[0],
            HudSocialInteractionsRect {
                x: 50,
                y: 45,
                width: 73,
                height: 20
            }
        );
        assert_eq!(
            layout.search,
            HudSocialInteractionsRect {
                x: 69,
                y: 74,
                width: 200,
                height: 15
            }
        );
        assert_eq!(layout.list.height, 80);
        assert_eq!(layout.max_scroll_amount, 212);
        assert_eq!(layout.done.y, 213);
        assert_eq!(layout.blocking_help.map(|rect| rect.y), Some(160));
        assert_eq!(layout.scrollbar_thumb.map(|thumb| thumb.y), Some(136));
    }

    #[test]
    fn hud_social_interactions_hit_testing_uses_visible_rows_and_generation() {
        let size = PhysicalSize::new(320, 240);
        let screen = screen(8);
        assert_eq!(
            hud_social_interactions_hit_target_at(
                &screen,
                &HudFontGlyphMap::default(),
                size,
                (55.0, 50.0),
            ),
            Some(HudSocialInteractionsHitTarget {
                generation: 7,
                kind: HudSocialInteractionsHitTargetKind::Focus(HudSocialInteractionsFocus::Tab(
                    HudSocialInteractionsPage::All
                ),),
            })
        );
        assert_eq!(
            hud_social_interactions_hit_target_at(
                &screen,
                &HudFontGlyphMap::default(),
                size,
                (230.0, 100.0),
            )
            .map(|target| target.kind),
            Some(HudSocialInteractionsHitTargetKind::Focus(
                HudSocialInteractionsFocus::Player {
                    profile_id: 1_u128.to_be_bytes(),
                    action: HudSocialInteractionsPlayerAction::Hide,
                },
            ))
        );
        assert!(matches!(
            hud_social_interactions_hit_target_at(
                &screen,
                &HudFontGlyphMap::default(),
                size,
                (281.0, 120.0),
            )
            .map(|target| target.kind),
            Some(HudSocialInteractionsHitTargetKind::Scrollbar { .. })
        ));
    }

    #[test]
    fn hud_social_interactions_sanitizes_search_rows_and_link_focus() {
        let mut main = match screen(MAX_ROWS + 10) {
            HudSocialInteractionsScreen::Main(main) => main,
            _ => unreachable!(),
        };
        main.search_text = "abcdefghijklmnop-extra".to_string();
        main.search_cursor = usize::MAX;
        let Some(HudSocialInteractionsScreen::Main(main)) =
            sanitize_hud_social_interactions_screen(HudSocialInteractionsScreen::Main(main))
        else {
            panic!("sanitized main screen");
        };
        assert_eq!(main.rows.len(), MAX_ROWS);
        assert_eq!(main.search_text, "abcdefghijklmnop");
        assert_eq!(main.search_cursor, 16);

        let link = HudSocialInteractionsLinkConfirmationScreen {
            generation: 4,
            title: String::new(),
            message: String::new(),
            open_label: String::new(),
            copy_label: String::new(),
            cancel_label: String::new(),
            focus: HudSocialInteractionsFocus::Search,
            hovered: Some(HudSocialInteractionsFocus::Done),
        };
        let Some(HudSocialInteractionsScreen::LinkConfirmation(link)) =
            sanitize_hud_social_interactions_screen(HudSocialInteractionsScreen::LinkConfirmation(
                link,
            ))
        else {
            panic!("sanitized link screen");
        };
        assert_eq!(link.focus, HudSocialInteractionsFocus::LinkOpen);
        assert_eq!(link.hovered, None);
    }

    #[test]
    fn hud_social_interactions_collects_after_ordinary_hud_and_before_stats() {
        let source = include_str!("../hud.rs");
        let ordinary_hud = source.find("let mut screen_blur = None;").unwrap();
        let frame_collection = &source[ordinary_hud..];
        let social = frame_collection
            .find("if let Some(screen) = &self.hud_social_interactions_screen")
            .unwrap();
        let stats = frame_collection
            .find("if let Some(screen) = &self.hud_stats_screen")
            .unwrap();
        assert!(social > 0);
        assert!(social < stats);
    }
}
