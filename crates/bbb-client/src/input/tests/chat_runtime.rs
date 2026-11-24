use super::*;
use bbb_client_render::{HudAsciiGlyph, HudChatInputView, HudFontGlyphMap};

#[test]
fn raw_suggest_replacement_truncates_at_java_units_and_resets_selection() {
    let mut session = Session::new();
    session.type_text("draft");
    let mut units = vec![65; 255];
    units.extend([0xd83d, 0xde00, 66]);
    assert!(session.input.replace_chat_entry_java_text_with_measurer(
        units.clone().into(),
        &FixedCommandSuggestionTextMeasurer
    ));
    let entry = session.input.screen.chat_entry().unwrap();
    assert!(matches!(entry.owner, CommandEntryKind::Chat(_)));
    assert_eq!(entry.edit.value(), &units[..256]);
    assert_eq!(entry.edit.cursor(), 256);
    assert_eq!(entry.edit.selection(), 256);
}

struct Session {
    input: ClientInputState,
    world: WorldStore,
    counters: NetCounters,
    commands: Option<mpsc::Sender<NetCommand>>,
    rx: mpsc::Receiver<NetCommand>,
    glyphs: HudFontGlyphMap,
}

impl Session {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel(128);
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.set_missing_glyph(HudAsciiGlyph::empty(6.0));
        for ch in " /ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzé中�".chars() {
            glyphs.insert_selected_codepoint_first_wins(
                ch as u32,
                HudAsciiGlyph::empty(40.0),
                HudAsciiGlyph::empty(6.0),
                40.0,
            );
        }
        glyphs.insert_selected_codepoint_first_wins(
            '😀' as u32,
            HudAsciiGlyph::empty(40.0),
            HudAsciiGlyph::empty(12.0),
            40.0,
        );
        let mut session = Self {
            input: ClientInputState::new(true),
            world: WorldStore::new(),
            counters: NetCounters::default(),
            commands: Some(tx),
            rx,
            glyphs,
        };
        session
            .input
            .set_chat_input_surface_size(PhysicalSize::new(22, 60));
        session.key(KeyCode::KeyT);
        session
    }

    fn event(&mut self, code: KeyCode, state: ElementState) {
        crate::input::key_events::dispatch_key_input(
            &mut self.input,
            &mut self.counters,
            &mut self.world,
            &self.commands,
            None,
            PhysicalKey::Code(code),
            state,
            PhysicalSize::new(22, 60),
            &self.glyphs,
            None,
            None,
            &mut true,
            &mut crate::simulation::effects::PlatformEffects::default(),
        );
    }

    fn key(&mut self, code: KeyCode) {
        self.event(code, ElementState::Pressed);
        self.event(code, ElementState::Released);
    }

    fn type_text(&mut self, text: &str) {
        handle_text_input_with_item_runtime(
            &mut self.input,
            &mut self.counters,
            &mut self.world,
            &self.commands,
            None,
            text,
            &self.glyphs,
        );
    }

    fn view(&self) -> HudChatInputView {
        self.input.screen.chat_entry().unwrap().chat_input_view()
    }

    fn raw(&mut self, value: &[u16]) {
        let layout = text_field::TextFieldLayout {
            glyphs: self.glyphs.non_fishy(),
            inner_width: 18,
        };
        self.input
            .screen
            .chat_entry_mut()
            .unwrap()
            .edit
            .set_value(value, Some(&layout));
    }

    fn response(&mut self, start: i32, length: i32, text: &str) {
        let id = self
            .world
            .command_suggestions()
            .last_request
            .as_ref()
            .unwrap()
            .id;
        self.world.apply_command_suggestions(CommandSuggestions {
            id,
            start,
            length,
            suggestions: vec![CommandSuggestion {
                text: text.into(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            }],
        });
    }

    fn requests(&mut self) -> Vec<CommandSuggestionRequest<bbb_protocol::JavaString>> {
        let mut requests = Vec::new();
        while let Ok(command) = self.rx.try_recv() {
            if let NetCommand::CommandSuggestionRequest(request) = command {
                requests.push(request);
            }
        }
        requests
    }
}

