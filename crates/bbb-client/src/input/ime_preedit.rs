use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ImePreeditState {
    text: String,
    caret: Option<usize>,
    focused_start: usize,
    focused_end: usize,
    initialized_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImePreeditHudState {
    pub(crate) text: String,
    pub(crate) caret: Option<usize>,
    pub(crate) focused_start: usize,
    pub(crate) focused_end: usize,
    pub(crate) caret_visible: bool,
}

impl ImePreeditHudState {
    pub(crate) fn into_hud_ime_preedit(self) -> bbb_client_render::HudImePreedit {
        bbb_client_render::HudImePreedit {
            text: self.text,
            caret: self.caret,
            focused_start: self.focused_start,
            focused_end: self.focused_end,
            caret_visible: self.caret_visible,
        }
    }
}

impl ImePreeditState {
    pub(super) fn from_event(
        text: &str,
        cursor_range: Option<(usize, usize)>,
        now: Instant,
    ) -> Option<Self> {
        if text.is_empty() {
            return None;
        }
        let original_text = text;
        let text = original_text
            .chars()
            .filter(|ch| !ch.is_control())
            .take(DIALOG_IME_PREEDIT_MAX_CHARS)
            .collect::<String>();
        if text.is_empty() {
            return None;
        }
        let text_len = text.chars().count();
        let (caret, focused_start, focused_end) =
            cursor_range.map_or((None, 0, text_len), |(begin, end)| {
                let begin = ime_preedit_char_index_for_byte(original_text, begin);
                let end = ime_preedit_char_index_for_byte(original_text, end);
                let start = begin.min(end);
                let finish = begin.max(end);
                if start == finish {
                    (Some(end), 0, text_len)
                } else {
                    (Some(end), start, finish)
                }
            });
        Some(Self {
            text,
            caret,
            focused_start,
            focused_end,
            initialized_at: now,
        })
    }

    pub(super) fn hud_state(&self, now: Instant) -> ImePreeditHudState {
        let elapsed = now
            .checked_duration_since(self.initialized_at)
            .unwrap_or_default();
        ImePreeditHudState {
            text: self.text.clone(),
            caret: self.caret,
            focused_start: self.focused_start,
            focused_end: self.focused_end,
            caret_visible: (elapsed.as_millis()
                / DIALOG_IME_PREEDIT_CURSOR_BLINK_INTERVAL.as_millis())
            .is_multiple_of(2),
        }
    }
}

fn ime_preedit_char_index_for_byte(text: &str, byte_index: usize) -> usize {
    text.char_indices()
        .take_while(|(index, _)| *index < byte_index.min(text.len()))
        .filter(|(_, ch)| !ch.is_control())
        .take(DIALOG_IME_PREEDIT_MAX_CHARS)
        .count()
}

impl ClientInputState {
    pub(crate) fn handle_sign_editor_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) -> bool {
        self.handle_sign_editor_ime_preedit_at(text, cursor_range, bbb_client_clock::now())
    }
    pub(super) fn handle_sign_editor_ime_preedit_at(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        now: Instant,
    ) -> bool {
        let Some(editor) = self.screen.sign_editor_mut() else {
            return false;
        };
        editor.ime_preedit = ImePreeditState::from_event(text, cursor_range, now);
        true
    }
    pub(crate) fn clear_sign_editor_ime_preedit(&mut self) {
        if let Some(editor) = self.screen.sign_editor_mut() {
            editor.ime_preedit = None;
        }
    }
    pub(crate) fn handle_chat_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(entry) = self.screen.chat_entry_mut() {
            entry.edit.set_ime_preedit(text, cursor_range);
        }
    }
    pub(crate) fn clear_chat_ime_preedit(&mut self) {
        if let Some(entry) = self.screen.chat_entry_mut() {
            entry.edit.clear_ime_preedit();
        }
    }
    pub(crate) fn chat_ime_preedit_hud_state(&self) -> Option<ImePreeditHudState> {
        self.screen.chat_entry()?.edit.ime_preedit_hud_state()
    }

    pub(crate) fn handle_book_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(input) = self.screen.book_mut().and_then(|book| book.input.as_mut()) {
            input.ime_preedit =
                ImePreeditState::from_event(text, cursor_range, bbb_client_clock::now());
        }
    }
    pub(crate) fn clear_book_ime_preedit(&mut self) {
        if let Some(input) = self.screen.book_mut().and_then(|book| book.input.as_mut()) {
            input.ime_preedit = None;
        }
    }
    pub(crate) fn book_ime_preedit_hud_state(&self) -> Option<ImePreeditHudState> {
        Some(
            self.screen
                .book()?
                .input
                .as_ref()?
                .ime_preedit
                .as_ref()?
                .hud_state(bbb_client_clock::now()),
        )
    }

    pub(crate) fn handle_anvil_rename_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(container) = self.screen.container_mut() {
            container.recipe_book_search.clear_ime_preedit();
            container.anvil_rename.set_ime_preedit(text, cursor_range);
        }
    }
    pub(crate) fn clear_anvil_rename_ime_preedit(&mut self) {
        if let Some(container) = self.screen.container_mut() {
            container.anvil_rename.clear_ime_preedit();
        }
    }
    pub(crate) fn anvil_rename_field(&self) -> Option<&text_field::TextField> {
        self.screen
            .container()
            .map(|container| &*container.anvil_rename)
    }

    pub(crate) fn handle_recipe_book_search_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(container) = self.screen.container_mut() {
            container.anvil_rename.clear_ime_preedit();
            container
                .recipe_book_search
                .set_ime_preedit(text, cursor_range);
        }
    }
    pub(crate) fn clear_recipe_book_search_ime_preedit(&mut self) {
        if let Some(container) = self.screen.container_mut() {
            container.recipe_book_search.clear_ime_preedit();
        }
    }

    pub(crate) fn handle_creative_search_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(creative) = self.screen.creative_mut() {
            creative.search_mut().set_ime_preedit(text, cursor_range);
        }
    }
    pub(crate) fn clear_creative_search_ime_preedit(&mut self) {
        if let Some(creative) = self.screen.creative_mut() {
            creative.search_mut().clear_ime_preedit();
        }
    }

    pub(crate) fn handle_structure_block_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(editor) = self.screen.structure_block_mut() {
            for field in &mut editor.fields {
                field.clear_ime_preedit();
            }
            if let Some(field) = editor.fields.get_mut(editor.focus.index()) {
                field.set_ime_preedit(text, cursor_range);
            }
        }
    }
    pub(crate) fn clear_structure_block_ime_preedit(&mut self) {
        if let Some(editor) = self.screen.structure_block_mut() {
            for field in &mut editor.fields {
                field.clear_ime_preedit();
            }
        }
    }

    pub(crate) fn handle_jigsaw_block_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(editor) = self.screen.jigsaw_block_mut() {
            for field in &mut editor.fields {
                field.clear_ime_preedit();
            }
            if let Some(field) = editor.fields.get_mut(editor.focus.index()) {
                field.set_ime_preedit(text, cursor_range);
            }
        }
    }
    pub(crate) fn clear_jigsaw_block_ime_preedit(&mut self) {
        if let Some(editor) = self.screen.jigsaw_block_mut() {
            for field in &mut editor.fields {
                field.clear_ime_preedit();
            }
        }
    }

    pub(crate) fn handle_test_block_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(editor) = self.screen.test_block_mut() {
            if editor.message_is_editable() {
                editor.message_mut().set_ime_preedit(text, cursor_range);
            } else {
                editor.message_mut().clear_ime_preedit();
            }
        }
    }
    pub(crate) fn clear_test_block_ime_preedit(&mut self) {
        if let Some(editor) = self.screen.test_block_mut() {
            editor.message_mut().clear_ime_preedit();
        }
    }

    pub(crate) fn handle_test_instance_block_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(editor) = self.screen.test_instance_block_mut() {
            for field in &mut editor.fields {
                field.clear_ime_preedit();
            }
            if let Some(field) = editor.fields.get_mut(editor.focus.index()) {
                field.set_ime_preedit(text, cursor_range);
            }
        }
    }
    pub(crate) fn clear_test_instance_block_ime_preedit(&mut self) {
        if let Some(editor) = self.screen.test_instance_block_mut() {
            for field in &mut editor.fields {
                field.clear_ime_preedit();
            }
        }
    }

    pub(crate) fn handle_social_interactions_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(social) = self.screen.social_mut() {
            social.search_mut().set_ime_preedit(text, cursor_range);
        }
    }
    pub(crate) fn clear_social_interactions_ime_preedit(&mut self) {
        if let Some(social) = self.screen.social_mut() {
            social.search_mut().clear_ime_preedit();
        }
    }

    pub(crate) fn handle_game_rules_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(screen) = self.screen.game_rules_mut() {
            screen.set_focused_ime_preedit(text, cursor_range);
        }
    }
    pub(crate) fn clear_game_rules_ime_preedit(&mut self) {
        if let Some(screen) = self.screen.game_rules_mut() {
            screen.clear_ime_preedit();
        }
    }

    pub(crate) fn handle_debug_options_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
    ) {
        if let Some(screen) = self.screen.debug_options_mut() {
            screen.search.set_ime_preedit(text, cursor_range);
        }
    }
    pub(crate) fn clear_debug_options_ime_preedit(&mut self) {
        if let Some(screen) = self.screen.debug_options_mut() {
            screen.search.clear_ime_preedit();
        }
    }

    /// Clears preedit composition from every screen on the stack.
    pub(crate) fn clear_all_ime_preedits(&mut self) {
        self.clear_dialog_ime_preedit();
        for screen in self.screen.iter_mut() {
            match screen {
                Screen::CommandEntry(entry) => entry.edit.clear_ime_preedit(),
                Screen::Sign(editor) => editor.ime_preedit = None,
                Screen::Book(book) => {
                    if let Some(input) = book.input.as_mut() {
                        input.ime_preedit = None;
                    }
                }
                Screen::Container(container) => {
                    container.anvil_rename.clear_ime_preedit();
                    container.recipe_book_search.clear_ime_preedit();
                }
                Screen::Creative(creative) => creative.search_mut().clear_ime_preedit(),
                Screen::StructureBlock(editor) => {
                    for field in &mut editor.fields {
                        field.clear_ime_preedit();
                    }
                }
                Screen::JigsawBlock(editor) => {
                    for field in &mut editor.fields {
                        field.clear_ime_preedit();
                    }
                }
                Screen::TestBlock(editor) => editor.message_mut().clear_ime_preedit(),
                Screen::TestInstanceBlock(editor) => {
                    for field in &mut editor.fields {
                        field.clear_ime_preedit();
                    }
                }
                Screen::Social(social) => social.search_mut().clear_ime_preedit(),
                Screen::GameRules(rules) => rules.clear_ime_preedit(),
                Screen::DebugOptions(debug) => debug.search.clear_ime_preedit(),
                _ => {}
            }
        }
    }
}
pub(super) fn sign_editor_ime_preedit_hud_state(
    editor: &SignEditorInputState,
    now: Instant,
) -> Option<ImePreeditHudState> {
    Some(editor.ime_preedit.as_ref()?.hud_state(now))
}
