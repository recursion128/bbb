//! The one `ChatComponent` page geometry: the startup chat options drive the
//! wrap width, the draw pose, every chat fill, every pointer test and the
//! scroll bound.

use super::*;

/// The startup `Options` chat page values (332-363).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudChatOptions {
    pub scale: f64,
    pub width: f64,
    pub height_focused: f64,
    pub height_unfocused: f64,
    pub line_spacing: f64,
    pub opacity: f64,
}

impl Default for HudChatOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            width: 1.0,
            height_focused: 1.0,
            // `ChatComponent.defaultUnfocusedPct`.
            height_unfocused: 70.0 / 160.0,
            line_spacing: 0.0,
            opacity: 1.0,
        }
    }
}

impl HudChatOptions {
    /// Every chat page option is a `UnitDouble`; anything else falls back to
    /// the option's initial value (560-562).
    pub(in crate::hud) fn sanitized(self) -> Self {
        let initial = Self::default();
        let unit = |value: f64, initial: f64| {
            if (0.0..=1.0).contains(&value) {
                value
            } else {
                initial
            }
        };
        Self {
            scale: unit(self.scale, initial.scale),
            width: unit(self.width, initial.width),
            height_focused: unit(self.height_focused, initial.height_focused),
            height_unfocused: unit(self.height_unfocused, initial.height_unfocused),
            line_spacing: unit(self.line_spacing, initial.line_spacing),
            opacity: unit(self.opacity, initial.opacity),
        }
    }

    /// `ChatComponent.getWidth(double)`.
    fn chat_width(self) -> i32 {
        (self.width * 280.0 + 40.0).floor() as i32
    }

    /// `ChatComponent.getLineHeight`, the same
    /// `entryHeight` the draw path computes.
    fn line_height(self) -> i32 {
        (9.0 * (self.line_spacing + 1.0)) as i32
    }

    /// `ChatComponent.getLinesPerPage` over `getHeight(double)`
    /// (419-423, 431-433).
    fn lines_per_page(self, height: f64) -> usize {
        let height = (height * 160.0 + 20.0).floor() as i32;
        usize::try_from(height / self.line_height()).unwrap_or(0)
    }

    /// `ChatComponent.addMessageToDisplayQueue` wraps at a double quotient
    /// floored; the draw path ceils a float one.
    pub(in crate::hud) fn wrap_width(self) -> i32 {
        (f64::from(self.chat_width()) / self.scale).floor() as i32
    }
}

/// The focused page `ChatComponent.scrollChat` clamps against: the wrapped
/// line count and `getLinesPerPage` while `ChatScreen` is open.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudChatScrollPage {
    pub lines: usize,
    pub lines_per_page: usize,
}

impl HudChatScrollPage {
    /// `ChatComponent.scrollChat` in Java int arithmetic.
    pub fn scroll(self, position: usize, dir: i32) -> usize {
        let int = |value: usize| i32::try_from(value).unwrap_or(i32::MAX);
        let top = int(self.lines).wrapping_sub(int(self.lines_per_page));
        let position = int(position).wrapping_add(dir).min(top);
        usize::try_from(position).unwrap_or(0)
    }
}

/// The trimmed lines one chat message update appended: the wrapped line count
/// before them and each appended message's line count, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HudChatAppendedLines {
    pub prior_lines: usize,
    pub message_lines: Vec<usize>,
}

impl HudChatAppendedLines {
    /// The lines one source replacement adds to `trimmedMessages`. A refresh
    /// re-adds every entry to an empty list;
    /// otherwise the entries past the previous newest sequence follow the
    /// previous lines, or none when the list was cleared first
    pub(in crate::hud) fn replaced(
        previous: &[HudChatMessage],
        previous_lines: usize,
        messages: &[HudChatMessage],
        message_lines: &[usize],
        refreshed: bool,
    ) -> Option<Self> {
        let newest = previous.last().map(|message| message.display_sequence);
        let first = if refreshed {
            0
        } else {
            messages
                .iter()
                .position(|message| Some(message.display_sequence) > newest)
                .unwrap_or(messages.len())
        };
        let appended = message_lines.get(first..).unwrap_or_default();
        if appended.is_empty() {
            return None;
        }
        // Keeping no previous entry below the history cap means they were cleared, not trimmed.
        let cleared = first == 0 && messages.len() < HUD_CHAT_MAX_MESSAGES;
        Some(Self {
            prior_lines: if refreshed || cleared {
                0
            } else {
                previous_lines
            },
            message_lines: appended.to_vec(),
        })
    }

