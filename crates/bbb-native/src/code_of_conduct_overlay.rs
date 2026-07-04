use bbb_control::{CodeOfConductControlRequest, SharedControlRequests};
use bbb_world::{CodeOfConductState, WorldStore};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton},
};

const OVERLAY_WIDTH: u32 = 500;
const OVERLAY_HEIGHT: u32 = 300;
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

#[derive(Debug, Default)]
pub(crate) struct CodeOfConductOverlayState {
    rendered_hash: Option<i32>,
    dismissed_hash: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayAction {
    Accept,
    Remember,
    Decline,
}

#[derive(Debug, Clone, Copy)]
struct OverlayRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl CodeOfConductOverlayState {
    pub(crate) fn update_renderer(
        &mut self,
        renderer: &mut bbb_renderer::Renderer,
        world: &WorldStore,
        accepted_current_code_of_conduct: bool,
    ) {
        if accepted_current_code_of_conduct {
            if let Some(code_of_conduct) = world.last_code_of_conduct() {
                self.dismissed_hash = Some(code_of_conduct.text_hash);
            }
        }
        let Some(code_of_conduct) = self.visible_code_of_conduct(world) else {
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

    pub(crate) fn handle_mouse_input(
        &mut self,
        world: &WorldStore,
        control_requests: &SharedControlRequests,
        button: MouseButton,
        state: ElementState,
        cursor: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(code_of_conduct) = self.visible_code_of_conduct(world) else {
            return false;
        };

        if button != MouseButton::Left || state != ElementState::Pressed {
            return true;
        };

        let Some(cursor) = cursor else {
            return true;
        };
        let Some(action) = hit_test_overlay_action(cursor, surface_size) else {
            return true;
        };

        let request = match action {
            OverlayAction::Accept => CodeOfConductControlRequest::Accept { remember: false },
            OverlayAction::Remember => CodeOfConductControlRequest::Accept { remember: true },
            OverlayAction::Decline => CodeOfConductControlRequest::Decline,
        };
        let Ok(mut guard) = control_requests.lock() else {
            return true;
        };
        guard.code_of_conduct_requests.push(request);
        self.dismissed_hash = Some(code_of_conduct.text_hash);
        true
    }

    pub(crate) fn is_visible(&self, world: &WorldStore) -> bool {
        self.visible_code_of_conduct(world).is_some()
    }

    fn visible_code_of_conduct<'a>(&self, world: &'a WorldStore) -> Option<&'a CodeOfConductState> {
        let code_of_conduct = world.last_code_of_conduct()?;
        (self.dismissed_hash != Some(code_of_conduct.text_hash)).then_some(code_of_conduct)
    }
}

fn hit_test_overlay_action(
    cursor: PhysicalPosition<f64>,
    surface_size: PhysicalSize<u32>,
) -> Option<OverlayAction> {
    let (origin_x, origin_y) = overlay_origin(surface_size);
    let x = cursor.x - origin_x;
    let y = cursor.y - origin_y;
    if ACCEPT_BUTTON.contains(x, y) {
        Some(OverlayAction::Accept)
    } else if REMEMBER_BUTTON.contains(x, y) {
        Some(OverlayAction::Remember)
    } else if DECLINE_BUTTON.contains(x, y) {
        Some(OverlayAction::Decline)
    } else {
        None
    }
}

fn overlay_origin(surface_size: PhysicalSize<u32>) -> (f64, f64) {
    (
        (f64::from(surface_size.width.max(1)) - f64::from(OVERLAY_WIDTH)) * 0.5,
        (f64::from(surface_size.height.max(1)) - f64::from(OVERLAY_HEIGHT)) * 0.5,
    )
}

impl OverlayRect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && y >= f64::from(self.y)
            && x < f64::from(self.x + self.width)
            && y < f64::from(self.y + self.height)
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
    use bbb_world::WorldStore;

    #[test]
    fn wraps_and_truncates_code_of_conduct_text() {
        let lines = wrap_code_of_conduct_text(
            "Keep chat friendly and do not grief other players on this server.",
            16,
            2,
        );

        assert_eq!(
            lines,
            vec!["KEEP CHAT".to_string(), "FRIENDLY AND...".to_string()]
        );
    }

