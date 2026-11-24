use super::*;

const STANDING_SIGN_MAX_LINE_WIDTH: i32 = 90;
#[cfg(test)]
pub(super) const HANGING_SIGN_MAX_LINE_WIDTH: i32 = 60;
#[cfg(not(test))]
const HANGING_SIGN_MAX_LINE_WIDTH: i32 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignEditorInputState {
    pub(super) pos: ProtocolBlockPos,
    pub(super) is_front_text: bool,
    pub(super) lines: [String; 4],
    pub(super) line: usize,
    /// `TextFieldHelper` positions in UTF-16 units of the active line.
    pub(super) cursor: usize,
    pub(super) selection: usize,
    pub(super) max_line_width: i32,
    /// `AbstractSignEditScreen.cursorBlinkStartTime`.
    pub(super) blink_start: u64,
    pub(super) cursor_position: Option<(i32, i32)>,
    pub(super) ime_preedit: Option<ImePreeditState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignEditorHudState {
    pub(crate) pos: BlockPos,
    pub(crate) is_front_text: bool,
    pub(crate) lines: [String; 4],
    pub(crate) line: usize,
    pub(crate) cursor: usize,
    pub(crate) selection: usize,
    pub(crate) cursor_visible: bool,
    pub(crate) preedit: Option<ImePreeditHudState>,
}

impl ClientInputState {
    pub(crate) fn sign_editor_is_open(&self) -> bool {
        self.screen.sign_editor().is_some()
    }

    /// Vanilla's per-tick sign-presence/player-range lifetime guard
    /// (`AbstractSignEditScreen.tick`).
    pub(crate) fn sync_sign_editor(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        let invalid = self
            .screen
            .sign_editor()
            .is_some_and(|editor| !world.sign_editor_is_valid_at(editor.pos));
        if invalid {
            submit_sign_editor(self, counters, net_commands);
        }
        invalid
    }

    pub(crate) fn sign_editor_hud_state(&self) -> Option<SignEditorHudState> {
        let editor = self.screen.sign_editor()?;
        Some(sign_editor_hud_state_from_editor(
            editor,
            sign_editor_ime_preedit_hud_state(editor, bbb_client_clock::now()),
        ))
    }

    pub(crate) fn handle_sign_editor_cursor_moved(
        &mut self,
        position: Option<PhysicalPosition<f64>>,
    ) -> bool {
        let Some(editor) = self.screen.sign_editor_mut() else {
            return false;
        };
        editor.cursor_position = position.and_then(|position| {
            (position.x.is_finite() && position.y.is_finite())
                .then_some((position.x.floor() as i32, position.y.floor() as i32))
        });
        true
    }

    pub(crate) fn handle_sign_editor_mouse_input(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        if !self.handle_sign_editor_cursor_moved(cursor_position) {
            return false;
        }
        if matches!((button, state), (MouseButton::Left, ElementState::Pressed))
            && self.sign_editor_done_button_hovered(surface_size)
        {
            submit_sign_editor(self, counters, net_commands);
        }
        true
    }

    pub(crate) fn sign_editor_done_button_hovered(&self, surface_size: PhysicalSize<u32>) -> bool {
        self.screen
            .sign_editor()
            .and_then(|editor| editor.cursor_position)
            .is_some_and(|(x, y)| {
                bbb_client_render::hud_sign_editor_done_button_rect(surface_size).contains(x, y)
            })
    }
}

pub(super) fn handle_sign_editor_text(
    input: &mut ClientInputState,
    text: &str,
    text_measurer: &impl CommandSuggestionTextMeasurer,
) -> bool {
    let Some(editor) = input.screen.sign_editor_mut() else {
        return false;
    };

    let max_line_width = editor.max_line_width;
    let glyphs = text_measurer.command_suggestion_glyphs();
    edit_sign_line(editor, |field| {
        field.type_validated(text, |value| sign_line_fits(value, max_line_width, glyphs))
    });
    true
}

pub(super) fn handle_sign_editor_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
    text_measurer: &impl CommandSuggestionTextMeasurer,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
) -> bool {
    if !input.sign_editor_is_open() {
        return false;
    }
    if !pressed {
        return true;
    }

    let modifiers = input.text_field_modifiers();
    let glyphs = text_measurer.command_suggestion_glyphs();
    match code {
        KeyCode::Escape => submit_sign_editor(input, counters, net_commands),
        KeyCode::ArrowUp => {
            if let Some(editor) = input.screen.sign_editor_mut() {
                set_sign_editor_line(editor, (editor.line + 3) % editor.lines.len());
            }
        }
        KeyCode::ArrowDown | KeyCode::Enter | KeyCode::NumpadEnter => {
            if let Some(editor) = input.screen.sign_editor_mut() {
                set_sign_editor_line(editor, (editor.line + 1) % editor.lines.len());
            }
        }
        KeyCode::KeyV if modifiers.shortcut => {
            let pasted = clipboard
                .as_deref_mut()
                .and_then(DebugClipboard::get_debug_clipboard_text)
                .map(|text| sanitize_sign_clipboard_text(&text));
            if let (Some(editor), Some(pasted)) = (input.screen.sign_editor_mut(), pasted) {
                let max_line_width = editor.max_line_width;
                edit_sign_line(editor, |field| {
                    field.insert_validated(&pasted, |value| {
                        sign_line_fits(value, max_line_width, glyphs)
                    })
                });
            }
        }
        _ => {
            if let Some(editor) = input.screen.sign_editor_mut() {
                // TextFieldHelper keys match EditBox here: no length limit,
                // and only paste (above) can grow the line.
                edit_sign_line(editor, |field| field.key(code, modifiers, clipboard, None));
            }
        }
    }
    true
}

