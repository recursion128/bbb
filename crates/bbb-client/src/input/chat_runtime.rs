use super::text_field::{TextField, TextFieldLayout};
use super::*;

impl Default for ChatEntryState {
    fn default() -> Self {
        Self {
            edit: text_field::TextField::new(CHAT_ENTRY_MAX_LENGTH),
            suggestion_cursor: 0,
            command_completion: None,
            last_suggestion_request_text: None,
            suggestion_list: None,
            hidden_suggestion_id: None,
            suggestion_narration: None,
            mouse_position: None,
            suppress_open_key_commit: false,
            owner: CommandEntryKind::Chat(ChatScreenEntryState {
                method: ChatMethod::Message,
                is_draft: false,
                history_buffer: Vec::new(),
                history_pos: 0,
                revision: 0,
                initial: Vec::new(),
                dragging: false,
                last_click: None,
            }),
        }
    }
}

impl ChatEntryState {
    pub(super) fn invalidate_suggestion_list(&mut self) {
        self.suggestion_list = None;
        if let Some(cached) = self.command_completion.as_mut() {
            cached.suggestions_synced = false;
        }
    }

    fn completion_prefix(&self) -> bbb_protocol::JavaString {
        self.edit.value()[..self.edit.cursor()].to_vec().into()
    }

    pub(super) fn starts_with_slash(&self) -> bool {
        self.edit.value().first() == Some(&u16::from(b'/'))
    }

    pub(super) fn chat_input_view(&self) -> bbb_client_render::HudChatInputView {
        bbb_client_render::HudChatInputView {
            value: self.edit.value().to_vec(),
            cursor: self.edit.cursor(),
            selection: self.edit.selection(),
            display_start: self.edit.display_start(),
        }
    }

    fn chat_view(&self) -> Option<bbb_client_render::HudChatInputView> {
        matches!(self.owner, CommandEntryKind::Chat(_)).then(|| self.chat_input_view())
    }

    /// Only the ChatScreen box keeps a stateful `displayPos`; the command
    /// block box is drawn scrolled to its cursor.
    pub(super) fn edit_layout<'a>(
        &self,
        layout: &'a TextFieldLayout<'a>,
    ) -> Option<&'a TextFieldLayout<'a>> {
        matches!(self.owner, CommandEntryKind::Chat(_)).then_some(layout)
    }
}

impl ClientInputState {
    pub(super) fn chat_input_layout<'a>(
        &self,
        measurer: &'a impl CommandSuggestionTextMeasurer,
    ) -> TextFieldLayout<'a> {
        TextFieldLayout {
            glyphs: measurer.chat_input_glyphs(),
            inner_width: self.chat_input_inner_width,
        }
    }

    pub(crate) fn rebuild_chat_input(
        &mut self,
        save_current: bool,
        surface: PhysicalSize<u32>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        self.set_chat_input_surface_size(surface);
        let layout = self.chat_input_layout(measurer);
        let Some(entry) = self.screen.chat_entry_mut() else {
            return;
        };
        let CommandEntryKind::Chat(screen) = &mut entry.owner else {
            return;
        };
        // Vanilla ChatScreen saves the value only on resize, not Screen.init.
        if save_current {
            screen.initial = entry.edit.value().to_vec();
        }
        entry.edit = TextField::new(CHAT_ENTRY_MAX_LENGTH);
        entry.edit.set_value(&screen.initial, Some(&layout));
        screen.history_pos = self.recent_chat.len();
        screen.dragging = false;
        entry.last_suggestion_request_text = None;
        entry.invalidate_suggestion_list();
        entry.hidden_suggestion_id = None;
        entry.suggestion_narration = None;
    }

    pub(crate) fn refresh_chat_command_suggestions(
        &mut self,
        counters: &mut NetCounters,
        world: &mut WorldStore,
        commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if let Some(entry) = self.screen.chat_entry_mut() {
            entry.invalidate_suggestion_list();
            entry.hidden_suggestion_id = None;
        }
        maybe_queue_command_suggestion_request(self, counters, world, commands);
    }

    pub(crate) fn set_chat_input_surface_size(&mut self, surface: PhysicalSize<u32>) {
        self.chat_input_inner_width = i32::try_from(surface.width)
            .unwrap_or(i32::MAX)
            .saturating_sub(4);
    }
}

#[cfg(test)]
pub(super) fn open_chat_entry(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    method: ChatMethod,
    suppress_open_key_commit: bool,
) {
    open_chat_entry_with_measurer(
        input,
        world,
        counters,
        net_commands,
        method,
        suppress_open_key_commit,
        &FallbackTextMeasurer,
    );
}

