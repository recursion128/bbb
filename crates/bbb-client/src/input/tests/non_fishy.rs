use super::*;

#[test]
fn non_fishy_command_suggestion_anchor_uses_chat_input_width_but_popup_stays_ordinary() {
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    for ch in ['/', 'a'] {
        let any = bbb_client_render::HudAsciiGlyph {
            advance: 40.0,
            ..Default::default()
        };
        let filtered = bbb_client_render::HudAsciiGlyph {
            advance: 4.0,
            ..any
        };
        glyphs.insert_selected_codepoint_first_wins(ch as u32, any, filtered, 40.0);
    }
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
            text: "aa".into(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        }],
    });
    let hud = input
        .command_suggestion_hud_state(&world, &glyphs, PhysicalSize::new(320, 240))
        .expect("chat suggestions");
    assert_eq!(hud.rect.x, 7);
    assert_eq!(hud.rect.width, 81);
    let entry = input.screen.chat_entry_mut().unwrap();
    entry.owner = CommandEntryKind::CommandBlock(CommandBlockEditorInputState {
        pos: BlockPos { x: 0, y: 0, z: 0 },
        mode: CommandBlockMode::Sequence,
        conditional: false,
        automatic: false,
        track_output: false,
        previous_output: String::new(),
        controls_enabled: true,
    });
    update_chat_suggestion_list_rect(entry, &glyphs, PhysicalSize::new(320, 240));
    let rect = entry.suggestion_list.as_ref().unwrap().rect.unwrap();
    assert_eq!(rect.x, 54);
    assert_eq!(rect.width, 81);
}
