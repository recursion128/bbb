use bbb_control::CodeOfConductControlRequest;
use bbb_renderer::HudFontGlyphMap;
use bbb_world::{CodeOfConductState, WorldStore};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

const TITLE: &str = "Server Code of Conduct";
const CHECKBOX_LABEL: &str = "Do not notify again for this Code of Conduct";
const ACKNOWLEDGE_LABEL: &str = "Acknowledge";
const DISCONNECT_LABEL: &str = "Disconnect";
const FONT_LINE_HEIGHT: i32 = 9;
const CONTENT_SPACING: i32 = 8;
const MESSAGE_CELL_PADDING: i32 = 12;
const TEXT_AREA_PADDING: i32 = 4;
const TEXT_AREA_SCROLL_RATE: f64 = 9.0;
const SCROLLBAR_WIDTH: i32 = 6;
const SCROLLBAR_MIN_HEIGHT: i32 = 32;
const CHECKBOX_BOX_SIZE: i32 = 17;
const CHECKBOX_LABEL_SPACING: i32 = 4;
const BUTTON_WIDTH: i32 = 150;
const BUTTON_HEIGHT: i32 = 20;
const BUTTON_SPACING: i32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeOfConductFocus {
    Message,
    Checkbox,
    Acknowledge,
    Disconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeOfConductHover {
    Message,
    Scrollbar,
    Checkbox,
    Acknowledge,
    Disconnect,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CodeOfConductRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeOfConductScreenLayout {
    pub(crate) title: CodeOfConductRect,
    pub(crate) message: CodeOfConductRect,
    pub(crate) message_scissor: CodeOfConductRect,
    pub(crate) message_lines: Vec<String>,
    pub(crate) message_content_height: i32,
    pub(crate) max_scroll_amount: i32,
    pub(crate) scrollbar_track: Option<CodeOfConductRect>,
    pub(crate) scrollbar_thumb: Option<CodeOfConductRect>,
    pub(crate) checkbox: CodeOfConductRect,
    pub(crate) checkbox_lines: Vec<String>,
    pub(crate) acknowledge: CodeOfConductRect,
    pub(crate) disconnect: CodeOfConductRect,
}

#[derive(Debug, Default)]
pub(crate) struct CodeOfConductOverlayState {
    active_hash: Option<i32>,
    dismissed_hash: Option<i32>,
    checkbox_selected: bool,
    scroll_amount: f64,
    dragging_scrollbar: bool,
    focus: Option<CodeOfConductFocus>,
    hover: Option<CodeOfConductHover>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct CodeOfConductOverlayProjection {
    pub(crate) active_hash: Option<i32>,
    dismissed_hash: Option<i32>,
    pub(crate) checkbox_selected: bool,
    pub(crate) scroll_amount: f64,
    pub(crate) dragging_scrollbar: bool,
    pub(crate) focus: Option<CodeOfConductFocus>,
    pub(crate) hover: Option<CodeOfConductHover>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CodeOfConductOverlayInput {
    pub(crate) consumed: bool,
    pub(crate) request: Option<CodeOfConductControlRequest>,
}

impl CodeOfConductOverlayState {
    pub(crate) fn projection(&self) -> CodeOfConductOverlayProjection {
        CodeOfConductOverlayProjection {
            active_hash: self.active_hash,
            dismissed_hash: self.dismissed_hash,
            checkbox_selected: self.checkbox_selected,
            scroll_amount: self.scroll_amount,
            dragging_scrollbar: self.dragging_scrollbar,
            focus: self.focus,
            hover: self.hover,
        }
    }

    pub(crate) fn sync(
        &mut self,
        world: &WorldStore,
        surface_size: PhysicalSize<u32>,
        glyphs: &HudFontGlyphMap,
    ) {
        let Some(code_of_conduct) = self.visible_code_of_conduct(world) else {
            self.active_hash = None;
            self.dragging_scrollbar = false;
            self.hover = None;
            return;
        };
        if self.active_hash != Some(code_of_conduct.text_hash) {
            self.active_hash = Some(code_of_conduct.text_hash);
            self.checkbox_selected = false;
            self.scroll_amount = 0.0;
            self.dragging_scrollbar = false;
            self.focus = Some(CodeOfConductFocus::Message);
            self.hover = None;
        }
        self.clamp_scroll(code_of_conduct, surface_size, glyphs);
    }

    pub(crate) fn handle_cursor_moved(
        &mut self,
        world: &WorldStore,
        previous_cursor: Option<PhysicalPosition<f64>>,
        cursor: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        glyphs: &HudFontGlyphMap,
    ) -> bool {
        self.sync(world, surface_size, glyphs);
        let Some(code_of_conduct) = self.visible_code_of_conduct(world) else {
            return false;
        };
        let layout = code_of_conduct_screen_layout(
            &code_of_conduct.text,
            surface_size,
            glyphs,
            self.scroll_amount,
        );
        if self.dragging_scrollbar {
            if let (Some(previous), Some(cursor)) = (previous_cursor, cursor) {
                if cursor.y < f64::from(layout.message.y) {
                    self.scroll_amount = 0.0;
                } else if cursor.y > f64::from(layout.message.bottom()) {
                    self.scroll_amount = f64::from(layout.max_scroll_amount);
                } else if let Some(thumb) = layout.scrollbar_thumb {
                    let drag_range = (layout.message.height - thumb.height).max(1);
                    let scale = (f64::from(layout.max_scroll_amount.max(1))
                        / f64::from(drag_range))
                    .max(1.0);
                    self.scroll_amount += (cursor.y - previous.y) * scale;
                }
                self.clamp_scroll(code_of_conduct, surface_size, glyphs);
            }
        }
        self.hover = cursor.and_then(|cursor| code_of_conduct_hover(&layout, cursor));
        true
    }

    pub(crate) fn handle_mouse_input(
        &mut self,
        world: &WorldStore,
        button: MouseButton,
        state: ElementState,
        cursor: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        glyphs: &HudFontGlyphMap,
    ) -> CodeOfConductOverlayInput {
        self.sync(world, surface_size, glyphs);
        let Some(code_of_conduct) = self.visible_code_of_conduct(world) else {
            return CodeOfConductOverlayInput {
                consumed: false,
                request: None,
            };
        };

        if button != MouseButton::Left {
            return CodeOfConductOverlayInput {
                consumed: true,
                request: None,
            };
        };
        if state == ElementState::Released {
            self.dragging_scrollbar = false;
            return CodeOfConductOverlayInput {
                consumed: true,
                request: None,
            };
        }

        let Some(cursor) = cursor else {
            return CodeOfConductOverlayInput {
                consumed: true,
                request: None,
            };
        };
        let layout = code_of_conduct_screen_layout(
            &code_of_conduct.text,
            surface_size,
            glyphs,
            self.scroll_amount,
        );
        self.hover = code_of_conduct_hover(&layout, cursor);
        let request = match self.hover {
            Some(CodeOfConductHover::Message) => {
                self.focus = Some(CodeOfConductFocus::Message);
                None
            }
            Some(CodeOfConductHover::Scrollbar) => {
                self.focus = Some(CodeOfConductFocus::Message);
                self.dragging_scrollbar = layout.max_scroll_amount > 0;
                None
            }
            Some(CodeOfConductHover::Checkbox) => {
                self.focus = Some(CodeOfConductFocus::Checkbox);
                self.checkbox_selected = !self.checkbox_selected;
                None
            }
            Some(CodeOfConductHover::Acknowledge) => {
                self.focus = Some(CodeOfConductFocus::Acknowledge);
                Some(CodeOfConductControlRequest::Accept {
                    remember: self.checkbox_selected,
                })
            }
            Some(CodeOfConductHover::Disconnect) => {
                self.focus = Some(CodeOfConductFocus::Disconnect);
                Some(CodeOfConductControlRequest::Decline)
            }
            None => None,
        };
        if request.is_some() {
            self.dismissed_hash = Some(code_of_conduct.text_hash);
            self.active_hash = None;
            self.dragging_scrollbar = false;
            self.hover = None;
        }
        CodeOfConductOverlayInput {
            consumed: true,
            request,
        }
    }

    pub(crate) fn handle_mouse_wheel(
        &mut self,
        world: &WorldStore,
        delta: MouseScrollDelta,
        cursor: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        glyphs: &HudFontGlyphMap,
    ) -> bool {
        self.sync(world, surface_size, glyphs);
        let Some(code_of_conduct) = self.visible_code_of_conduct(world) else {
            return false;
        };
        let layout = code_of_conduct_screen_layout(
            &code_of_conduct.text,
            surface_size,
            glyphs,
            self.scroll_amount,
        );
        if cursor.is_some_and(|cursor| layout.message_with_scrollbar_contains(cursor)) {
            let delta_pixels = match delta {
                MouseScrollDelta::LineDelta(_, vertical) => {
                    f64::from(vertical) * TEXT_AREA_SCROLL_RATE
                }
                MouseScrollDelta::PixelDelta(delta) => delta.y,
            };
            self.scroll_amount -= delta_pixels;
            self.clamp_scroll(code_of_conduct, surface_size, glyphs);
        }
        true
    }

    pub(crate) fn handle_key_input(
        &mut self,
        world: &WorldStore,
        physical_key: PhysicalKey,
        state: ElementState,
        surface_size: PhysicalSize<u32>,
        glyphs: &HudFontGlyphMap,
    ) -> CodeOfConductOverlayInput {
        self.sync(world, surface_size, glyphs);
        let Some(code_of_conduct) = self.visible_code_of_conduct(world) else {
            return CodeOfConductOverlayInput {
                consumed: false,
                request: None,
            };
        };
        if state != ElementState::Pressed {
            return CodeOfConductOverlayInput {
                consumed: true,
                request: None,
            };
        }
        let PhysicalKey::Code(key) = physical_key else {
            return CodeOfConductOverlayInput {
                consumed: true,
                request: None,
            };
        };

        let mut request = None;
        match key {
            KeyCode::Escape => {}
            KeyCode::Tab => self.focus_next(true),
            KeyCode::ArrowUp if self.focus == Some(CodeOfConductFocus::Message) => {
                let previous = self.scroll_amount;
                self.scroll_amount -= TEXT_AREA_SCROLL_RATE;
                self.clamp_scroll(code_of_conduct, surface_size, glyphs);
                if self.scroll_amount == previous {
                    self.focus_next(false);
                }
            }
            KeyCode::ArrowDown if self.focus == Some(CodeOfConductFocus::Message) => {
                let previous = self.scroll_amount;
                self.scroll_amount += TEXT_AREA_SCROLL_RATE;
                self.clamp_scroll(code_of_conduct, surface_size, glyphs);
                if self.scroll_amount == previous {
                    self.focus_next(true);
                }
            }
            KeyCode::ArrowUp | KeyCode::ArrowLeft => self.focus_next(false),
            KeyCode::ArrowDown | KeyCode::ArrowRight => self.focus_next(true),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => match self.focus {
                Some(CodeOfConductFocus::Checkbox) => {
                    self.checkbox_selected = !self.checkbox_selected;
                }
                Some(CodeOfConductFocus::Acknowledge) => {
                    request = Some(CodeOfConductControlRequest::Accept {
                        remember: self.checkbox_selected,
                    });
                }
                Some(CodeOfConductFocus::Disconnect) => {
                    request = Some(CodeOfConductControlRequest::Decline);
                }
                Some(CodeOfConductFocus::Message) | None => {}
            },
            _ => {}
        }
        if request.is_some() {
            self.dismissed_hash = Some(code_of_conduct.text_hash);
            self.active_hash = None;
            self.dragging_scrollbar = false;
            self.hover = None;
        }
        CodeOfConductOverlayInput {
            consumed: true,
            request,
        }
    }

    pub(crate) fn handle_cursor_left(&mut self) {
        self.hover = None;
    }

    pub(crate) fn is_visible(&self, world: &WorldStore) -> bool {
        self.visible_code_of_conduct(world).is_some()
    }

    fn clamp_scroll(
        &mut self,
        code_of_conduct: &CodeOfConductState,
        surface_size: PhysicalSize<u32>,
        glyphs: &HudFontGlyphMap,
    ) {
        let layout = code_of_conduct_screen_layout(
            &code_of_conduct.text,
            surface_size,
            glyphs,
            self.scroll_amount,
        );
        self.scroll_amount = self
            .scroll_amount
            .clamp(0.0, f64::from(layout.max_scroll_amount));
    }

    fn focus_next(&mut self, forward: bool) {
        const ORDER: [CodeOfConductFocus; 4] = [
            CodeOfConductFocus::Message,
            CodeOfConductFocus::Checkbox,
            CodeOfConductFocus::Acknowledge,
            CodeOfConductFocus::Disconnect,
        ];
        let current = self
            .focus
            .and_then(|focus| ORDER.iter().position(|entry| *entry == focus));
        let next = match (current, forward) {
            (Some(index), true) => (index + 1) % ORDER.len(),
            (Some(0), false) | (None, false) => ORDER.len() - 1,
            (Some(index), false) => index - 1,
            (None, true) => 0,
        };
        self.focus = Some(ORDER[next]);
    }

    fn visible_code_of_conduct<'a>(&self, world: &'a WorldStore) -> Option<&'a CodeOfConductState> {
        visible_code_of_conduct(world, self.dismissed_hash)
    }
}

pub(crate) fn code_of_conduct_renderer_screen(
    world: &WorldStore,
    projection: CodeOfConductOverlayProjection,
    accepted_current_code_of_conduct: bool,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
) -> Option<bbb_renderer::HudCodeOfConductScreen> {
    let accepted_hash = accepted_current_code_of_conduct
        .then(|| world.last_code_of_conduct().map(|state| state.text_hash))
        .flatten();
    let code_of_conduct =
        visible_code_of_conduct(world, accepted_hash.or(projection.dismissed_hash))?;
    let projection_is_current = projection.active_hash == Some(code_of_conduct.text_hash);
    let scroll_amount = projection_is_current
        .then_some(projection.scroll_amount)
        .unwrap_or_default();
    let layout =
        code_of_conduct_screen_layout(&code_of_conduct.text, surface_size, glyphs, scroll_amount);
    let focus = projection_is_current.then_some(projection.focus).flatten();
    let hover = projection_is_current.then_some(projection.hover).flatten();

    Some(bbb_renderer::HudCodeOfConductScreen {
        text_hash: code_of_conduct.text_hash,
        title: TITLE.to_string(),
        message_lines: layout
            .message_lines
            .into_iter()
            .map(|line| vec![bbb_renderer::HudStyledTextRun::plain(line)])
            .collect(),
        message_content_height: layout.message_content_height,
        title_rect: renderer_rect(layout.title),
        message_rect: renderer_rect(layout.message),
        message_scissor: renderer_rect(layout.message_scissor),
        scroll_amount: scroll_amount.clamp(0.0, f64::from(layout.max_scroll_amount)) as i32,
        max_scroll_amount: layout.max_scroll_amount,
        scrollbar_track: layout.scrollbar_track.map(renderer_rect),
        scrollbar_thumb: layout.scrollbar_thumb.map(renderer_rect),
        message_focused: focus == Some(CodeOfConductFocus::Message),
        scrollbar_hovered: hover == Some(CodeOfConductHover::Scrollbar),
        scrollbar_dragging: projection_is_current && projection.dragging_scrollbar,
        checkbox_rect: renderer_rect(layout.checkbox),
        checkbox_label: CHECKBOX_LABEL.to_string(),
        checkbox_label_lines: layout.checkbox_lines,
        checkbox_selected: projection_is_current && projection.checkbox_selected,
        checkbox_focused: focus == Some(CodeOfConductFocus::Checkbox),
        checkbox_hovered: hover == Some(CodeOfConductHover::Checkbox),
        acknowledge_rect: renderer_rect(layout.acknowledge),
        acknowledge_label: ACKNOWLEDGE_LABEL.to_string(),
        acknowledge_highlighted: focus == Some(CodeOfConductFocus::Acknowledge)
            || hover == Some(CodeOfConductHover::Acknowledge),
        disconnect_rect: renderer_rect(layout.disconnect),
        disconnect_label: DISCONNECT_LABEL.to_string(),
        disconnect_highlighted: focus == Some(CodeOfConductFocus::Disconnect)
            || hover == Some(CodeOfConductHover::Disconnect),
    })
}

fn renderer_rect(rect: CodeOfConductRect) -> bbb_renderer::HudCodeOfConductRect {
    bbb_renderer::HudCodeOfConductRect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

fn visible_code_of_conduct(
    world: &WorldStore,
    dismissed_hash: Option<i32>,
) -> Option<&CodeOfConductState> {
    let code_of_conduct = world.last_code_of_conduct()?;
    (dismissed_hash != Some(code_of_conduct.text_hash)).then_some(code_of_conduct)
}

pub(crate) fn code_of_conduct_screen_layout(
    text: &str,
    surface_size: PhysicalSize<u32>,
    glyphs: &HudFontGlyphMap,
    scroll_amount: f64,
) -> CodeOfConductScreenLayout {
    let screen_width = i32::try_from(surface_size.width.max(1)).unwrap_or(i32::MAX);
    let screen_height = i32::try_from(surface_size.height.max(1)).unwrap_or(i32::MAX);
    let bounded_message_width = screen_width.saturating_sub(MESSAGE_CELL_PADDING * 2).max(1);
    let message_width = screen_width
        .saturating_sub(100)
        .max(1)
        .min(bounded_message_width);
    let requested_message_height = screen_height.saturating_sub(100).max(1);
    let message_lines = wrap_vanilla_plain_text(
        text,
        u32::try_from(message_width.saturating_sub(TEXT_AREA_PADDING * 2).max(1)).unwrap_or(1),
        glyphs,
    );
    let message_content_height = i32::try_from(message_lines.len())
        .unwrap_or(i32::MAX)
        .saturating_mul(FONT_LINE_HEIGHT)
        .saturating_add(TEXT_AREA_PADDING * 2);
    let message_height = requested_message_height.min(message_content_height.max(1));
    let max_scroll_amount = message_content_height.saturating_sub(message_height).max(0);

    let title_width = styled_text_width(TITLE, true, glyphs)
        .min(u32::try_from(screen_width).unwrap_or(u32::MAX)) as i32;
    let checkbox_default_width = CHECKBOX_BOX_SIZE
        .saturating_add(CHECKBOX_LABEL_SPACING)
        .saturating_add(i32::try_from(glyphs.plain_text_width(CHECKBOX_LABEL)).unwrap_or(i32::MAX));
    let checkbox_width = checkbox_default_width
        .min(screen_width.saturating_sub(16).max(1))
        .max(1);
    let checkbox_label_width = checkbox_width
        .saturating_sub(CHECKBOX_BOX_SIZE + CHECKBOX_LABEL_SPACING)
        .max(1);
    let checkbox_lines =
        wrap_vanilla_plain_text(CHECKBOX_LABEL, checkbox_label_width as u32, glyphs);
    let checkbox_height = CHECKBOX_BOX_SIZE.max(
        i32::try_from(checkbox_lines.len().min(2))
            .unwrap_or(2)
            .saturating_mul(FONT_LINE_HEIGHT),
    );

    let nominal_button_row_width = BUTTON_WIDTH * 2 + BUTTON_SPACING;
    let button_row_width = nominal_button_row_width.min(screen_width).max(1);
    let button_width = if button_row_width >= BUTTON_SPACING + 2 {
        (button_row_width - BUTTON_SPACING) / 2
    } else {
        button_row_width / 2
    }
    .max(1);
    let actual_button_spacing = button_row_width
        .saturating_sub(button_width.saturating_mul(2))
        .max(0);
    let footer_width = checkbox_width.max(button_row_width);
    let footer_height = checkbox_height
        .saturating_add(CONTENT_SPACING)
        .saturating_add(BUTTON_HEIGHT);
    let message_cell_width = message_width.saturating_add(MESSAGE_CELL_PADDING * 2);
    let message_cell_height = message_height.saturating_add(MESSAGE_CELL_PADDING * 2);
    let content_width = title_width
        .max(message_cell_width)
        .max(footer_width)
        .min(screen_width);
    let content_height = FONT_LINE_HEIGHT
        .saturating_add(CONTENT_SPACING)
        .saturating_add(message_cell_height)
        .saturating_add(CONTENT_SPACING)
        .saturating_add(footer_height);
    let content_x = screen_width.saturating_sub(content_width) / 2;
    let content_y = screen_height.saturating_sub(content_height) / 2;

    let title = CodeOfConductRect {
        x: content_x + (content_width - title_width) / 2,
        y: content_y,
        width: title_width,
        height: FONT_LINE_HEIGHT,
    };
    let message = CodeOfConductRect {
        x: content_x + (content_width - message_width) / 2,
        y: content_y + FONT_LINE_HEIGHT + CONTENT_SPACING + MESSAGE_CELL_PADDING,
        width: message_width,
        height: message_height,
    };
    let footer_y =
        content_y + FONT_LINE_HEIGHT + CONTENT_SPACING + message_cell_height + CONTENT_SPACING;
    let checkbox = CodeOfConductRect {
        x: content_x + (content_width - checkbox_width) / 2,
        y: footer_y,
        width: checkbox_width,
        height: checkbox_height,
    };
    let button_row_x = content_x + (content_width - button_row_width) / 2;
    let button_y = footer_y + checkbox_height + CONTENT_SPACING;
    let acknowledge = CodeOfConductRect {
        x: button_row_x,
        y: button_y,
        width: button_width,
        height: BUTTON_HEIGHT,
    };
    let disconnect = CodeOfConductRect {
        x: button_row_x + button_width + actual_button_spacing,
        y: button_y,
        width: button_width,
        height: BUTTON_HEIGHT,
    };
    let message_scissor = CodeOfConductRect {
        x: message.x + 1,
        y: message.y + 1,
        width: message.width.saturating_sub(2).max(0),
        height: message.height.saturating_sub(2).max(0),
    };
    let (scrollbar_track, scrollbar_thumb) = if max_scroll_amount > 0 {
        let track = CodeOfConductRect {
            x: message.right(),
            y: message.y,
            width: SCROLLBAR_WIDTH,
            height: message.height,
        };
        let maximum_thumb_height = message.height.saturating_sub(8).max(1);
        let minimum_thumb_height = SCROLLBAR_MIN_HEIGHT.min(maximum_thumb_height);
        let proportional_height = message
            .height
            .saturating_mul(message.height)
            .checked_div(message_content_height.max(1))
            .unwrap_or(maximum_thumb_height);
        let thumb_height = proportional_height.clamp(minimum_thumb_height, maximum_thumb_height);
        let clamped_scroll = scroll_amount.clamp(0.0, f64::from(max_scroll_amount)) as i32;
        let thumb_y = message.y
            + clamped_scroll.saturating_mul(message.height - thumb_height)
                / max_scroll_amount.max(1);
        (
            Some(track),
            Some(CodeOfConductRect {
                x: track.x,
                y: thumb_y.max(message.y),
                width: track.width,
                height: thumb_height,
            }),
        )
    } else {
        (None, None)
    };

    CodeOfConductScreenLayout {
        title,
        message,
        message_scissor,
        message_lines,
        message_content_height,
        max_scroll_amount,
        scrollbar_track,
        scrollbar_thumb,
        checkbox,
        checkbox_lines,
        acknowledge,
        disconnect,
    }
}

fn code_of_conduct_hover(
    layout: &CodeOfConductScreenLayout,
    cursor: PhysicalPosition<f64>,
) -> Option<CodeOfConductHover> {
    if layout
        .scrollbar_track
        .is_some_and(|track| track.contains(cursor))
    {
        Some(CodeOfConductHover::Scrollbar)
    } else if layout.message.contains(cursor) {
        Some(CodeOfConductHover::Message)
    } else if layout.checkbox.contains(cursor) {
        Some(CodeOfConductHover::Checkbox)
    } else if layout.acknowledge.contains(cursor) {
        Some(CodeOfConductHover::Acknowledge)
    } else if layout.disconnect.contains(cursor) {
        Some(CodeOfConductHover::Disconnect)
    } else {
        None
    }
}

fn styled_text_width(text: &str, bold: bool, glyphs: &HudFontGlyphMap) -> u32 {
    let style = bbb_renderer::HudTextStyle {
        bold,
        ..bbb_renderer::HudTextStyle::default()
    };
    text.chars().fold(0u32, |width, ch| {
        width.saturating_add(glyphs.styled_char_advance(ch, style))
    })
}

fn wrap_vanilla_plain_text(text: &str, max_width: u32, glyphs: &HudFontGlyphMap) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut begin = 0usize;
    while begin < chars.len() {
        let mut cursor = begin;
        let mut last_space = None;
        let mut wrapped = false;
        while cursor < chars.len() {
            let ch = chars[cursor];
            if ch == '\n' {
                lines.push(chars[begin..cursor].iter().collect());
                begin = cursor + 1;
                wrapped = true;
                break;
            }
            if ch == ' ' {
                last_space = Some(cursor);
            }
            let candidate = chars[begin..=cursor].iter().collect::<String>();
            if cursor > begin && glyphs.plain_text_width(&candidate) > max_width.max(1) {
                if let Some(space) = last_space.filter(|space| *space >= begin) {
                    lines.push(chars[begin..space].iter().collect());
                    begin = space + 1;
                } else {
                    lines.push(chars[begin..cursor].iter().collect());
                    begin = cursor;
                }
                wrapped = true;
                break;
            }
            cursor += 1;
        }
        if !wrapped {
            lines.push(chars[begin..].iter().collect());
            begin = chars.len();
        }
    }
    if text.ends_with('\n') {
        lines.push(String::new());
    }
    lines
}

impl CodeOfConductScreenLayout {
    fn message_with_scrollbar_contains(&self, cursor: PhysicalPosition<f64>) -> bool {
        self.message.contains(cursor)
            || self
                .scrollbar_track
                .is_some_and(|track| track.contains(cursor))
    }
}

impl CodeOfConductRect {
    pub(crate) fn right(self) -> i32 {
        self.x.saturating_add(self.width)
    }

    pub(crate) fn bottom(self) -> i32 {
        self.y.saturating_add(self.height)
    }

    fn contains(self, cursor: PhysicalPosition<f64>) -> bool {
        cursor.x >= f64::from(self.x)
            && cursor.y >= f64::from(self.y)
            && cursor.x < f64::from(self.right())
            && cursor.y < f64::from(self.bottom())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_renderer::HudAsciiGlyph;
    use bbb_world::WorldStore;

    fn glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        let glyph = HudAsciiGlyph {
            width: 5,
            height: 8,
            advance: 6,
            ..HudAsciiGlyph::default()
        };
        for ch in (32u8..=126).map(char::from).chain("规则友好".chars()) {
            glyphs.insert_first_wins(ch, glyph);
        }
        glyphs
    }

    fn center(rect: CodeOfConductRect) -> PhysicalPosition<f64> {
        PhysicalPosition::new(
            f64::from(rect.x) + f64::from(rect.width) * 0.5,
            f64::from(rect.y) + f64::from(rect.height) * 0.5,
        )
    }

    #[test]
    fn ordinary_layout_matches_warning_screen_grid() {
        let layout = code_of_conduct_screen_layout(
            "Keep chat friendly.",
            PhysicalSize::new(800, 600),
            &glyphs(),
            0.0,
        );
        assert_eq!(
            layout.message,
            CodeOfConductRect {
                x: 50,
                y: 273,
                width: 700,
                height: 17,
            }
        );
        assert_eq!(layout.title.y, 244);
        assert_eq!(
            layout.acknowledge,
            CodeOfConductRect {
                x: 246,
                y: 335,
                width: 150,
                height: 20,
            }
        );
        assert_eq!(layout.disconnect.x, 404);
        assert_eq!(layout.max_scroll_amount, 0);
        assert!(layout.scrollbar_track.is_none());
        assert_eq!(layout.message_lines, ["Keep chat friendly."]);
    }

    #[test]
    fn compact_and_scrolling_layouts_are_bounded_and_keep_exact_text() {
        let compact = code_of_conduct_screen_layout(
            "规则\nKeep Mixed Case.",
            PhysicalSize::new(320, 240),
            &glyphs(),
            0.0,
        );
        assert_eq!(compact.message.width, 220);
        assert_eq!(
            compact.message_lines,
            ["规则".to_string(), "Keep Mixed Case.".to_string()]
        );
        for rect in [
            compact.title,
            compact.message,
            compact.checkbox,
            compact.acknowledge,
            compact.disconnect,
        ] {
            assert!(rect.x >= 0);
            assert!(rect.y >= 0);
            assert!(rect.right() <= 320);
            assert!(rect.bottom() <= 240);
        }

        let long_text = "Keep chat friendly and respect every player. ".repeat(80);
        let scrolling =
            code_of_conduct_screen_layout(&long_text, PhysicalSize::new(320, 240), &glyphs(), 27.0);
        assert!(scrolling.max_scroll_amount > 27);
        assert!(scrolling.scrollbar_track.is_some());
        assert!(scrolling.scrollbar_thumb.is_some());
        assert!(scrolling.message_lines.concat().contains("Keep"));
    }

    #[test]
    fn checkbox_value_flows_into_acknowledge_and_new_hash_resets_state() {
        let surface = PhysicalSize::new(800, 600);
        let glyphs = glyphs();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep chat friendly.".to_string());
        let layout = code_of_conduct_screen_layout("Keep chat friendly.", surface, &glyphs, 0.0);
        let mut overlay = CodeOfConductOverlayState::default();
        overlay.sync(&world, surface, &glyphs);

        let checked = overlay.handle_mouse_input(
            &world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(center(layout.checkbox)),
            surface,
            &glyphs,
        );
        assert_eq!(checked.request, None);
        assert!(overlay.checkbox_selected);
        assert_eq!(overlay.focus, Some(CodeOfConductFocus::Checkbox));

        let accepted = overlay.handle_mouse_input(
            &world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(center(layout.acknowledge)),
            surface,
            &glyphs,
        );
        assert_eq!(
            accepted.request,
            Some(CodeOfConductControlRequest::Accept { remember: true })
        );
        assert!(!overlay.is_visible(&world));

        world.apply_code_of_conduct("A later exact code.".to_string());
        overlay.sync(&world, surface, &glyphs);
        assert!(overlay.is_visible(&world));
        assert!(!overlay.checkbox_selected);
        assert_eq!(overlay.scroll_amount, 0.0);
        assert_eq!(overlay.focus, Some(CodeOfConductFocus::Message));
    }

    #[test]
    fn keyboard_focus_activation_and_escape_match_modal_screen() {
        let surface = PhysicalSize::new(800, 600);
        let glyphs = glyphs();
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep chat friendly.".to_string());
        let mut overlay = CodeOfConductOverlayState::default();

        let escape = overlay.handle_key_input(
            &world,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed,
            surface,
            &glyphs,
        );
        assert!(escape.consumed);
        assert_eq!(escape.request, None);
        assert!(overlay.is_visible(&world));

        for key in [KeyCode::Tab, KeyCode::Space, KeyCode::Tab] {
            let input = overlay.handle_key_input(
                &world,
                PhysicalKey::Code(key),
                ElementState::Pressed,
                surface,
                &glyphs,
            );
            assert_eq!(input.request, None);
        }
        assert!(overlay.checkbox_selected);
        assert_eq!(overlay.focus, Some(CodeOfConductFocus::Acknowledge));
        let accepted = overlay.handle_key_input(
            &world,
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
            surface,
            &glyphs,
        );
        assert_eq!(
            accepted.request,
            Some(CodeOfConductControlRequest::Accept { remember: true })
        );
    }

    #[test]
    fn wheel_and_scrollbar_drag_clamp_to_derived_scroll_range() {
        let surface = PhysicalSize::new(320, 240);
        let glyphs = glyphs();
        let text = "One long rule for every connected player. ".repeat(80);
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.clone());
        let mut overlay = CodeOfConductOverlayState::default();
        let layout = code_of_conduct_screen_layout(&text, surface, &glyphs, 0.0);
        let message_cursor = center(layout.message);
        assert!(overlay.handle_mouse_wheel(
            &world,
            MouseScrollDelta::LineDelta(0.0, -3.0),
            Some(message_cursor),
            surface,
            &glyphs,
        ));
        assert_eq!(overlay.scroll_amount, 27.0);

        let track = layout.scrollbar_track.unwrap();
        overlay.handle_mouse_input(
            &world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(center(track)),
            surface,
            &glyphs,
        );
        assert!(overlay.dragging_scrollbar);
        overlay.handle_cursor_moved(
            &world,
            Some(center(track)),
            Some(PhysicalPosition::new(
                f64::from(track.x),
                f64::from(layout.message.bottom() + 10),
            )),
            surface,
            &glyphs,
        );
        assert_eq!(overlay.scroll_amount, f64::from(layout.max_scroll_amount));
        overlay.handle_mouse_input(
            &world,
            MouseButton::Left,
            ElementState::Released,
            None,
            surface,
            &glyphs,
        );
        assert!(!overlay.dragging_scrollbar);
    }

    #[test]
    fn renderer_projection_preserves_exact_text_and_interaction_flags() {
        let surface = PhysicalSize::new(800, 600);
        let glyphs = glyphs();
        let text = "规则 Keep Mixed Case.";
        let mut world = WorldStore::new();
        world.apply_code_of_conduct(text.to_string());
        let layout = code_of_conduct_screen_layout(text, surface, &glyphs, 0.0);
        let mut overlay = CodeOfConductOverlayState::default();
        overlay.handle_mouse_input(
            &world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(center(layout.checkbox)),
            surface,
            &glyphs,
        );
        overlay.handle_cursor_moved(
            &world,
            None,
            Some(center(layout.acknowledge)),
            surface,
            &glyphs,
        );

        let screen =
            code_of_conduct_renderer_screen(&world, overlay.projection(), false, surface, &glyphs)
                .unwrap();

        assert_eq!(screen.message_lines[0][0].text, text);
        assert!(screen.checkbox_selected);
        assert!(screen.checkbox_focused);
        assert!(screen.acknowledge_highlighted);
        assert!(code_of_conduct_renderer_screen(
            &world,
            overlay.projection(),
            true,
            surface,
            &glyphs,
        )
        .is_none());
    }
}