pub(super) fn open_chat_entry_with_measurer(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    method: ChatMethod,
    suppress_open_key_commit: bool,
    measurer: &impl CommandSuggestionTextMeasurer,
) {
    let layout = input.chat_input_layout(measurer);
    release_active_input(input, world, counters, net_commands);
    input.next_chat_screen_revision = input.next_chat_screen_revision.saturating_add(1);
    let revision = input.next_chat_screen_revision;
    let (value, is_draft) = input
        .latest_chat_draft
        .as_ref()
        .filter(|draft| method.restores(draft))
        .map(|draft| (draft.text.clone(), true))
        .unwrap_or_else(|| (method.prefix().encode_utf16().collect(), false));
    let mut edit = TextField::new(CHAT_ENTRY_MAX_LENGTH);
    edit.set_value(&value, Some(&layout));
    let entry = ChatEntryState {
        edit,
        suggestion_cursor: value.len(),
        command_completion: None,
        last_suggestion_request_text: None,
        suggestion_list: None,
        hidden_suggestion_id: None,
        suggestion_narration: None,
        mouse_position: None,
        suppress_open_key_commit,
        owner: CommandEntryKind::Chat(ChatScreenEntryState {
            method,
            is_draft,
            history_buffer: Vec::new(),
            history_pos: input.recent_chat.len(),
            revision,
            initial: value,
            dragging: false,
            last_click: None,
        }),
    };
    input.set_screen(Screen::CommandEntry(entry), counters, net_commands);
    input.reset_chat_scroll();
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_chat_entry_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    pressed: bool,
    mut clipboard: Option<&mut dyn DebugClipboard>,
    measurer: &impl CommandSuggestionTextMeasurer,
) {
    if !pressed {
        return;
    }
    let layout = input.chat_input_layout(measurer);

    if let Some(entry) = input.screen.chat_entry_mut() {
        sync_chat_suggestion_list(entry, world);
    }
    if handle_chat_edit_key(
        input,
        counters,
        world,
        net_commands,
        code,
        &mut clipboard,
        &layout,
    ) {
        return;
    }

    match code {
        KeyCode::Enter | KeyCode::NumpadEnter => {
            submit_chat_entry(input, counters, world, net_commands)
        }
        KeyCode::Escape => {
            let hidden = input.screen.chat_entry_mut().and_then(|entry| {
                let list = entry.suggestion_list.take()?;
                entry.hidden_suggestion_id = Some(list.result_id);
                entry.suggestion_narration = None;
                Some(())
            });
            if hidden.is_none() {
                let close_owner = input.screen.chat_entry().map(|entry| &entry.owner);
                if matches!(close_owner, Some(CommandEntryKind::Chat(_))) {
                    input.close_chat_screen(ChatScreenExitReason::Intentional);
                } else {
                    input.screen.take_chat_entry();
                    input.reset_chat_scroll();
                }
            }
        }
        KeyCode::PageUp => {
            if input
                .screen
                .chat_entry()
                .is_none_or(|entry| !entry.commands_only())
            {
                input.scroll_chat_lines(input.chat_lines_per_page().wrapping_sub(1));
            }
        }
        KeyCode::PageDown => {
            if input
                .screen
                .chat_entry()
                .is_none_or(|entry| !entry.commands_only())
            {
                input.scroll_chat_lines(input.chat_lines_per_page().wrapping_neg().wrapping_add(1));
            }
        }
        KeyCode::Tab => {
            let shift_down = input.shift_down();
            let revealed_list = input
                .screen
                .chat_entry_mut()
                .is_some_and(|entry| show_hidden_chat_suggestion_list(entry, world));
            if revealed_list {
                return;
            }
            let used_list = input.screen.chat_entry_mut().is_some_and(|entry| {
                let Some(tab_cycles) = entry.suggestion_list.as_ref().map(|list| list.tab_cycles)
                else {
                    return false;
                };
                if tab_cycles {
                    cycle_chat_suggestion(entry, if shift_down { -1 } else { 1 });
                }
                use_selected_chat_suggestion(entry, &layout);
                true
            });
            if !used_list {
                apply_latest_chat_suggestion(input, world, &layout);
            }
        }
        KeyCode::ArrowUp => {
            let mut used_suggestions = false;
            if let Some(entry) = input.screen.chat_entry_mut() {
                if entry.suggestion_list.is_some() {
                    cycle_chat_suggestion(entry, -1);
                    if let Some(list) = entry.suggestion_list.as_mut() {
                        list.tab_cycles = false;
                    }
                    used_suggestions = true;
                }
            }
            if !used_suggestions {
                move_in_chat_history(input, -1, &layout);
                maybe_queue_command_suggestion_request(input, counters, world, net_commands);
            }
        }
        KeyCode::ArrowDown => {
            let mut used_suggestions = false;
            if let Some(entry) = input.screen.chat_entry_mut() {
                if entry.suggestion_list.is_some() {
                    cycle_chat_suggestion(entry, 1);
                    if let Some(list) = entry.suggestion_list.as_mut() {
                        list.tab_cycles = false;
                    }
                    used_suggestions = true;
                }
            }
            if !used_suggestions {
                move_in_chat_history(input, 1, &layout);
                maybe_queue_command_suggestion_request(input, counters, world, net_commands);
            }
        }
        _ => {}
    }
}

fn handle_chat_edit_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
    layout: &TextFieldLayout<'_>,
) -> bool {
    let modifiers = input.text_field_modifiers();
    let Some(entry) = input.screen.chat_entry_mut() else {
        return false;
    };
    let layout = entry.edit_layout(layout);
    let edited = match &mut entry.owner {
        CommandEntryKind::Chat(screen) if code == KeyCode::Backspace && screen.is_draft => {
            screen.is_draft = false;
            entry.edit.set_value(&[], layout);
            entry.invalidate_suggestion_list();
            entry.suggestion_narration = None;
            true
        }
        _ => match entry.edit.key(code, modifiers, clipboard, layout) {
            Some(edited) => edited,
            None => return false,
        },
    };
    update_chat_entry_after_text_change(input, counters, world, net_commands, edited);
    true
}