    /// A later report over an empty list does not depend on the earlier one;
    /// any other continues it.
    pub(in crate::hud) fn then(mut self, next: Self) -> Self {
        if next.prior_lines == 0 {
            return next;
        }
        self.message_lines.extend(next.message_lines);
        self
    }
}

/// `ChatComponent.chatScrollbarPos` and `newMessageSinceScroll`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudChatScroll {
    pub position: usize,
    pub new_message_since_scroll: bool,
}

impl HudChatScroll {
    /// `ChatComponent.scrollChat`.
    pub fn scrolled(self, page: HudChatScrollPage, dir: i32) -> Self {
        let position = page.scroll(self.position, dir);
        Self {
            position,
            new_message_since_scroll: self.new_message_since_scroll && position > 0,
        }
    }

    /// `addMessageToDisplayQueue` while chat is focused: every appended line
    /// scrolls one line against the count before it, and the lines are trimmed
    /// to 100 after each message.
    pub fn followed(mut self, lines_per_page: usize, appended: &HudChatAppendedLines) -> Self {
        let mut lines = appended.prior_lines;
        for &count in &appended.message_lines {
            for _ in 0..count {
                if self.position == 0 {
                    return self;
                }
                self.new_message_since_scroll = true;
                self = self.scrolled(
                    HudChatScrollPage {
                        lines,
                        lines_per_page,
                    },
                    1,
                );
                lines = lines.saturating_add(1);
            }
            lines = lines.min(HUD_CHAT_MAX_WRAPPED_LINES);
        }
        self
    }
}

/// The subset of JOML `Matrix3x2f` the chat pose uses, evaluated in Java's
/// float order so pointer inverses round exactly as the capture path does.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HudChatPose {
    m00: f32,
    m01: f32,
    m10: f32,
    m11: f32,
    m20: f32,
    m21: f32,
}

impl HudChatPose {
    fn chat(scale: f32) -> Self {
        let mut pose = Self {
            m00: 1.0,
            m01: 0.0,
            m10: 0.0,
            m11: 1.0,
            m20: 0.0,
            m21: 0.0,
        };
        pose.m00 *= scale;
        pose.m01 *= scale;
        pose.m10 *= scale;
        pose.m11 *= scale;
        pose.m20 += pose.m00 * 4.0 + pose.m10 * 0.0;
        pose.m21 += pose.m01 * 4.0 + pose.m11 * 0.0;
        pose
    }

    fn invert(self) -> Self {
        let s = 1.0 / (self.m00 * self.m11 - self.m01 * self.m10);
        Self {
            m00: self.m11 * s,
            m01: -self.m01 * s,
            m10: -self.m10 * s,
            m11: self.m00 * s,
            m20: (self.m10 * self.m21 - self.m20 * self.m11) * s,
            m21: (self.m20 * self.m01 - self.m00 * self.m21) * s,
        }
    }

    fn transform(self, x: f32, y: f32) -> [f32; 2] {
        [
            self.m00 * x + self.m10 * y + self.m20,
            self.m01 * x + self.m11 * y + self.m21,
        ]
    }
}

/// One `fill` of the focused scrollbar: chat-local `[x0, y0, x1, y1]` with
/// `x0 <= x1`, `y0 <= y1`, and its ARGB colour.
pub(in crate::hud) type HudChatScrollbarFill = ([i32; 4], u32);

/// `ChatComponent.extractRenderState`'s page quantities and pose for one set
/// of chat options. Coordinates passed in are
/// chat-local, exactly the integers the Vanilla draw path hands its access.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::hud) struct HudChatGeometry {
    pub(in crate::hud) scale: f32,
    pub(in crate::hud) max_width: i32,
    pub(in crate::hud) chat_bottom: i32,
    pub(in crate::hud) entry_height: i32,
    entry_bottom_to_message_y: i32,
    pub(in crate::hud) text_opacity: f32,
    focused_lines_per_page: usize,
    unfocused_lines_per_page: usize,
    pose: HudChatPose,
    inverse: HudChatPose,
}