#[test]
fn chat_runtime_real_keys_preserve_utf16_selection_and_filtered_scroll_history() {
    let mut session = Session::new();
    session.type_text("😀AB");
    let typed = session.view();
    assert_eq!(typed.value, "😀AB".encode_utf16().collect::<Vec<_>>());
    assert_eq!(
        (typed.cursor, typed.selection, typed.display_start),
        (4, 4, 1)
    );
    session.event(KeyCode::ShiftLeft, ElementState::Pressed);
    session.key(KeyCode::ArrowLeft);
    let selected = session.view();
    assert_eq!(
        (selected.cursor, selected.selection, selected.display_start),
        (3, 4, 1)
    );
    session.key(KeyCode::Home);
    assert_eq!(
        (
            session.view().cursor,
            session.view().selection,
            session.view().display_start
        ),
        (0, 4, 0)
    );
    session.event(KeyCode::ShiftLeft, ElementState::Released);
    session.key(KeyCode::End);
    assert_eq!(
        (
            session.view().cursor,
            session.view().selection,
            session.view().display_start
        ),
        (4, 4, 1)
    );
    session.event(KeyCode::ControlLeft, ElementState::Pressed);
    session.key(KeyCode::KeyA);
    session.event(KeyCode::ControlLeft, ElementState::Released);
    let all = session.view();
    assert_eq!((all.cursor, all.selection, all.display_start), (4, 0, 0));
    session.type_text("A");
    assert_eq!(session.view().value, vec![65]);
}

#[test]
fn chat_runtime_consecutive_edits_keep_viewport_that_final_cursor_cannot_reconstruct() {
    let mut session = Session::new();
    session.type_text("😀AB");
    session.event(KeyCode::ShiftLeft, ElementState::Pressed);
    session.key(KeyCode::Home);
    session.key(KeyCode::End);
    session.event(KeyCode::ShiftLeft, ElementState::Released);
    assert_eq!(
        (
            session.view().cursor,
            session.view().selection,
            session.view().display_start
        ),
        (4, 4, 1)
    );
    session.event(KeyCode::ControlLeft, ElementState::Pressed);
    session.key(KeyCode::KeyA);
    session.event(KeyCode::ControlLeft, ElementState::Released);
    assert_eq!(session.view().cursor, 4);
    assert_eq!(session.view().display_start, 0);
    session.key(KeyCode::ArrowLeft);
    assert_eq!(
        (
            session.view().cursor,
            session.view().selection,
            session.view().display_start
        ),
        (3, 3, 0)
    );
}

#[test]
fn chat_runtime_history_and_draft_restore_raw_surrogates() {
    let mut session = Session::new();
    let history = [65, 0xd800, 66];
    session.raw(&history);
    session.key(KeyCode::Enter);
    session.key(KeyCode::KeyT);
    let working = [66, 0xdc00, 65];
    session.raw(&working);
    session.key(KeyCode::ArrowUp);
    assert_eq!(session.view().value, history);
    session.key(KeyCode::ArrowDown);
    assert_eq!(session.view().value, working);
    assert!(session.input.interrupt_chat_screen());
    session.key(KeyCode::KeyT);
    assert_eq!(session.view().value, working);
    assert_eq!(session.view().cursor, 3);
}

#[test]
fn chat_runtime_tab_sets_value_before_repositioning_cursor_with_tail_visible() {
    let mut session = Session::new();
    session.world.apply_commands(ask_server_command_tree());
    session.type_text("/xZ");
    session.key(KeyCode::Home);
    session.key(KeyCode::ArrowRight);
    session.key(KeyCode::ArrowRight);
    session.response(1, 1, "ABCDEFG");
    session.key(KeyCode::Tab);
    let view = session.view();
    assert_eq!(view.value, "/ABCDEFGZ".encode_utf16().collect::<Vec<_>>());
    assert_eq!((view.cursor, view.selection, view.display_start), (8, 8, 6));
}

