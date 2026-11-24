use super::*;

pub(super) struct SimulationClipboard<'a> {
    pub(super) read: Option<String>,
    pub(super) effects: &'a RefCell<Vec<PlatformEffect>>,
}

impl DebugClipboard for SimulationClipboard<'_> {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        self.effects
            .borrow_mut()
            .push(PlatformEffect::ClipboardWrite(text.into()));
        true
    }

    fn set_debug_clipboard_java_text(&mut self, text: &bbb_protocol::JavaString) -> bool {
        self.effects
            .borrow_mut()
            .push(PlatformEffect::ClipboardWrite(text.clone()));
        true
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        self.read.take()
    }
}

pub(super) struct SimulationTerrainReload<'a> {
    pub(super) effects: &'a RefCell<Vec<PlatformEffect>>,
}

impl TerrainReloadRequestSink for SimulationTerrainReload<'_> {
    fn request_reload_all_chunks(&mut self) {
        self.effects
            .borrow_mut()
            .push(PlatformEffect::ReloadTerrain);
    }
}

impl CommandSuggestionTextMeasurer for HudFontGlyphMap {
    fn command_suggestion_glyphs(&self) -> &HudFontGlyphMap {
        self
    }

    fn chat_input_glyphs(&self) -> &HudFontGlyphMap {
        self.non_fishy()
    }
}

impl DialogTextMeasurer for HudFontGlyphMap {
    fn glyphs(&self) -> &HudFontGlyphMap {
        self
    }

    fn width(&self, text: &str) -> i32 {
        self.plain_text_width(text)
    }

    fn display_start(&self, text: &str, scroll_to: usize, width: i32) -> usize {
        self.plain_text_display_start_for_width(text, scroll_to, width)
            .unwrap_or(0)
    }

    fn cursor_for_width(&self, text: &str, display_start: usize, width: i32) -> usize {
        self.plain_text_cursor_for_width_from(text, display_start, width)
            .unwrap_or(display_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_render::{hud_coded_string_runs, hud_coded_string_width};

    #[test]
    fn native_font_measurers_preserve_fractional_negative_and_zero_advances() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [('A', 0.5), ('B', -2.25), ('C', 6.0), ('Z', 0.0)] {
            glyphs.insert_first_wins(
                ch,
                bbb_client_render::HudAsciiGlyph {
                    advance,
                    ..Default::default()
                },
            );
        }
        assert_eq!(
            hud_coded_string_width(
                &hud_coded_string_runs("AA"),
                glyphs.command_suggestion_glyphs()
            ),
            1
        );
        assert_eq!(DialogTextMeasurer::width(&glyphs, "CBA"), 5);
        assert_eq!(DialogTextMeasurer::width(&glyphs, "B"), -2);
        assert_eq!(
            DialogTextMeasurer::cursor_for_width(&glyphs, "BAAZ", 0, 0),
            4
        );
        assert_eq!(DialogTextMeasurer::display_start(&glyphs, "AAB", 3, 0), 0);
    }

    #[test]
    fn non_fishy_native_font_measurers_preserve_ordinary_negative_advances() {
        let mut glyphs = HudFontGlyphMap::new();
        let any = bbb_client_render::HudAsciiGlyph {
            advance: -2.25,
            ..Default::default()
        };
        let filtered = bbb_client_render::HudAsciiGlyph {
            advance: 4.0,
            ..any
        };
        glyphs.insert_selected_codepoint_first_wins('A' as u32, any, filtered, -2.25);
        let cloned = glyphs.clone();
        let runs = hud_coded_string_runs("AA");
        assert_eq!(hud_coded_string_width(&runs, cloned.chat_input_glyphs()), 8);
        assert_eq!(
            hud_coded_string_width(&runs, cloned.command_suggestion_glyphs()),
            -4
        );
        assert_eq!(DialogTextMeasurer::width(&cloned, "AA"), -4);
    }

    #[test]
    fn fixed_width_measurer_uses_six_pixel_advances() {
        let measurer = crate::input::FallbackTextMeasurer;
        assert_eq!(
            hud_coded_string_width(
                &hud_coded_string_runs("abc"),
                measurer.command_suggestion_glyphs()
            ),
            18
        );
        assert_eq!(
            hud_coded_string_width(
                &hud_coded_string_runs(""),
                measurer.command_suggestion_glyphs()
            ),
            0
        );
    }
}