pub(super) fn move_in_chat_history(
    input: &mut ClientInputState,
    direction: i32,
    layout: &TextFieldLayout<'_>,
) {
    let recent_len = input.recent_chat.len();
    let Some((history_pos, current_text)) = input.screen.chat_entry().and_then(|entry| {
        let CommandEntryKind::Chat(screen) = &entry.owner else {
            return None;
        };
        Some((screen.history_pos, entry.edit.value().to_vec()))
    }) else {
        return;
    };
    let next = (i64::try_from(history_pos).unwrap_or(i64::MAX) + i64::from(direction))
        .clamp(0, i64::try_from(recent_len).unwrap_or(i64::MAX)) as usize;
    if next == history_pos {
        return;
    }
    let replacement = if next == recent_len {
        input
            .screen
            .chat_entry()
            .and_then(|entry| match &entry.owner {
                CommandEntryKind::Chat(screen) => Some(screen.history_buffer.clone()),
                CommandEntryKind::CommandBlock(_) | CommandEntryKind::MinecartCommandBlock(_) => {
                    None
                }
            })
            .unwrap_or_default()
    } else {
        input.recent_chat.get(next).cloned().unwrap_or_default()
    };
    let Some(entry) = input.screen.chat_entry_mut() else {
        return;
    };
    let CommandEntryKind::Chat(screen) = &mut entry.owner else {
        return;
    };
    if history_pos == recent_len {
        screen.history_buffer = current_text;
    }
    screen.history_pos = next;
    screen.is_draft = false;
    entry.edit.set_value(&replacement, Some(layout));
    entry.last_suggestion_request_text = None;
    entry.invalidate_suggestion_list();
    entry.hidden_suggestion_id = None;
    entry.suggestion_narration = None;
}

fn set_suggested_value(
    entry: &mut ChatEntryState,
    value: &[u16],
    cursor: usize,
    layout: &TextFieldLayout<'_>,
) {
    let layout = entry.edit_layout(layout);
    entry.edit.set_value(value, layout);
    entry.edit.set_cursor(cursor, layout);
    entry.edit.set_selection(cursor, layout);
    if let CommandEntryKind::Chat(screen) = &mut entry.owner {
        screen.is_draft = false;
    }
}

pub(super) fn apply_latest_chat_suggestion(
    input: &mut ClientInputState,
    world: &WorldStore,
    layout: &TextFieldLayout<'_>,
) {
    let Some(entry) = input.screen.chat_entry_mut() else {
        return;
    };
    if entry.commands_only() || entry.starts_with_slash() {
        sync_chat_suggestion_list(entry, world);
        use_selected_chat_suggestion(entry, layout);
        return;
    }
    let Some((updated, cursor)) =
        latest_custom_chat_completion_utf16(world, entry.edit.value(), entry.suggestion_cursor)
    else {
        return;
    };
    set_suggested_value(entry, &updated, cursor, layout);
}

fn build_chat_command_completion_cache(
    owner: &CommandEntryKind,
    edit: &text_field::TextField,
    suggestion_cursor: usize,
    revision: bbb_client_world::CommandCompletionRevision,
    world: &WorldStore,
) -> ChatCommandCompletionCache {
    let value = edit.value();
    let prefix = &value[..suggestion_cursor.min(value.len())];
    let completion = world.complete_command_utf16(prefix);
    let syntax = chat_input_syntax_state(owner, value, world);
    ChatCommandCompletionCache {
        contents: value.to_vec(),
        cursor: edit.cursor(),
        suggestion_cursor,
        revision,
        completion,
        syntax,
        suggestions_synced: false,
    }
}

fn refresh_chat_command_completion<'a>(
    entry: &'a mut ChatEntryState,
    world: &WorldStore,
) -> &'a mut ChatCommandCompletionCache {
    let revision = world.command_completion_revision();
    let cache = entry.command_completion.get_or_insert_with(|| {
        build_chat_command_completion_cache(
            &entry.owner,
            &entry.edit,
            entry.suggestion_cursor,
            revision,
            world,
        )
    });
    if cache.contents.as_slice() != entry.edit.value()
        || cache.cursor != entry.edit.cursor()
        || cache.suggestion_cursor != entry.suggestion_cursor
        || cache.revision != revision
    {
        *cache = build_chat_command_completion_cache(
            &entry.owner,
            &entry.edit,
            entry.suggestion_cursor,
            revision,
            world,
        );
    }
    cache
}