impl HudChatGeometry {
    pub(in crate::hud) fn new(options: HudChatOptions, surface_size: PhysicalSize<u32>) -> Self {
        let scale = options.scale as f32;
        let spacing = options.line_spacing;
        let screen_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
        let pose = HudChatPose::chat(scale);
        Self {
            scale,
            max_width: (options.chat_width() as f32 / scale).ceil() as i32,
            chat_bottom: (screen_height.wrapping_sub(HUD_CHAT_BOTTOM_MARGIN) as f32 / scale).floor()
                as i32,
            entry_height: options.line_height(),
            entry_bottom_to_message_y: (8.0 * (spacing + 1.0) - 4.0 * spacing).round() as i32,
            text_opacity: options.opacity as f32 * 0.9 + 0.1,
            focused_lines_per_page: options.lines_per_page(options.height_focused),
            unfocused_lines_per_page: options.lines_per_page(options.height_unfocused),
            pose,
            inverse: pose.invert(),
        }
    }

    pub(in crate::hud) fn lines_per_page(&self, focused: bool) -> usize {
        if focused {
            self.focused_lines_per_page
        } else {
            self.unfocused_lines_per_page
        }
    }

    pub(in crate::hud) fn scroll_page(&self, lines: usize) -> HudChatScrollPage {
        HudChatScrollPage {
            lines,
            lines_per_page: self.focused_lines_per_page,
        }
    }

    /// `entryBottom` of the page row `row` lines above the bottom
    pub(in crate::hud) fn entry_bottom(&self, row: usize) -> i32 {
        self.chat_bottom.wrapping_sub(
            i32::try_from(row)
                .unwrap_or(i32::MAX)
                .wrapping_mul(self.entry_height),
        )
    }

    pub(in crate::hud) fn entry_top(&self, row: usize) -> i32 {
        self.entry_bottom(row).wrapping_sub(self.entry_height)
    }

    pub(in crate::hud) fn text_top(&self, row: usize) -> i32 {
        self.entry_bottom(row)
            .wrapping_sub(self.entry_bottom_to_message_y)
    }

    /// `lineAboveMessagesY` over the drawn row count.
    pub(in crate::hud) fn line_above_messages(&self, drawn_rows: usize) -> i32 {
        self.chat_bottom.wrapping_sub(
            i32::try_from(drawn_rows)
                .unwrap_or(i32::MAX)
                .wrapping_add(1)
                .wrapping_mul(self.entry_height),
        )
    }

    /// The restriction prompt's text top.
    pub(in crate::hud) fn restricted_text_top(&self, drawn_rows: usize) -> i32 {
        self.line_above_messages(drawn_rows)
            .wrapping_add(self.entry_height)
            .wrapping_sub(self.entry_bottom_to_message_y)
            .wrapping_sub(1)
    }

    /// `queueLineBottom - 8`.
    pub(in crate::hud) fn queue_text_top(&self) -> i32 {
        self.chat_bottom
            .wrapping_add(HUD_CHAT_MESSAGE_HEIGHT)
            .wrapping_sub(8)
    }

