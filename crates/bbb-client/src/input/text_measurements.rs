pub(crate) trait CommandSuggestionTextMeasurer {
    fn chat_input_glyphs(&self) -> &bbb_client_render::HudFontGlyphMap;
    fn command_suggestion_glyphs(&self) -> &bbb_client_render::HudFontGlyphMap;
}

impl CommandSuggestionTextMeasurer for bbb_client_render::Renderer {
    fn command_suggestion_glyphs(&self) -> &bbb_client_render::HudFontGlyphMap {
        self.hud_text_measurements_ref()
    }

    fn chat_input_glyphs(&self) -> &bbb_client_render::HudFontGlyphMap {
        self.hud_text_measurements_ref().non_fishy()
    }
}

#[cfg(test)]
pub(crate) struct FallbackTextMeasurer;

#[cfg(test)]
impl CommandSuggestionTextMeasurer for FallbackTextMeasurer {
    fn command_suggestion_glyphs(&self) -> &bbb_client_render::HudFontGlyphMap {
        self.chat_input_glyphs()
    }

    fn chat_input_glyphs(&self) -> &bbb_client_render::HudFontGlyphMap {
        static GLYPHS: std::sync::OnceLock<bbb_client_render::HudFontGlyphMap> =
            std::sync::OnceLock::new();
        GLYPHS.get_or_init(|| {
            let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
            glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(6.0));
            glyphs
        })
    }
}