pub(super) fn sync_chat_suggestion_list(entry: &mut ChatEntryState, world: &WorldStore) {
    if refresh_chat_command_completion(entry, world).suggestions_synced {
        return;
    }
    if !entry.commands_only() && !entry.starts_with_slash() {
        return;
    }
    if entry
        .suggestion_list
        .as_ref()
        .is_some_and(|list| list.tab_cycles)
    {
        return;
    }
    let Some(cached) = entry.command_completion.as_mut() else {
        return;
    };
    cached.suggestions_synced = true;
    let completion = cached.completion.clone();
    let value = entry.edit.value();
    let raw_prefix = &value[..entry.suggestion_cursor.min(value.len())];
    let prefix = String::from_utf16_lossy(raw_prefix);
    // The local completion answers in the edit buffer's own units, which is what
    // a server suggestion response carries too (`SuggestionContext.startPos`).
    let result = if completion.request_server {
        let suggestions = world.command_suggestions();
        let request = match suggestions.last_request.as_ref() {
            Some(request) if request.command.units() == raw_prefix => request,
            _ => {
                entry.suggestion_list = None;
                return;
            }
        };
        match suggestions
            .last_id
            .and_then(|id| suggestions.by_id.get(&id))
        {
            Some(result) if result.id == request.id => result.clone(),
            _ => {
                entry.suggestion_list = None;
                return;
            }
        }
    } else {
        CommandSuggestionsResultState {
            // Server request ids are always non-negative and wrap to zero.
            id: -1,
            start: i32::try_from(completion.start).unwrap_or(i32::MAX),
            length: i32::try_from(completion.length).unwrap_or(i32::MAX),
            suggestions: completion
                .suggestions
                .into_iter()
                .map(|suggestion| CommandSuggestionState {
                    text: suggestion.text,
                    tooltip: suggestion.tooltip,
                    tooltip_translation_key: suggestion.tooltip_translation_key,
                    tooltip_styled: None,
                    tooltip_component: None,
                })
                .collect(),
        }
    };

    if entry.hidden_suggestion_id == Some(result.id) || result.suggestions.is_empty() {
        entry.suggestion_list = None;
        return;
    }
    if entry.suggestion_list.as_ref().is_some_and(|list| {
        list.original_contents.as_slice() == entry.edit.value()
            && list.result_id == result.id
            && list.result == result
    }) {
        return;
    }

    let partial = &prefix;
    let last_word_start = partial
        .char_indices()
        .filter_map(|(index, ch)| ch.is_whitespace().then_some(index + ch.len_utf8()))
        .next_back()
        .unwrap_or(0);
    let last_word = partial[last_word_start..].to_lowercase();
    // `CommandSuggestions.sortSuggestions` (:193-210) lowercases the last word
    // and compares it with `suggestion.getText()` in Java `char` units, so the
    // candidate side stays raw.
    let last_word_units: Vec<u16> = last_word.encode_utf16().collect();
    let minecraft_prefix: Vec<u16> = "minecraft:".encode_utf16().collect();
    let (direct, partial): (Vec<_>, Vec<_>) = (0..result.suggestions.len()).partition(|index| {
        let text = result.suggestions[*index].text.units();
        text.starts_with(&last_word_units)
            || text
                .strip_prefix(minecraft_prefix.as_slice())
                .is_some_and(|path| path.starts_with(&last_word_units))
    });
    let sorted_indices = direct.into_iter().chain(partial).collect();
    let resolved = result
        .suggestions
        .iter()
        // `FormattedText.of(suggestion.getText())` resolves the candidate's own
        // units, so the runs never pass through a projection.
        .map(|suggestion| {
            bbb_client_render::hud_resolve_formatted_utf16_text(suggestion.text.units())
        })
        .collect();
    entry.suggestion_list = Some(ChatSuggestionListState {
        result_id: result.id,
        result,
        original_contents: entry.edit.value().to_vec(),
        sorted_indices,
        resolved,
        current: 0,
        offset: 0,
        tab_cycles: false,
        rect: None,
    });
}

pub(super) fn show_hidden_chat_suggestion_list(
    entry: &mut ChatEntryState,
    world: &WorldStore,
) -> bool {
    let Some(hidden_id) = entry.hidden_suggestion_id.take() else {
        return false;
    };
    if let Some(cached) = entry.command_completion.as_mut() {
        cached.suggestions_synced = false;
    }
    sync_chat_suggestion_list(entry, world);
    let shown = entry
        .suggestion_list
        .as_ref()
        .is_some_and(|list| list.result_id == hidden_id);
    if shown {
        update_chat_suggestion_narration(entry);
    } else {
        entry.hidden_suggestion_id = Some(hidden_id);
    }
    shown
}

pub(super) fn cycle_chat_suggestion(entry: &mut ChatEntryState, direction: i32) {
    let Some(list) = entry.suggestion_list.as_ref() else {
        return;
    };
    let len = list.sorted_indices.len();
    if len == 0 {
        return;
    }
    let current = if direction < 0 {
        list.current.checked_sub(1).unwrap_or(len - 1)
    } else {
        (list.current + 1) % len
    };
    select_chat_suggestion(entry, current, true);
}

pub(super) fn select_chat_suggestion(entry: &mut ChatEntryState, index: usize, narrate: bool) {
    let line_limit = entry.suggestion_line_limit();
    let Some(list) = entry.suggestion_list.as_mut() else {
        return;
    };
    let len = list.sorted_indices.len();
    if len == 0 {
        return;
    }
    let current = index % len;
    let changed = list.current != current;
    list.current = current;
    let last_visible = list.offset.saturating_add(line_limit.saturating_sub(1));
    if current < list.offset {
        list.offset = current.min(len.saturating_sub(line_limit));
    } else if current > last_visible {
        list.offset = current
            .saturating_add(1)
            .saturating_sub(line_limit)
            .min(len.saturating_sub(line_limit));
    }
    if !narrate || !changed {
        return;
    }
    update_chat_suggestion_narration(entry);
}

pub(super) fn update_chat_suggestion_narration(entry: &mut ChatEntryState) {
    let Some(list) = entry.suggestion_list.as_ref() else {
        return;
    };
    let len = list.sorted_indices.len();
    let current = list.current;
    let Some(suggestion) = list
        .sorted_indices
        .get(current)
        .and_then(|index| list.result.suggestions.get(*index))
    else {
        return;
    };
    entry.suggestion_narration = Some(CommandSuggestionNarrationState {
        position: current + 1,
        total: len,
        text: suggestion.text.clone(),
        tooltip: suggestion.tooltip.clone(),
        tooltip_translation_key: suggestion.tooltip_translation_key.clone(),
    });
}

