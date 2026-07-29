use bbb_control::CodeOfConductControlRequest;
use bbb_renderer::HudFontGlyphMap;
use bbb_world::{CodeOfConductState, WorldStore};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey},
};

const OVERLAY_WIDTH: u32 = 500;
const OVERLAY_HEIGHT: u32 = 300;
const TITLE: &str = "Server Code of Conduct";
const CHECKBOX_LABEL: &str = "Do not notify again for this Code of Conduct";
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
const PANEL_COLOR: [u8; 4] = [18, 22, 28, 235];
const PANEL_BORDER: [u8; 4] = [126, 151, 176, 255];
const BUTTON_COLOR: [u8; 4] = [45, 64, 78, 255];
const BUTTON_HOVERLESS_BORDER: [u8; 4] = [171, 192, 211, 255];
const ACCEPT_COLOR: [u8; 4] = [105, 202, 132, 255];
const REMEMBER_COLOR: [u8; 4] = [108, 176, 231, 255];
const DECLINE_COLOR: [u8; 4] = [233, 116, 106, 255];
const TEXT_PRIMARY: [u8; 4] = [245, 248, 250, 255];
const TEXT_SECONDARY: [u8; 4] = [196, 207, 218, 255];

const ACCEPT_BUTTON: OverlayRect = OverlayRect {
    x: 24,
    y: 246,
    width: 120,
    height: 32,
};
const REMEMBER_BUTTON: OverlayRect = OverlayRect {
    x: 154,
    y: 246,
    width: 190,
    height: 32,
};
const DECLINE_BUTTON: OverlayRect = OverlayRect {
    x: 354,
    y: 246,
    width: 122,
    height: 32,
};

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