    /// The two scrollbar fills of the focused page,
    /// in Java int arithmetic.
    pub(in crate::hud) fn scrollbar(
        &self,
        total: usize,
        drawn_rows: usize,
        scroll: HudChatScroll,
    ) -> Option<[HudChatScrollbarFill; 2]> {
        let int = |value: usize| i32::try_from(value).unwrap_or(i32::MAX);
        let total = int(total);
        if total <= 0 {
            return None;
        }
        let chat_height = int(drawn_rows).wrapping_mul(self.entry_height);
        let virtual_height = total.wrapping_mul(self.entry_height);
        let y = int(scroll.position)
            .wrapping_mul(chat_height)
            .wrapping_div(total)
            .wrapping_sub(self.chat_bottom);
        let height = chat_height
            .wrapping_mul(chat_height)
            .checked_div(virtual_height)?;
        if virtual_height == chat_height {
            return None;
        }
        let alpha: u32 = if y > 0 { 170 } else { 96 };
        let color = if scroll.new_message_since_scroll {
            13_382_451
        } else {
            3_355_562
        };
        let start = self.max_width.wrapping_add(4);
        let fill = |x0: i32, x1: i32, rgb: u32| {
            let (y0, y1) = (y.wrapping_neg(), y.wrapping_neg().wrapping_sub(height));
            (
                [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)],
                alpha << 24 | rgb,
            )
        };
        Some([
            fill(start, start.wrapping_add(2), color),
            fill(start.wrapping_add(2), start.wrapping_add(1), 13_421_772),
        ])
    }

    pub(in crate::hud) fn screen_point(&self, x: f32, y: f32) -> [f32; 2] {
        self.pose.transform(x, y)
    }

    /// A chat-local `fill(x0, y0, x1, y1)` through the pose, in the
    /// `[top_left, bottom_left, bottom_right, top_right]` quad order.
    pub(in crate::hud) fn fill_corners(&self, [x0, y0, x1, y1]: [i32; 4]) -> [[f32; 2]; 4] {
        let (x0, y0, x1, y1) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
        [
            self.pose.transform(x0, y0),
            self.pose.transform(x0, y1),
            self.pose.transform(x1, y1),
            self.pose.transform(x1, y0),
        ]
    }

    /// The integer GUI pointer in chat-local space.
    pub(in crate::hud) fn local_point(&self, pointer: (i32, i32)) -> [f32; 2] {
        self.inverse.transform(pointer.0 as f32, pointer.1 as f32)
    }

    /// `DrawingFocusedGraphicsAccess.isMouseOver`.
    pub(in crate::hud) fn local_rect_contains(
        &self,
        pointer: (i32, i32),
        [x0, y0, x1, y1]: [i32; 4],
    ) -> bool {
        let [x, y] = self.local_point(pointer);
        x >= x0 as f32 && x < x1 as f32 && y >= y0 as f32 && y < y1 as f32
    }

    /// The prepared bounds of a chat line accepted at `(0, text_top)`, moved
    /// through `ScreenRectangle.transformMaxBounds` and tested with
    /// `containsPoint`.
    pub(in crate::hud) fn text_bounds_contain(
        &self,
        bounds: Option<[f32; 4]>,
        text_top: i32,
        pointer: (i32, i32),
    ) -> bool {
        let Some([left, top, right, bottom]) = bounds else {
            return false;
        };
        let (left, right) = (left.floor() as i32, right.ceil() as i32);
        let (top, bottom) = (
            (top.floor() as i32).wrapping_add(text_top),
            (bottom.ceil() as i32).wrapping_add(text_top),
        );
        let corners = [
            self.pose.transform(left as f32, top as f32),
            self.pose.transform(right as f32, top as f32),
            self.pose.transform(left as f32, bottom as f32),
            self.pose.transform(right as f32, bottom as f32),
        ];
        let axis = |index: usize, fold: fn(f32, f32) -> f32, start: f32| {
            corners.iter().map(|corner| corner[index]).fold(start, fold)
        };
        let (min_x, max_x) = (axis(0, f32::min, f32::MAX), axis(0, f32::max, f32::MIN));
        let (min_y, max_y) = (axis(1, f32::min, f32::MAX), axis(1, f32::max, f32::MIN));
        let screen_left = min_x.floor() as i32;
        let screen_top = min_y.floor() as i32;
        let screen_right = screen_left.wrapping_add((max_x - min_x).ceil() as i32);
        let screen_bottom = screen_top.wrapping_add((max_y - min_y).ceil() as i32);
        pointer.0 >= screen_left
            && pointer.0 < screen_right
            && pointer.1 >= screen_top
            && pointer.1 < screen_bottom
    }

    /// `ActiveTextCollector.findElementUnderCursor` over one text accepted at
    /// `(0, text_top)`.
    pub(in crate::hud) fn active_area_hits<P: Copy>(
        &self,
        text: &HudStyledTextGeometry,
        text_top: i32,
        pointer: (i32, i32),
        candidate: impl Fn(usize) -> Option<(P, bool, bool)>,
        hits: &mut HudActiveAreaHits<P>,
    ) -> bool {
        if !self.text_bounds_contain(text.text_bounds, text_top, pointer) {
            return false;
        }
        let [x, y] = self.local_point(pointer);
        hud_active_area_hits(text, (x, y - text_top as f32), candidate, hits)
    }

    /// `ChatComponent.forEachLine`: the drawn page
    /// rows as `(row, line_index, alpha)`, top row first. A faded line keeps its row.
    pub(in crate::hud) fn visible_rows(
        &self,
        lines: &[HudChatLine],
        context: &HudChatContext,
    ) -> Vec<(usize, usize, f32)> {
        let mut rows = lines
            .iter()
            .enumerate()
            .rev()
            .skip(context.scroll.position)
            .take(self.lines_per_page(context.focused))
            .enumerate()
            .map(|(row, (line_index, line))| (row, line_index, hud_chat_line_alpha(context, line)))
            .filter(|(_, _, alpha)| *alpha > 1.0e-5)
            .collect::<Vec<_>>();
        rows.reverse();
        rows
    }
}