pub(super) fn use_selected_chat_suggestion(
    entry: &mut ChatEntryState,
    layout: &TextFieldLayout<'_>,
) {
    let Some(list) = entry.suggestion_list.as_ref() else {
        return;
    };
    let Some(suggestion) = list
        .sorted_indices
        .get(list.current)
        .and_then(|index| list.result.suggestions.get(*index))
    else {
        return;
    };
    let start = match usize::try_from(list.result.start) {
        Ok(start) => start,
        Err(_) => return,
    };
    let Some(updated) = apply_command_suggestion_utf16(
        &list.original_contents,
        list.result.start,
        list.result.length,
        &suggestion.text,
    ) else {
        return;
    };
    let cursor = start.saturating_add(suggestion.text.units().len());
    set_suggested_value(entry, &updated, cursor, layout);
    entry.last_suggestion_request_text = Some(entry.completion_prefix());
    if let Some(list) = entry.suggestion_list.as_mut() {
        list.tab_cycles = true;
    }
}

pub(super) fn compute_chat_suggestion_list_rect(
    entry: &ChatEntryState,
    measurer: &impl CommandSuggestionTextMeasurer,
    surface_size: PhysicalSize<u32>,
) -> Option<CommandSuggestionRect> {
    let command_block_layout = entry.commands_only();
    let list = entry.suggestion_list.as_ref()?;
    let max_width = list
        .sorted_indices
        .iter()
        .filter_map(|index| list.resolved.get(*index))
        .map(|runs| {
            bbb_client_render::hud_coded_string_width(runs, measurer.command_suggestion_glyphs())
        })
        .max()
        .unwrap_or_default();
    let start = usize::try_from(list.result.start).unwrap_or_default();
    let prefix_width = list
        .original_contents
        .get(..start)
        .map(|prefix| {
            // `EditBox.getScreenX(charIndex)` measures
            // `font.width(value.substring(0, charIndex))`,
            // so this anchor consumes codes even though the edit box draws the
            // same characters raw, and it reads the edit box's own font.
            let prefix = String::from_utf16_lossy(prefix);
            let runs = bbb_client_render::hud_coded_string_runs(&prefix);
            let glyphs = if command_block_layout {
                measurer.command_suggestion_glyphs()
            } else {
                measurer.chat_input_glyphs()
            };
            bbb_client_render::hud_coded_string_width(&runs, glyphs)
        })
        .unwrap_or_default();
    let x = if command_block_layout {
        let input_x = i32::try_from(surface_size.width / 2)
            .unwrap_or(i32::MAX)
            .saturating_sub(150);
        let max_x = input_x.saturating_add(296).saturating_sub(max_width);
        input_x
            .saturating_add(4)
            .saturating_add(prefix_width)
            .clamp(0, max_x.max(0))
    } else {
        let max_x = i32::try_from(surface_size.width)
            .unwrap_or(i32::MAX)
            .saturating_sub(max_width);
        // ChatScreen's unbordered edit box starts at x=4. Vanilla first clamps
        // getScreenX(start), then SuggestionsList shifts that anchor one pixel
        // left (including at the right edge).
        (4i32.saturating_add(prefix_width))
            .clamp(0, max_x.max(0))
            .saturating_sub(1)
    };
    let line_limit = if command_block_layout {
        COMMAND_BLOCK_SUGGESTION_LINE_LIMIT
    } else {
        COMMAND_SUGGESTION_LINE_LIMIT
    };
    let visible = list.sorted_indices.len().min(line_limit);
    let height = u32::try_from(visible)
        .unwrap_or_default()
        .saturating_mul(COMMAND_SUGGESTION_LINE_HEIGHT as u32);
    let y = if command_block_layout {
        71
    } else {
        i32::try_from(surface_size.height)
            .unwrap_or(i32::MAX)
            .saturating_sub(15)
            .saturating_sub_unsigned(height)
    };
    Some(CommandSuggestionRect {
        x,
        y,
        width: max_width.saturating_add(1).max(0) as u32,
        height,
    })
}

pub(super) fn update_chat_suggestion_list_rect(
    entry: &mut ChatEntryState,
    measurer: &impl CommandSuggestionTextMeasurer,
    surface_size: PhysicalSize<u32>,
) {
    let rect = compute_chat_suggestion_list_rect(entry, measurer, surface_size);
    if let Some(list) = entry.suggestion_list.as_mut() {
        list.rect = rect;
    }
}

