use super::*;

impl Renderer {
    pub fn hud_plain_text_cursor_for_width(&self, text: &str, width: i32) -> Option<usize> {
        hud_plain_text_cursor_for_width(text, width, &self.pack.hud_font_glyphs)
    }

    /// Measures ordinary HUD text with the active font atlas.
    pub fn hud_raw_string_width(&self, text: &str) -> i32 {
        hud_raw_string_width(text, &self.pack.hud_font_glyphs)
    }

    pub fn hud_plain_text_cursor_for_width_from(
        &self,
        text: &str,
        display_start: usize,
        width: i32,
    ) -> Option<usize> {
        hud_plain_text_cursor_for_width_from(text, display_start, width, &self.pack.hud_font_glyphs)
    }

    pub fn hud_plain_text_display_start_for_width(
        &self,
        text: &str,
        scroll_to: usize,
        width: i32,
    ) -> Option<usize> {
        hud_plain_text_display_start_for_width(text, scroll_to, width, &self.pack.hud_font_glyphs)
    }

    /// Clones the pure font-advance projection used by input layout. The
    /// simulation owner may keep this value; it contains no renderer/GPU
    /// resource and is refreshed after resource reloads.
    pub fn hud_text_measurements(&self) -> HudFontGlyphMap {
        self.pack.hud_font_glyphs.clone()
    }

    pub fn hud_text_measurements_ref(&self) -> &HudFontGlyphMap {
        &self.pack.hud_font_glyphs
    }

    pub fn hud_font_book_ref(&self) -> &HudFontGlyphBook {
        &self.pack.hud_font_book
    }
}