/// The restriction prompt `handleMessage` receives: the red underlined
/// restrictions link, clipped with a plain ellipsis and given its hover text
/// when wider than the page (ChatComponent,
/// ComponentRenderUtils, StringSplitter).
pub(in crate::hud) struct HudChatPrompt {
    pub(in crate::hud) runs: Vec<HudComponentTextRun>,
    pub(in crate::hud) text: HudStyledTextGeometry,
    pub(in crate::hud) clipped: bool,
}

pub(in crate::hud) fn hud_chat_restricted_prompt(
    restricted_text: &str,
    fonts: &HudFontGlyphBook,
    max_width: i32,
) -> HudChatPrompt {
    let style = HudTextStyle {
        underlined: true,
        ..HudTextStyle::default()
    };
    let mut runs = hud_coded_string_runs_with_style(restricted_text, style)
        .into_iter()
        .map(|mut run| {
            run.color = run.color.or(Some(0xFF_5555));
            HudComponentTextRun::from(run)
        })
        .collect::<Vec<_>>();
    let clipped = hud_component_text_width(&runs, fonts) > max_width;
    if clipped {
        let glyphs = fonts.glyphs(None);
        let mut remaining = max_width.wrapping_sub(hud_raw_string_width("...", glyphs)) as f32;
        let mut kept = Vec::new();
        for run in &runs {
            let mut text = String::new();
            let mut stopped = false;
            for ch in run.visual.text.chars() {
                remaining -= hud_font_glyph(ch, glyphs).styled_advance_f32(run.visual.style);
                if remaining < 0.0 {
                    stopped = true;
                    break;
                }
                text.push(ch);
            }
            if !text.is_empty() {
                kept.push(HudComponentTextRun {
                    visual: HudStyledTextRun {
                        text,
                        ..run.visual.clone()
                    },
                    font: run.font.clone(),
                });
            }
            if stopped {
                break;
            }
        }
        kept.push(HudComponentTextRun::plain("..."));
        runs = kept;
    }
    let ellipsis_chars = if clipped { 3 } else { 0 };
    let chars = runs
        .iter()
        .map(|run| run.visual.text.chars().count())
        .sum::<usize>();
    let targets = (0..chars)
        .map(|index| (index + ellipsis_chars < chars).then_some(0))
        .collect::<Vec<_>>();
    HudChatPrompt {
        text: hud_component_text_hit_geometry(&runs, &targets, fonts, true),
        runs,
        clipped,
    }
}

/// `Component.translatable("chat.queue", queueSize)` with the queue expand
/// click on every glyph (204-208).
pub(in crate::hud) fn hud_chat_queue_prompt(
    queue_text: &str,
    fonts: &HudFontGlyphBook,
) -> HudChatPrompt {
    let runs = hud_coded_string_runs(queue_text)
        .into_iter()
        .map(HudComponentTextRun::from)
        .collect::<Vec<_>>();
    let chars = runs
        .iter()
        .map(|run| run.visual.text.chars().count())
        .sum::<usize>();
    HudChatPrompt {
        text: hud_component_text_hit_geometry(&runs, &vec![Some(0); chars], fonts, true),
        runs,
        clipped: false,
    }
}