#[derive(Debug, Default)]
pub(crate) struct CodeOfConductOverlayRenderer {
    rendered_hash: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CodeOfConductOverlayInput {
    pub(crate) consumed: bool,
    pub(crate) request: Option<CodeOfConductControlRequest>,
}

#[derive(Debug, Clone, Copy)]
struct OverlayRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
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
            let vertical = match delta {
                MouseScrollDelta::LineDelta(_, vertical) => f64::from(vertical),
                MouseScrollDelta::PixelDelta(delta) => delta.y,
            };
            self.scroll_amount -= vertical * TEXT_AREA_SCROLL_RATE;
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

impl CodeOfConductOverlayRenderer {
    pub(crate) fn update_renderer(
        &mut self,
        renderer: &mut bbb_renderer::Renderer,
        world: &WorldStore,
        projection: CodeOfConductOverlayProjection,
        accepted_current_code_of_conduct: bool,
    ) {
        let accepted_hash = accepted_current_code_of_conduct
            .then(|| world.last_code_of_conduct().map(|state| state.text_hash))
            .flatten();
        let dismissed_hash = accepted_hash.or(projection.dismissed_hash);
        let Some(code_of_conduct) = visible_code_of_conduct(world, dismissed_hash) else {
            if self.rendered_hash.is_some() {
                renderer.clear_hud_code_of_conduct_overlay();
                self.rendered_hash = None;
            }
            return;
        };
        if self.rendered_hash == Some(code_of_conduct.text_hash) {
            return;
        }

        let rgba = render_code_of_conduct_overlay(code_of_conduct);
        if let Err(err) =
            renderer.set_hud_code_of_conduct_overlay(OVERLAY_WIDTH, OVERLAY_HEIGHT, &rgba)
        {
            tracing::warn!(?err, "failed to upload code-of-conduct overlay");
        }
        self.rendered_hash = Some(code_of_conduct.text_hash);
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

fn render_code_of_conduct_overlay(code_of_conduct: &CodeOfConductState) -> Vec<u8> {
    let mut rgba = vec![0; (OVERLAY_WIDTH * OVERLAY_HEIGHT * 4) as usize];
    draw_rect(
        &mut rgba,
        OverlayRect {
            x: 0,
            y: 0,
            width: OVERLAY_WIDTH,
            height: OVERLAY_HEIGHT,
        },
        PANEL_COLOR,
    );
    draw_border(
        &mut rgba,
        OverlayRect {
            x: 0,
            y: 0,
            width: OVERLAY_WIDTH,
            height: OVERLAY_HEIGHT,
        },
        PANEL_BORDER,
    );

    draw_text(&mut rgba, 24, 22, "SERVER CODE OF CONDUCT", 2, TEXT_PRIMARY);
    draw_text(
        &mut rgba,
        24,
        46,
        "REVIEW THE SERVER RULES BEFORE JOINING.",
        1,
        TEXT_SECONDARY,
    );
    draw_text(
        &mut rgba,
        24,
        62,
        &format!("HASH {}", code_of_conduct.text_hash),
        1,
        TEXT_SECONDARY,
    );

    for (index, line) in wrap_code_of_conduct_text(&code_of_conduct.text, 38, 8)
        .iter()
        .enumerate()
    {
        draw_text(&mut rgba, 24, 88 + index as u32 * 17, line, 2, TEXT_PRIMARY);
    }

    draw_button(&mut rgba, ACCEPT_BUTTON, "ACCEPT", ACCEPT_COLOR);
    draw_button(&mut rgba, REMEMBER_BUTTON, "REMEMBER", REMEMBER_COLOR);
    draw_button(&mut rgba, DECLINE_BUTTON, "DECLINE", DECLINE_COLOR);

    rgba
}

fn draw_button(rgba: &mut [u8], rect: OverlayRect, label: &str, accent: [u8; 4]) {
    draw_rect(rgba, rect, BUTTON_COLOR);
    draw_border(rgba, rect, BUTTON_HOVERLESS_BORDER);
    draw_rect(
        rgba,
        OverlayRect {
            x: rect.x,
            y: rect.y,
            width: 4,
            height: rect.height,
        },
        accent,
    );
    let text_width = text_width(label, 2);
    let text_x = rect.x + rect.width.saturating_sub(text_width) / 2;
    let text_y = rect.y + rect.height.saturating_sub(14) / 2;
    draw_text(rgba, text_x, text_y, label, 2, TEXT_PRIMARY);
}

fn draw_rect(rgba: &mut [u8], rect: OverlayRect, color: [u8; 4]) {
    let x_end = (rect.x + rect.width).min(OVERLAY_WIDTH);
    let y_end = (rect.y + rect.height).min(OVERLAY_HEIGHT);
    for y in rect.y.min(OVERLAY_HEIGHT)..y_end {
        for x in rect.x.min(OVERLAY_WIDTH)..x_end {
            put_pixel(rgba, x, y, color);
        }
    }
}

fn draw_border(rgba: &mut [u8], rect: OverlayRect, color: [u8; 4]) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    draw_rect(
        rgba,
        OverlayRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: 1,
        },
        color,
    );
    draw_rect(
        rgba,
        OverlayRect {
            x: rect.x,
            y: rect.y + rect.height - 1,
            width: rect.width,
            height: 1,
        },
        color,
    );
    draw_rect(
        rgba,
        OverlayRect {
            x: rect.x,
            y: rect.y,
            width: 1,
            height: rect.height,
        },
        color,
    );
    draw_rect(
        rgba,
        OverlayRect {
            x: rect.x + rect.width - 1,
            y: rect.y,
            width: 1,
            height: rect.height,
        },
        color,
    );
}

fn draw_text(rgba: &mut [u8], x: u32, y: u32, text: &str, scale: u32, color: [u8; 4]) {
    let mut cursor_x = x;
    let advance = 6 * scale;
    for ch in text.chars() {
        draw_char(rgba, cursor_x, y, ch, scale, color);
        cursor_x = cursor_x.saturating_add(advance);
    }
}

fn draw_char(rgba: &mut [u8], x: u32, y: u32, ch: char, scale: u32, color: [u8; 4]) {
    let rows = glyph_rows(ch);
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..5 {
            if bits & (1 << (4 - col)) == 0 {
                continue;
            }
            let px = x + col * scale;
            let py = y + row as u32 * scale;
            draw_rect(
                rgba,
                OverlayRect {
                    x: px,
                    y: py,
                    width: scale,
                    height: scale,
                },
                color,
            );
        }
    }
}