/// Runs `edit` on the active line as `TextFieldHelper` sees it.
pub(super) fn edit_sign_line<R>(
    editor: &mut SignEditorInputState,
    edit: impl FnOnce(&mut text_field::TextField) -> R,
) -> R {
    let value: Vec<u16> = editor.lines[editor.line].encode_utf16().collect();
    let mut field = text_field::TextField::from_parts(
        &value,
        editor.cursor,
        editor.selection,
        0,
        i32::MAX as usize,
    );
    let result = edit(&mut field);
    editor.lines[editor.line] = field.text();
    editor.cursor = field.cursor();
    editor.selection = field.selection();
    result
}

/// AbstractSignEditScreen admits a candidate while
/// `font.width(value) <= sign.getMaxTextLineWidth()`, and `Font.width(String)`
/// consumes the codes the line was preloaded with.
pub(super) fn sign_line_fits(
    value: &[u16],
    max_line_width: i32,
    glyphs: &bbb_client_render::HudFontGlyphMap,
) -> bool {
    let runs = bbb_client_render::hud_coded_string_runs(&String::from_utf16_lossy(value));
    bbb_client_render::hud_coded_string_width(&runs, glyphs) <= max_line_width
}

pub(super) fn sign_editor_input_state(
    input: &ClientInputState,
    world: &WorldStore,
    request: &bbb_client_world::OpenSignEditorState,
) -> SignEditorInputState {
    let lines = request
        .lines_for_filter(input.client_text_filtering_enabled())
        .clone();
    let cursor = lines[0].encode_utf16().count();
    let pos = request.pos;
    SignEditorInputState {
        pos,
        is_front_text: request.is_front_text,
        lines,
        line: 0,
        cursor,
        selection: cursor,
        max_line_width: sign_editor_max_line_width(world, pos),
        blink_start: text_field::clock_millis(),
        cursor_position: None,
        ime_preedit: None,
    }
}

pub(super) fn sign_editor_max_line_width(world: &WorldStore, pos: ProtocolBlockPos) -> i32 {
    let hanging = world
        .probe_block(pos)
        .and_then(|probe| probe.block_name)
        .and_then(|name| sign_wood_and_form_for_block_name(&name))
        .is_some_and(|(_, form)| {
            matches!(
                form,
                SignBlockForm::HangingCeiling | SignBlockForm::HangingWall
            )
        });
    if hanging {
        HANGING_SIGN_MAX_LINE_WIDTH
    } else {
        STANDING_SIGN_MAX_LINE_WIDTH
    }
}

pub(super) fn sign_editor_hud_state_from_editor(
    editor: &SignEditorInputState,
    preedit: Option<ImePreeditHudState>,
) -> SignEditorHudState {
    SignEditorHudState {
        pos: editor.pos,
        is_front_text: editor.is_front_text,
        lines: editor.lines.clone(),
        line: editor.line,
        cursor: sign_line_char_index(&editor.lines[editor.line], editor.cursor),
        selection: sign_line_char_index(&editor.lines[editor.line], editor.selection),
        cursor_visible: text_field::caret_visible(editor.blink_start, text_field::clock_millis()),
        preedit,
    }
}

pub(super) fn submit_sign_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(Screen::Sign(editor)) = input
        .screen
        .close_if(|screen| matches!(screen, Screen::Sign(_)))
    else {
        return;
    };
    queue_sign_editor_removed(counters, net_commands, &editor);
}

pub(super) fn queue_sign_editor_removed(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    editor: &SignEditorInputState,
) {
    queue_sign_update_command(
        counters,
        net_commands,
        SignUpdate {
            pos: editor.pos,
            is_front_text: editor.is_front_text,
            lines: editor.lines.clone(),
        },
    );
}

pub(super) fn set_sign_editor_line(editor: &mut SignEditorInputState, line: usize) {
    editor.line = line;
    editor.cursor = editor.lines[line].encode_utf16().count();
    editor.selection = editor.cursor;
}

/// The renderer draws sign lines by `char`; a helper position never splits a pair.
pub(super) fn sign_line_char_index(line: &str, units: usize) -> usize {
    let mut consumed = 0;
    line.chars()
        .take_while(|ch| {
            consumed += ch.len_utf16();
            consumed <= units
        })
        .count()
}

/// `TextFieldHelper.getClipboardContents`: strip `\r` and formatting codes;
/// charTyped's filter then applies as it does to typed text.
pub(super) fn sanitize_sign_clipboard_text(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            continue;
        }
        if ch == '\u{a7}' {
            let formatting = chars.peek().copied().is_some_and(|code| {
                let code = code.to_ascii_lowercase();
                VANILLA_CHAT_FORMATTINGS
                    .iter()
                    .any(|entry| entry.legacy_code == code)
            });
            if formatting {
                chars.next();
                continue;
            }
        }
        if text_field::is_allowed_chat_character(ch) {
            sanitized.push(ch);
        }
    }
    sanitized
}