/// The local parse of the chat input, reduced to what the formatter draws with.
///
/// Vanilla parses this screen's value only when it starts with a slash
/// a plain message leaves `currentParse`
/// null and keeps the edit box's own colour. The parse is of the whole value
/// and survives the draft state, because `ChatScreen.isDraft` only prepends a
/// formatter (267).
fn chat_input_syntax_state(
    owner: &CommandEntryKind,
    value: &[u16],
    world: &WorldStore,
) -> Option<HudCommandInputSyntax> {
    if !matches!(owner, CommandEntryKind::Chat(_)) {
        return None;
    }
    if value.first() != Some(&u16::from(b'/')) {
        return None;
    }
    // The parse reads the raw buffer, so every position it reports is already
    // an edit-box unit index and no projection stands between the two.
    let parse = world.parse_command_utf16(value);
    let mut arguments: Vec<&str> = Vec::new();
    let mut ranges = Vec::new();
    // `formatText` reads `getLastChild().getArguments()`, not the whole parse:
    // `/execute ... run <cmd>` recolours the receiver's arguments only, and the
    // outer context stays literal grey (CommandSuggestions)
    let owned = parse
        .arguments
        .get(parse.innermost_context_argument_start..)
        .unwrap_or_default();
    for argument in owned {
        if argument.end <= argument.start {
            continue;
        }
        let range = HudCommandInputRange {
            start: argument.start,
            end: argument.end,
        };
        // Brigadier stores one context's arguments in a LinkedHashMap keyed by
        // argument name (44-51): a repeated name
        // keeps its first iteration position and its last parsed range.
        match arguments.iter().position(|name| *name == argument.name) {
            Some(index) => ranges[index] = range,
            None => {
                arguments.push(&argument.name);
                ranges.push(range);
            }
        }
    }
    // A name whose last range follows a later-inserted name (`a b a`) reads
    // backwards in that map order, which throws inside Vanilla's `substring`;
    // dropping the range leaves those units literal instead of double-drawing.
    let mut previous_end = 0usize;
    ranges.retain(|range| {
        let keep = range.start >= previous_end;
        if keep {
            previous_end = range.end;
        }
        keep
    });
    let unread = (parse.cursor < value.len()).then_some(HudCommandInputRange {
        start: parse.cursor,
        end: value.len(),
    });
    Some(HudCommandInputSyntax {
        arguments: ranges,
        unread,
    })
}

pub(super) fn chat_suggestion_hud_state(
    entry: &ChatEntryState,
    syntax: Option<HudCommandInputSyntax>,
) -> CommandSuggestionHudState {
    let rect_override = entry.suggestion_list.as_ref().and_then(|list| list.rect);
    chat_suggestion_hud_state_with_rect(entry, syntax, rect_override)
}

pub(super) fn chat_suggestion_hud_state_with_rect(
    entry: &ChatEntryState,
    syntax: Option<HudCommandInputSyntax>,
    rect_override: Option<CommandSuggestionRect>,
) -> CommandSuggestionHudState {
    let empty_rect = CommandSuggestionRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    let Some(list) = entry.suggestion_list.as_ref() else {
        return CommandSuggestionHudState {
            input_text: entry.edit.text(),
            input_view: entry.chat_view(),
            draft: matches!(&entry.owner, CommandEntryKind::Chat(screen) if screen.is_draft),
            syntax,
            cursor: entry.edit.cursor(),
            selection: entry.edit.selection(),
            cursor_visible: entry.edit.caret_visible(text_field::clock_millis()),
            suffix: None,
            rect: empty_rect,
            entries: Vec::new(),
            has_previous: false,
            has_next: false,
            tooltip: None,
            narration: entry.suggestion_narration.clone(),
            command_block_layout: entry.commands_only(),
        };
    };
    let result = &list.result;
    let visible_end = list
        .offset
        .saturating_add(entry.suggestion_line_limit())
        .min(list.sorted_indices.len());
    let entries = list.sorted_indices[list.offset..visible_end]
        .iter()
        .enumerate()
        .filter_map(|(visible, index)| {
            list.resolved
                .get(*index)
                .map(|runs| HudCommandSuggestionEntry {
                    runs: runs.clone(),
                    selected: list.offset + visible == list.current,
                })
        })
        .collect();
    let selected = list
        .sorted_indices
        .get(list.current)
        .and_then(|index| result.suggestions.get(*index));
    let suffix = selected
        .and_then(|suggestion| {
            apply_command_suggestion_utf16(
                &list.original_contents,
                result.start,
                result.length,
                &suggestion.text,
            )
        })
        .and_then(|suggested| {
            suggested
                .strip_prefix(entry.edit.value())
                .map(String::from_utf16_lossy)
        });
    let hovered = entry
        .mouse_position
        .zip(rect_override)
        .is_some_and(|((x, y), rect)| rect.hovered_line(x, y).is_some());
    let tooltip = if hovered {
        selected
            .and_then(|suggestion| {
                suggestion.tooltip.clone().map(|text| {
                    let runs = suggestion.tooltip_styled.clone().unwrap_or_else(|| {
                        vec![StyledTextRun {
                            text: text.clone(),
                            style: ComponentStyle::default(),
                        }]
                    });
                    (text, suggestion.tooltip_translation_key.clone(), runs)
                })
            })
            .and_then(|(text, translation_key, runs)| {
                entry
                    .mouse_position
                    .map(|(x, y)| CommandSuggestionHudTooltip {
                        text,
                        translation_key,
                        runs,
                        x,
                        y,
                    })
            })
    } else {
        None
    };

    CommandSuggestionHudState {
        input_text: entry.edit.text(),
        input_view: entry.chat_view(),
        draft: matches!(&entry.owner, CommandEntryKind::Chat(screen) if screen.is_draft),
        syntax,
        cursor: entry.edit.cursor(),
        selection: entry.edit.selection(),
        cursor_visible: entry.edit.caret_visible(text_field::clock_millis()),
        suffix,
        rect: rect_override.unwrap_or(empty_rect),
        entries,
        has_previous: list.offset > 0,
        has_next: visible_end < list.sorted_indices.len(),
        tooltip,
        narration: entry.suggestion_narration.clone(),
        command_block_layout: entry.commands_only(),
    }
}