    #[test]
    fn hit_test_maps_overlay_buttons_from_surface_coordinates() {
        let surface = PhysicalSize::new(800, 600);
        let (origin_x, origin_y) = overlay_origin(surface);

        assert_eq!(
            hit_test_overlay_action(
                PhysicalPosition::new(origin_x + 30.0, origin_y + 252.0),
                surface,
            ),
            Some(OverlayAction::Accept)
        );
        assert_eq!(
            hit_test_overlay_action(
                PhysicalPosition::new(origin_x + 180.0, origin_y + 252.0),
                surface,
            ),
            Some(OverlayAction::Remember)
        );
        assert_eq!(
            hit_test_overlay_action(
                PhysicalPosition::new(origin_x + 380.0, origin_y + 252.0),
                surface,
            ),
            Some(OverlayAction::Decline)
        );
        assert_eq!(
            hit_test_overlay_action(
                PhysicalPosition::new(origin_x + 250.0, origin_y + 220.0),
                surface,
            ),
            None
        );
    }

    #[test]
    fn click_queues_accept_request_and_dismisses_current_overlay() {
        let mut world = WorldStore::new();
        world.apply_code_of_conduct("Keep chat friendly.".to_string());
        let requests = bbb_control::SharedControlRequests::default();
        let mut overlay = CodeOfConductOverlayState::default();
        let surface = PhysicalSize::new(800, 600);
        let (origin_x, origin_y) = overlay_origin(surface);

        assert!(overlay.handle_mouse_input(
            &world,
            &requests,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(origin_x + 30.0, origin_y + 252.0)),
            surface,
        ));

        assert_eq!(
            requests.lock().unwrap().code_of_conduct_requests,
            vec![CodeOfConductControlRequest::Accept { remember: false }]
        );
        assert!(overlay.visible_code_of_conduct(&world).is_none());
    }

    #[test]
    fn click_queues_remember_and_decline_requests() {
        let surface = PhysicalSize::new(800, 600);
        let (origin_x, origin_y) = overlay_origin(surface);

        let mut remember_world = WorldStore::new();
        remember_world.apply_code_of_conduct("Remember this server.".to_string());
        let remember_requests = bbb_control::SharedControlRequests::default();
        let mut remember_overlay = CodeOfConductOverlayState::default();
        assert!(remember_overlay.handle_mouse_input(
            &remember_world,
            &remember_requests,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(origin_x + 180.0, origin_y + 252.0)),
            surface,
        ));
        assert_eq!(
            remember_requests.lock().unwrap().code_of_conduct_requests,
            vec![CodeOfConductControlRequest::Accept { remember: true }]
        );

        let mut decline_world = WorldStore::new();
        decline_world.apply_code_of_conduct("Decline this server.".to_string());
        let decline_requests = bbb_control::SharedControlRequests::default();
        let mut decline_overlay = CodeOfConductOverlayState::default();
        assert!(decline_overlay.handle_mouse_input(
            &decline_world,
            &decline_requests,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(origin_x + 380.0, origin_y + 252.0)),
            surface,
        ));
        assert_eq!(
            decline_requests.lock().unwrap().code_of_conduct_requests,
            vec![CodeOfConductControlRequest::Decline]
        );
    }

    #[test]
    fn rendered_overlay_has_expected_size_and_visible_pixels() {
        let state = CodeOfConductState {
            text: "Keep chat friendly.".to_string(),
            text_hash: bbb_world::code_of_conduct_text_hash("Keep chat friendly."),
        };

        let rgba = render_code_of_conduct_overlay(&state);

        assert_eq!(rgba.len(), (OVERLAY_WIDTH * OVERLAY_HEIGHT * 4) as usize);
        assert!(rgba.chunks_exact(4).any(|pixel| pixel[3] > 0));
        assert!(rgba.chunks_exact(4).any(|pixel| pixel == TEXT_PRIMARY));
    }
}