fn put_pixel(rgba: &mut [u8], x: u32, y: u32, color: [u8; 4]) {
    if x >= OVERLAY_WIDTH || y >= OVERLAY_HEIGHT {
        return;
    }
    let index = ((y * OVERLAY_WIDTH + x) * 4) as usize;
    rgba[index..index + 4].copy_from_slice(&color);
}

fn text_width(text: &str, scale: u32) -> u32 {
    text.chars().count() as u32 * 6 * scale
}

fn wrap_code_of_conduct_text(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let normalized = normalize_display_text(text);
    if normalized.is_empty() {
        return vec!["NO SERVER TEXT WAS PROVIDED.".to_string()];
    }

    let chars = normalized.chars().collect::<Vec<_>>();
    let mut lines = Vec::new();
    let mut start = 0usize;
    while start < chars.len() && lines.len() < max_lines {
        let raw_end = (start + max_chars).min(chars.len());
        let mut end = raw_end;
        if raw_end < chars.len() {
            if let Some(space) = chars[start..raw_end].iter().rposition(|ch| *ch == ' ') {
                if space > 0 {
                    end = start + space;
                }
            }
        }
        let line = chars[start..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        if !line.is_empty() {
            lines.push(line);
        }
        start = end;
        while start < chars.len() && chars[start] == ' ' {
            start += 1;
        }
    }

    if start < chars.len() && !lines.is_empty() {
        let last = lines.last_mut().unwrap();
        while last.chars().count() + 3 > max_chars {
            last.pop();
        }
        while last.ends_with(' ') {
            last.pop();
        }
        last.push_str("...");
    }
    lines
}

fn normalize_display_text(text: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
            continue;
        }
        let ch = if ch.is_ascii() && !ch.is_control() {
            ch.to_ascii_uppercase()
        } else {
            '?'
        };
        out.push(ch);
        last_was_space = false;
    }
    out.trim().to_string()
}

fn glyph_rows(ch: char) -> [u32; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'B' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        'C' => [0x0f, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0f],
        'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        'F' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        'G' => [0x0f, 0x10, 0x10, 0x13, 0x11, 0x11, 0x0f],
        'H' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'I' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1f],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0e],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        'Q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0a],
        'X' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        '0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        '1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
        '2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        '3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
        '4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        '5' => [0x1f, 0x10, 0x10, 0x1e, 0x01, 0x01, 0x1e],
        '6' => [0x0f, 0x10, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        '7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        '9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x01, 0x1e],
        '&' => [0x0c, 0x12, 0x14, 0x08, 0x15, 0x12, 0x0d],
        '-' => [0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x0c, 0x04, 0x08],
        ':' => [0x00, 0x0c, 0x0c, 0x00, 0x0c, 0x0c, 0x00],
        ';' => [0x00, 0x0c, 0x0c, 0x00, 0x0c, 0x04, 0x08],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '?' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        '\'' => [0x04, 0x04, 0x08, 0x00, 0x00, 0x00, 0x00],
        '"' => [0x0a, 0x0a, 0x0a, 0x00, 0x00, 0x00, 0x00],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '[' => [0x0e, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0e],
        ']' => [0x0e, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0e],
        '+' => [0x00, 0x04, 0x04, 0x1f, 0x04, 0x04, 0x00],
        '=' => [0x00, 0x00, 0x1f, 0x00, 0x1f, 0x00, 0x00],
        ' ' => [0x00; 7],
        _ => glyph_rows('?'),
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
}