#[cfg(test)]
pub(super) fn latest_custom_chat_completion_text(
    world: &WorldStore,
    text: &str,
    cursor: usize,
) -> Option<(String, usize)> {
    let cursor = text.chars().take(cursor).map(char::len_utf16).sum();
    let (value, cursor) = latest_custom_chat_completion_utf16(
        world,
        &text.encode_utf16().collect::<Vec<_>>(),
        cursor,
    )?;
    Some((
        String::from_utf16_lossy(&value),
        String::from_utf16_lossy(&value[..cursor]).chars().count(),
    ))
}

pub(super) fn latest_custom_chat_completion_utf16(
    world: &WorldStore,
    value: &[u16],
    cursor: usize,
) -> Option<(Vec<u16>, usize)> {
    let cursor = cursor.min(value.len());
    let partial = String::from_utf16_lossy(&value[..cursor]);
    if partial.trim().is_empty() {
        return None;
    }
    let last_word_end = partial
        .char_indices()
        .filter_map(|(index, ch)| ch.is_whitespace().then_some(index + ch.len_utf8()))
        .next_back()
        .unwrap_or(0);
    // `CommandSuggestions.updateCommandInfo` (:253-256) builds the
    // `SuggestionsBuilder` over the raw value and `SharedSuggestionProvider.suggest`
    // lowers `getRemaining()` from there, so the pattern is the raw tail. The
    // projection keeps one unit per buffer unit, which is what turns the byte
    // offset above back into a cursor index.
    let last_word_start = partial[..last_word_end].encode_utf16().count();
    let lower_prefix =
        bbb_protocol::JavaString::from(value[last_word_start..cursor].to_vec()).to_lowercase_root();

    let mut candidates: BTreeSet<&str> = world
        .player_info()
        .entries
        .values()
        .map(|entry| entry.profile.name.as_str())
        .collect();
    candidates.extend(world.custom_chat_completions().iter().map(String::as_str));
    let suggestion = candidates
        .into_iter()
        .filter(|candidate| {
            matches_suggestion_sub_str(
                lower_prefix.units(),
                bbb_protocol::JavaString::from(*candidate)
                    .to_lowercase_root()
                    .units(),
            )
        })
        .min_by(|left, right| {
            (!is_direct_chat_suggestion(left, lower_prefix.units()))
                .cmp(&(!is_direct_chat_suggestion(right, lower_prefix.units())))
                .then_with(|| left.to_lowercase().cmp(&right.to_lowercase()))
                .then_with(|| left.cmp(right))
        })?;

    let updated = apply_command_suggestion_utf16(
        value,
        i32::try_from(last_word_start).ok()?,
        i32::try_from(cursor - last_word_start).ok()?,
        &bbb_protocol::JavaString::from(suggestion),
    )?;
    Some((updated, last_word_start + suggestion.encode_utf16().count()))
}

pub(super) fn is_direct_chat_suggestion(candidate: &str, lower_prefix: &[u16]) -> bool {
    let candidate = bbb_protocol::JavaString::from(candidate);
    let candidate = candidate.units();
    let namespace = "minecraft:".encode_utf16().collect::<Vec<_>>();
    candidate.starts_with(lower_prefix)
        || candidate
            .strip_prefix(namespace.as_slice())
            .is_some_and(|path| path.starts_with(lower_prefix))
}

fn apply_command_suggestion_utf16(
    current: &[u16],
    start: i32,
    length: i32,
    suggestion: &bbb_protocol::JavaString,
) -> Option<Vec<u16>> {
    let start = usize::try_from(start).ok()?;
    let length = usize::try_from(length).ok()?;
    let end = start.checked_add(length)?;
    let prefix = current.get(..start)?;
    let suffix = current.get(end..)?;
    let mut updated = prefix.to_vec();
    updated.extend_from_slice(suggestion.units());
    updated.extend_from_slice(suffix);
    Some(updated)
}

pub(super) fn update_chat_entry_after_text_change(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    edited: bool,
) {
    let Some(entry) = input.screen.chat_entry_mut() else {
        return;
    };
    if !edited {
        return;
    }
    if let CommandEntryKind::Chat(screen) = &mut entry.owner {
        screen.is_draft = false;
    }
    entry.invalidate_suggestion_list();
    entry.hidden_suggestion_id = None;
    entry.suggestion_narration = None;
    entry.last_suggestion_request_text = None;
    maybe_queue_command_suggestion_request(input, counters, world, net_commands);
}

pub(super) fn insert_chat_entry_text(
    entry: &mut ChatEntryState,
    text: &str,
    layout: &TextFieldLayout<'_>,
) -> bool {
    let layout = entry.edit_layout(layout);
    entry.edit.type_text(text, layout)
}