#[test]
fn chat_runtime_server_utf16_ranges_replace_multibyte_prefix_and_preserve_tail() {
    for (text, prefix_steps, start, length, replacement, expected, cursor) in [
        ("/éXTAIL", 3, 2, 1, "AB", "/éABTAIL", 4),
        ("/中XTAIL", 3, 2, 1, "AB", "/中ABTAIL", 4),
        ("/😀XTAIL", 3, 3, 1, "AB", "/😀ABTAIL", 5),
        ("/é中😀XTAIL", 5, 1, 5, "AB", "/ABTAIL", 3),
    ] {
        let mut session = Session::new();
        session.world.apply_commands(ask_server_command_tree());
        session.type_text(text);
        session.key(KeyCode::Home);
        for _ in 0..prefix_steps {
            session.key(KeyCode::ArrowRight);
        }
        session.response(start, length, replacement);
        session.key(KeyCode::Tab);
        assert_eq!(
            session.view().value,
            expected.encode_utf16().collect::<Vec<_>>(),
            "{text}"
        );
        assert_eq!(
            (session.view().cursor, session.view().selection),
            (cursor, cursor)
        );
    }
}

#[test]
fn chat_runtime_local_utf8_completion_range_converts_to_utf16_once() {
    let mut session = Session::new();
    session
        .world
        .apply_commands(local_literal_command_tree(&["é中😀XYZ"]));
    session.type_text("/é中😀X");
    session.key(KeyCode::Tab);
    assert_eq!(
        session.view().value,
        "/é中😀XYZ".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!((session.view().cursor, session.view().selection), (8, 8));
    assert!(session.requests().is_empty());
}

#[test]
fn chat_runtime_only_edit_after_cursor_move_requests_prefix_and_rejects_stale_response() {
    let mut session = Session::new();
    session.world.apply_commands(ask_server_command_tree());
    session.type_text("/é😀X");
    let original = session.requests();
    assert_eq!(original.len(), 1);
    assert_eq!(original[0].command, "/é😀X");
    session.key(KeyCode::ArrowLeft);
    assert!(
        session.requests().is_empty(),
        "moving EditBox cursor does not call its responder"
    );
    session.type_text("A");
    let edited = session.requests();
    assert_eq!(edited.len(), 1);
    assert_eq!(edited[0].command, "/é😀A");
    assert_ne!(edited[0].id, original[0].id);
    session.world.apply_command_suggestions(CommandSuggestions {
        id: original[0].id,
        start: 1,
        length: 4,
        suggestions: vec![CommandSuggestion {
            text: "STALE".into(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    session.key(KeyCode::Tab);
    assert_eq!(
        session.view().value,
        "/é😀AX".encode_utf16().collect::<Vec<_>>()
    );
}

#[test]
fn chat_runtime_invalid_server_ranges_are_ignored_without_editing_raw_input() {
    for (start, length) in [(-1, 1), (1, -1), (i32::MAX, i32::MAX), (99, 1)] {
        let mut session = Session::new();
        session.world.apply_commands(ask_server_command_tree());
        session.type_text("/é😀X");
        let before = session.view().value;
        session.response(start, length, "BAD");
        session.key(KeyCode::Tab);
        assert_eq!(session.view().value, before);
    }
}

#[test]
fn chat_runtime_plain_tab_preserves_raw_surrogates_outside_replaced_word() {
    let mut session = Session::new();
    session
        .world
        .apply_custom_chat_completions(CustomChatCompletions {
            action: CustomChatCompletionsAction::Set,
            entries: vec!["alpha".into()],
        });
    session.raw(&[0xd800, 32, 32, 0xdc00]);
    session.key(KeyCode::Home);
    for _ in 0..2 {
        session.key(KeyCode::ArrowRight);
    }
    session.type_text("a");
    session.key(KeyCode::Tab);
    assert_eq!(
        session.view().value,
        [0xd800, 32, 97, 108, 112, 104, 97, 32, 0xdc00]
    );
    assert_eq!((session.view().cursor, session.view().selection), (7, 7));
    assert!(session.requests().is_empty());
}

#[test]
fn chat_runtime_history_changes_request_prefix_without_another_edit() {
    let mut session = Session::new();
    session.world.apply_commands(ask_server_command_tree());
    session.type_text("/first");
    session.key(KeyCode::Enter);
    session.requests();
    session.key(KeyCode::KeyT);
    session.type_text("/working");
    let working = session.requests();
    assert_eq!(working.len(), 1);
    session.key(KeyCode::ArrowUp);
    assert_eq!(
        session.view().value,
        "/first".encode_utf16().collect::<Vec<_>>()
    );
    let previous = session.requests();
    assert_eq!(previous.len(), 1);
    assert_eq!(previous[0].command, "/first");
    assert_ne!(previous[0].id, working[0].id);
    session.key(KeyCode::ArrowDown);
    let restored = session.requests();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].command, "/working");
    assert_ne!(restored[0].id, previous[0].id);
}

#[test]
fn chat_runtime_same_display_text_raw_edit_exits_draft_and_saves_new_units() {
    let mut session = Session::new();
    session.raw(&[0xd800]);
    assert!(session.input.interrupt_chat_screen());
    session.key(KeyCode::KeyT);
    assert!(
        matches!(&session.input.screen.chat_entry().unwrap().owner, CommandEntryKind::Chat(screen) if screen.is_draft)
    );
    session.event(KeyCode::ControlLeft, ElementState::Pressed);
    session.key(KeyCode::KeyA);
    session.event(KeyCode::ControlLeft, ElementState::Released);
    session.type_text("�");
    assert_eq!(session.view().value, [0xfffd]);
    assert!(
        matches!(&session.input.screen.chat_entry().unwrap().owner, CommandEntryKind::Chat(screen) if !screen.is_draft)
    );
    assert!(session.input.interrupt_chat_screen());
    session.key(KeyCode::KeyT);
    assert_eq!(session.view().value, [0xfffd]);
}

#[test]
fn chat_runtime_cursor_after_tab_keeps_original_suggestion_cycle() {
    let mut session = Session::new();
    session.world.apply_commands(ask_server_command_tree());
    session.type_text("/g");
    let request = session.requests().pop().unwrap();
    session.world.apply_command_suggestions(CommandSuggestions {
        id: request.id,
        start: 1,
        length: 1,
        suggestions: ["give", "gift"]
            .into_iter()
            .map(|text| CommandSuggestion {
                text: text.into(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            })
            .collect(),
    });
    session.key(KeyCode::Tab);
    assert_eq!(
        session.view().value,
        "/give".encode_utf16().collect::<Vec<_>>()
    );
    session.key(KeyCode::ArrowLeft);
    assert_eq!(session.view().cursor, 4);
    assert!(session.requests().is_empty());
    session.key(KeyCode::Tab);
    assert_eq!(
        session.view().value,
        "/gift".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!((session.view().cursor, session.view().selection), (5, 5));
    assert!(session.requests().is_empty());
}

#[test]
fn chat_runtime_submit_normalizes_whitespace_but_preserves_raw_surrogates_in_history() {
    for (value, expected) in [
        (vec![0, 0x1680, 0xa0, 0, 65, 0, 0xa0, 0x1680, 0], vec![65]),
        (vec![65, 0xa0, 0xa0, 66], vec![65, 32, 32, 66]),
        (
            vec![0x1680, 0x1680, 65, 0x1680, 0x1680, 66, 0x1680],
            vec![65, 32, 66],
        ),
        (
            vec![0xa0, 0xd800, 65, 0xdc00, 0xa0],
            vec![0xd800, 65, 0xdc00],
        ),
    ] {
        let mut session = Session::new();
        session.raw(&value);
        session.key(KeyCode::Enter);
        assert!(!session.input.chat_entry_is_active());
        assert_eq!(session.input.recent_chat.back(), Some(&expected));
        session.key(KeyCode::KeyT);
        session.key(KeyCode::ArrowUp);
        assert_eq!(session.view().value, expected);
    }
}

#[test]
fn chat_runtime_submit_observes_editbox_raw_cap_before_normalization() {
    let mut emoji = vec![65; 255];
    emoji.extend([0xd83d, 0xde00]);
    let mut session = Session::new();
    session.raw(&emoji);
    let mut capped = vec![65; 255];
    capped.push(0xd83d);
    assert_eq!(session.view().value, capped);
    session.key(KeyCode::Enter);
    assert_eq!(session.input.recent_chat.back(), Some(&capped));
    let mut spaced = vec![65; 255];
    spaced.extend([32, 66]);
    let mut session = Session::new();
    session.raw(&spaced);
    assert_eq!(session.view().value.len(), 256);
    assert_eq!(session.view().value.last(), Some(&32));
    session.key(KeyCode::Enter);
    assert_eq!(session.input.recent_chat.back(), Some(&vec![65; 255]));
}

#[test]
fn chat_normalization_matches_java_trim_normalize_space_and_utf16_truncation() {
    assert_eq!(
        normalize_chat_message(" \t hello\n\r world  "),
        "hello world"
    );
    assert_eq!(normalize_chat_message("a\u{a0}\u{a0}b"), "a  b");
    assert_eq!(
        normalize_chat_message("\u{2007}a\u{202f}"),
        "\u{2007}a\u{202f}"
    );
    let oversized = format!("{}😀z", "a".repeat(254));
    assert_eq!(
        normalize_chat_message(&oversized),
        format!("{}😀", "a".repeat(254))
    );
}

#[test]
fn chat_runtime_disallowed_typed_character_keeps_the_draft() {
    // EditBox.charTyped drops a disallowed code point before insertText.
    for units in [1, 256] {
        let mut session = Session::new();
        session.raw(&vec![65; units]);
        assert!(session.input.interrupt_chat_screen());
        session.key(KeyCode::KeyT);
        session.type_text("\u{7f}");
        assert_eq!(session.view().value, vec![65; units]);
        assert!(matches!(
            &session.input.screen.chat_entry().unwrap().owner,
            CommandEntryKind::Chat(screen) if screen.is_draft
        ));
    }
}

/// One command node for the chat syntax fixtures. `mode` is the
/// `brigadier:string` mode byte: 0 single word, 1 quotable, 2 greedy.
fn string_command_node(
    node_type: CommandNodeType,
    name: Option<&str>,
    mode: u8,
    children: Vec<i32>,
    executable: bool,
) -> CommandNode {
    CommandNode {
        node_type,
        flags: 0,
        children,
        redirect: None,
        name: name.map(str::to_string),
        parser: (node_type == CommandNodeType::Argument).then(|| CommandArgumentParser {
            type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
            name: "brigadier:string".to_string(),
            properties: vec![mode],
        }),
        suggestions: None,
        executable,
        restricted: false,
    }
}

/// Root -> "cmd" -> one string argument per name, chained in order.
fn syntax_command_tree(arguments: &[(&str, u8)]) -> Commands {
    let last = arguments.len() + 1;
    let mut nodes = vec![
        string_command_node(CommandNodeType::Root, None, 0, vec![1], false),
        string_command_node(CommandNodeType::Literal, Some("cmd"), 0, vec![2], false),
    ];
    nodes.extend(arguments.iter().enumerate().map(|(index, (name, mode))| {
        let node = index + 2;
        string_command_node(
            CommandNodeType::Argument,
            Some(name),
            *mode,
            (node < last).then(|| node as i32 + 1).into_iter().collect(),
            node == last,
        )
    }));
    Commands {
        root_index: 0,
        nodes,
    }
}

fn syntax_range(start: usize, end: usize) -> HudCommandInputRange {
    HudCommandInputRange { start, end }
}

/// Root -> "execute" -> {"run" (redirect to root), "if" -> "condition"
/// (redirect back to "execute")}, plus root -> "cmd" -> "value". This is the
/// shape `ExecuteCommand` and its `fork` conditional register: `run`
/// redirects to the dispatcher root, a conditional redirects back to the
/// "execute" literal, and the receiver command sits on the root.
fn redirect_command_tree() -> Commands {
    let mut nodes = vec![
        string_command_node(CommandNodeType::Root, None, 0, vec![1, 5], false),
        string_command_node(
            CommandNodeType::Literal,
            Some("execute"),
            0,
            vec![2, 3],
            false,
        ),
        string_command_node(CommandNodeType::Literal, Some("run"), 0, vec![], false),
        string_command_node(CommandNodeType::Literal, Some("if"), 0, vec![4], false),
        string_command_node(
            CommandNodeType::Argument,
            Some("condition"),
            0,
            vec![],
            false,
        ),
        string_command_node(CommandNodeType::Literal, Some("cmd"), 0, vec![6], false),
        string_command_node(CommandNodeType::Argument, Some("value"), 0, vec![], true),
    ];
    nodes[2].redirect = Some(0);
    nodes[4].redirect = Some(1);
    Commands {
        root_index: 0,
        nodes,
    }
}

impl Session {
    fn hud(&mut self) -> crate::input::CommandSuggestionHudState {
        let Session { input, world, .. } = self;
        input
            .command_suggestion_hud_state(
                world,
                &FixedCommandSuggestionTextMeasurer,
                PhysicalSize::new(22, 60),
            )
            .expect("chat entry owns a HUD state")
    }
}

#[test]
fn chat_input_syntax_uses_utf16_units_of_the_local_parse() {
    let mut session = Session::new();
    session
        .world
        .apply_commands(syntax_command_tree(&[("a", 1), ("b", 0)]));
    // The quoted argument is five UTF-16 units but eight UTF-8 bytes, so the
    // second range starts three bytes later than its unit index.
    session.type_text("/cmd \"é😀\" b");
    let hud = session.hud();
    let syntax = hud.syntax.expect("a command input is parsed");
    assert_eq!(
        syntax.arguments,
        vec![syntax_range(5, 10), syntax_range(11, 12)]
    );
    assert_eq!(syntax.unread, None);
    assert!(!hud.draft);
    assert_eq!(hud.input_text, "/cmd \"é😀\" b");
}

#[test]
fn chat_input_syntax_marks_the_unread_tail_and_skips_plain_messages() {
    let mut session = Session::new();
    session
        .world
        .apply_commands(syntax_command_tree(&[("a", 0), ("b", 0), ("c", 0)]));
    session.type_text("/cmd x y z extra");
    let syntax = session.hud().syntax.expect("a command input is parsed");
    assert_eq!(
        syntax.arguments,
        vec![syntax_range(5, 6), syntax_range(7, 8), syntax_range(9, 10)]
    );
    // The parser stopped before "extra", which is what `canRead` colours.
    assert_eq!(syntax.unread, Some(syntax_range(11, 16)));

    let mut session = Session::new();
    session
        .world
        .apply_commands(syntax_command_tree(&[("a", 0)]));
    session.type_text("/nope");
    let syntax = session.hud().syntax.expect("a slash input is parsed");
    assert!(syntax.arguments.is_empty());
    assert_eq!(syntax.unread, Some(syntax_range(1, 5)));
    // Reopening with just the slash parses an empty command: nothing is red.
    let mut slash = Session::new();
    slash.world.apply_commands(syntax_command_tree(&[("a", 0)]));
    slash.type_text("/");
    let syntax = slash.hud().syntax.expect("a slash input is parsed");
    assert!(syntax.arguments.is_empty());
    assert_eq!(syntax.unread, None);

    // A message without the leading slash is never parsed.
    let mut message = Session::new();
    message
        .world
        .apply_commands(syntax_command_tree(&[("a", 0)]));
    message.type_text("cmd x");
    assert!(message.hud().syntax.is_none());
    // Neither is a space before the slash, which vanilla's reader rejects.
    let mut spaced = Session::new();
    spaced
        .world
        .apply_commands(syntax_command_tree(&[("a", 0)]));
    spaced.type_text(" /cmd x");
    assert!(spaced.hud().syntax.is_none());
}

#[test]
fn chat_input_syntax_follows_the_context_argument_map_order() {
    // "cmd" -> target -> target -> message: Brigadier's context map holds one
    // "target" at its first position, with the range it parsed last.
    let mut session = Session::new();
    session.world.apply_commands(syntax_command_tree(&[
        ("target", 0),
        ("target", 0),
        ("message", 0),
    ]));
    session.type_text("/cmd a b c");
    let syntax = session.hud().syntax.expect("a command input is parsed");
    assert_eq!(
        syntax.arguments,
        vec![syntax_range(7, 8), syntax_range(9, 10)]
    );
    assert_eq!(syntax.unread, None);

    // A repeated name behind another argument cannot be read in map order, so
    // the range that would move backwards is dropped rather than double-drawn.
    let mut session = Session::new();
    session.world.apply_commands(syntax_command_tree(&[
        ("target", 0),
        ("other", 0),
        ("target", 0),
    ]));
    session.type_text("/cmd a b c");
    let syntax = session.hud().syntax.expect("a command input is parsed");
    assert_eq!(syntax.arguments, vec![syntax_range(9, 10)]);
    assert_eq!(syntax.unread, None);
}

#[test]
fn chat_input_syntax_colours_only_the_innermost_redirect_context() {
    let mut session = Session::new();
    session.world.apply_commands(redirect_command_tree());
    session.type_text("/execute if first run cmd second extra");
    // The flat parse keeps both contexts' arguments; the recorded boundary is
    // what `formatText`'s `getLastChild()` slices at.
    let parse = session
        .world
        .parse_command("/execute if first run cmd second extra");
    assert_eq!(
        parse
            .arguments
            .iter()
            .map(|argument| &argument.name)
            .collect::<Vec<_>>(),
        vec!["condition", "value"]
    );
    assert_eq!(parse.innermost_context_argument_start, 1);

    let syntax = session.hud().syntax.expect("a command input is parsed");
    // "first" belongs to the outer context and stays literal grey; only the
    // "run" redirect's own argument is coloured. The unread tail is still red.
    assert_eq!(syntax.arguments, vec![syntax_range(26, 32)]);
    assert_eq!(syntax.unread, Some(syntax_range(33, 38)));

    // Without a redirect the innermost context is the whole parse, so the
    // ranges are exactly the ones the flat list always produced.
    let mut plain = Session::new();
    plain.world.apply_commands(redirect_command_tree());
    plain.type_text("/cmd second");
    let syntax = plain.hud().syntax.expect("a command input is parsed");
    assert_eq!(syntax.arguments, vec![syntax_range(5, 11)]);
    assert_eq!(syntax.unread, None);
}

#[test]
fn chat_input_syntax_keeps_the_parse_while_a_draft_is_shown() {
    let mut session = Session::new();
    session
        .world
        .apply_commands(syntax_command_tree(&[("a", 0)]));
    session.input.set_save_chat_drafts(true);
    session.type_text("/cmd x");
    session
        .input
        .close_chat_screen(ChatScreenExitReason::Intentional);
    session.key(KeyCode::KeyT);
    let hud = session.hud();
    assert!(hud.draft, "the recalled value is a draft");
    assert_eq!(
        hud.syntax
            .expect("the screen still holds its parse")
            .arguments,
        vec![syntax_range(5, 6)]
    );
}

/// `CommandSuggestions.showSuggestions` sizes the list from
/// `font.width(suggestion.getText())` and
/// the row draw reads the same `String`. Both go through one
/// `FormattedText.of` run list, so the codes must not reach the width either.
#[test]
fn suggestion_list_width_measures_the_resolved_run_list_not_the_raw_string() {
    let suggestion = "\u{a7}cseed\u{a7}rplain";
    let mut glyphs = HudFontGlyphMap::new();
    glyphs.set_missing_glyph(HudAsciiGlyph::empty(6.0));
    let (tx, _rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    world.apply_commands(ask_server_command_tree());
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "/a");
    world.apply_command_suggestions(CommandSuggestions {
        id: 0,
        start: 1,
        length: 1,
        suggestions: vec![CommandSuggestion {
            text: suggestion.into(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    let hud = input
        .command_suggestion_hud_state(&world, &glyphs, PhysicalSize::new(320, 240))
        .expect("chat suggestions");
    assert_eq!(
        hud.entries[0].runs,
        vec![
            bbb_client_render::HudStyledTextRun {
                shadow_color: None,
                text: "seed".to_string(),
                style: Default::default(),
                color: Some(0xFF_5555),
            },
            bbb_client_render::HudStyledTextRun::plain("plain"),
        ]
    );
    let entries = input
        .screen
        .chat_entry()
        .unwrap()
        .suggestion_list
        .as_ref()
        .unwrap();
    assert_eq!(entries.resolved, vec![hud.entries[0].runs.clone()]);
    let coded = bbb_client_render::hud_coded_string_width(&hud.entries[0].runs, &glyphs);
    assert_eq!(coded, 9 * 6);
    // The raw shape would have charged both code characters their advance.
    assert_ne!(coded, glyphs.plain_text_width(suggestion));
    assert_eq!(hud.rect.width, coded as u32 + 1);
}