pub(super) fn submit_chat_entry(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = input.screen.chat_entry() else {
        return;
    };
    if matches!(entry.owner, CommandEntryKind::Chat(_)) {
        let normalized_units = normalize_chat_units(entry.edit.value());
        let normalized = String::from_utf16_lossy(&normalized_units);
        let allowed = if normalized.strip_prefix('/').is_some() {
            chat_commands_allowed(world)
                && (chat_messages_allowed(world)
                    || !world.command_requires_signed_arguments_utf16(&normalized_units[1..]))
        } else {
            normalized.is_empty() || chat_messages_allowed(world)
        };
        if !allowed {
            return;
        }
        if !normalized.is_empty() {
            add_recent_chat_units(input, normalized_units.clone());
            if normalized_units.first() == Some(&u16::from(b'/')) {
                queue_chat_command(
                    counters,
                    world,
                    net_commands,
                    bbb_protocol::JavaString::from(normalized_units[1..].to_vec()),
                );
            } else {
                queue_chat_message_command(
                    counters,
                    world,
                    net_commands,
                    bbb_protocol::JavaString::from(normalized_units),
                );
            }
        }
        input.close_chat_screen(ChatScreenExitReason::Done);
        return;
    }
    let Some(entry) = input.screen.take_chat_entry() else {
        return;
    };
    if let CommandEntryKind::CommandBlock(editor) = &entry.owner {
        if !editor.controls_enabled {
            input.screen.open_child(Screen::CommandEntry(entry));
            return;
        }
        let pos = editor.pos;
        let mode = editor.mode;
        let track_output = editor.track_output;
        let conditional = editor.conditional;
        let automatic = editor.automatic;
        queue_set_command_block_command(
            counters,
            net_commands,
            SetCommandBlock {
                pos,
                command: entry.edit.value().to_vec().into(),
                mode,
                track_output,
                conditional,
                automatic,
            },
        );
        if !track_output {
            world.clear_command_block_last_output(editor.pos);
        }
        return;
    }
    if let CommandEntryKind::MinecartCommandBlock(editor) = &entry.owner {
        let track_output = editor.track_output;
        queue_set_command_minecart_command(
            counters,
            net_commands,
            SetCommandMinecart {
                entity_id: editor.entity_id,
                command: entry.edit.value().to_vec().into(),
                track_output,
            },
        );
        if !track_output {
            world.clear_minecart_command_block_last_output(editor.entity_id);
        }
        return;
    }
    unreachable!("all non-chat command entry owners return above");
}

#[cfg(test)]
pub(super) fn add_recent_chat(input: &mut ClientInputState, message: String) {
    add_recent_chat_units(input, message.encode_utf16().collect());
}

fn add_recent_chat_units(input: &mut ClientInputState, message: Vec<u16>) {
    if input.recent_chat.back() == Some(&message) {
        return;
    }
    if input.recent_chat.len() >= RECENT_CHAT_LIMIT {
        input.recent_chat.pop_front();
    }
    input.recent_chat.push_back(message);
}

pub(super) fn chat_messages_allowed(world: &WorldStore) -> bool {
    world.chat_presentation_config().visibility == bbb_client_world::ChatVisibilityState::Full
}

pub(super) fn chat_commands_allowed(world: &WorldStore) -> bool {
    world.chat_presentation_config().visibility != bbb_client_world::ChatVisibilityState::Hidden
}

#[cfg(test)]
pub(super) fn normalize_chat_message(message: &str) -> String {
    String::from_utf16_lossy(&normalize_chat_units(
        &message.encode_utf16().collect::<Vec<_>>(),
    ))
}

fn normalize_chat_units(message: &[u16]) -> Vec<u16> {
    let start = message
        .iter()
        .position(|unit| *unit > 32)
        .unwrap_or(message.len());
    let end = message
        .iter()
        .rposition(|unit| *unit > 32)
        .map_or(start, |end| end + 1);
    let trimmed = &message[start..end];
    let mut normalized = Vec::with_capacity(trimmed.len());
    let mut in_whitespace = false;
    for &unit in trimmed {
        if char::from_u32(u32::from(unit)).is_some_and(is_java_character_whitespace) {
            if !in_whitespace {
                normalized.push(32);
                in_whitespace = true;
            }
        } else {
            normalized.push(if unit == 0xa0 { 32 } else { unit });
            in_whitespace = false;
        }
    }
    let start = normalized
        .iter()
        .position(|unit| *unit > 32)
        .unwrap_or(normalized.len());
    let end = normalized
        .iter()
        .rposition(|unit| *unit > 32)
        .map_or(start, |end| end + 1);
    normalized.truncate(end);
    normalized.drain(..start);
    normalized.truncate(CHAT_ENTRY_MAX_LENGTH);
    normalized
}

pub(super) fn is_java_character_whitespace(ch: char) -> bool {
    matches!(
        ch,
        '\u{0009}'..='\u{000d}'
            | '\u{001c}'..='\u{0020}'
            | '\u{1680}'
            | '\u{2000}'..='\u{2006}'
            | '\u{2008}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{205f}'
            | '\u{3000}'
    )
}

pub(super) fn is_java_blank(value: &str) -> bool {
    value.is_empty() || value.chars().all(is_java_character_whitespace)
}

pub(super) fn maybe_queue_command_suggestion_request(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    let Some(entry) = input.screen.chat_entry_mut() else {
        return;
    };
    entry.suggestion_cursor = entry.edit.cursor();
    let prefix = entry.completion_prefix();
    if !refresh_chat_command_completion(entry, world)
        .completion
        .request_server
    {
        entry.last_suggestion_request_text = None;
        return;
    }
    if entry.last_suggestion_request_text.as_ref() == Some(&prefix) {
        return;
    }
    let text = prefix;
    entry.last_suggestion_request_text = Some(text.clone());
    let request = world.next_command_suggestion_request(text);
    queue_command_suggestion_request(counters, net_commands, request.id, request.command);
}
