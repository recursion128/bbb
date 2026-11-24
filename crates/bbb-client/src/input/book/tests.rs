use bbb_client_net::NetCommand;
use bbb_client_world::LocalPlayerPoseState;
use bbb_protocol::{
    item_types::{VANILLA_ITEM_WRITABLE_BOOK_ID, VANILLA_ITEM_WRITTEN_BOOK_ID},
    menu_types::VANILLA_MENU_TYPE_LECTERN_ID,
    packets::{
        ContainerSetContent, ContainerSetData, InteractionHand, ItemStackSummary, OpenScreen,
        SetPlayerInventory as ProtocolSetPlayerInventory,
    },
};

use super::*;
use crate::input::FallbackTextMeasurer;

fn book_key(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    code: KeyCode,
    measurer: &impl CommandSuggestionTextMeasurer,
) {
    assert!(input.handle_book_screen_key(
        &mut NetCounters::default(),
        world,
        &None,
        code,
        true,
        measurer,
        &mut None,
    ));
}

fn select_all(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    measurer: &impl CommandSuggestionTextMeasurer,
) {
    input.control_left_down = true;
    book_key(input, world, KeyCode::KeyA, measurer);
    input.control_left_down = false;
}

#[test]
fn book_screen_utf16_editing_preserves_split_surrogates_and_uses_unit_cursors() {
    let mut world = writable_book_world(vec!["😀A".to_string()], vec![None]);
    let mut input = ClientInputState::new(true);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    sync(&mut input);
    assert_eq!(input.book_input_state().unwrap().cursor, 3);
    book_key(
        &mut input,
        &mut world,
        KeyCode::ArrowLeft,
        &FallbackTextMeasurer,
    );
    book_key(
        &mut input,
        &mut world,
        KeyCode::ArrowLeft,
        &FallbackTextMeasurer,
    );
    assert_eq!(input.book_input_state().unwrap().cursor, 1);
    book_key(
        &mut input,
        &mut world,
        KeyCode::Backspace,
        &FallbackTextMeasurer,
    );
    assert_eq!(book_units(book(&input)).unwrap(), &[0xde00, 65]);
    assert!(input.handle_book_screen_text("§B", &FallbackTextMeasurer));
    assert_eq!(book_units(book(&input)).unwrap(), &[66, 0xde00, 65]);
    select_all(&mut input, &mut world, &FallbackTextMeasurer);
    let state = input.book_input_state().unwrap();
    assert_eq!((state.cursor, state.selection), (3, 0));
}

#[test]
fn book_screen_multiline_truncation_keeps_raw_half_and_does_not_reclaim_selection_budget() {
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(0.0));
    let mut world = writable_book_world(vec!["A".repeat(1023)], vec![None]);
    let mut input = ClientInputState::new(true);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    sync(&mut input);
    assert!(input.handle_book_screen_text("😀", &glyphs));
    assert_eq!(book_units(book(&input)).unwrap().len(), 1024);
    assert_eq!(book_units(book(&input)).unwrap().last(), Some(&0xd83d));
    select_all(&mut input, &mut world, &glyphs);
    assert!(input.handle_book_screen_text("B", &glyphs));
    assert!(book_units(book(&input)).unwrap().is_empty());
}

#[test]
fn book_screen_sign_scroll_keeps_clicks_and_selection_in_raw_window() {
    let mut world = writable_book_world(vec![String::new()], vec![None]);
    let mut input = ClientInputState::new(true);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    assert!(book_mut(&mut input).begin_sign(String::new()));
    assert!(book_mut(&mut input).replace_title("ABC"));
    let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
    glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(40.0));
    input
        .book_screen_mut()
        .unwrap()
        .sync_input_with_layout(&glyphs);
    assert_eq!(input.book_input_state().unwrap().display_start, 1);
    input.book_screen_mut().unwrap().seek_cursor_to_point(
        Some(PhysicalPosition::new(158.0, 51.0)),
        PhysicalSize::new(320, 240),
        false,
        &glyphs,
    );
    assert_eq!(input.book_input_state().unwrap().cursor, 2);
    select_all(&mut input, &mut world, &glyphs);
    let state = input.book_input_state().unwrap();
    assert_eq!(
        (state.cursor, state.selection, state.display_start),
        (3, 0, 0)
    );
    input.shift_left_down = true;
    book_key(&mut input, &mut world, KeyCode::End, &glyphs);
    let state = input.book_input_state().unwrap();
    assert_eq!(
        (state.cursor, state.selection, state.display_start),
        (3, 0, 1)
    );
}

#[test]
fn book_screen_sign_click_uses_centered_edit_box_origin() {
    let mut world = writable_book_world(vec![String::new()], vec![None]);
    let mut input = ClientInputState::new(true);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    assert!(book_mut(&mut input).begin_sign(String::new()));
    assert!(book_mut(&mut input).replace_title("AB"));
    sync(&mut input);
    input.book_screen_mut().unwrap().seek_cursor_to_point(
        Some(PhysicalPosition::new(158.0, 51.0)),
        PhysicalSize::new(320, 240),
        false,
        &FallbackTextMeasurer,
    );
    assert_eq!(input.book_input_state().unwrap().cursor, 1);
}

fn book_lines(
    text: &str,
    measurer: &impl CommandSuggestionTextMeasurer,
) -> Vec<std::ops::Range<usize>> {
    BookPageFont(measurer.command_suggestion_glyphs())
        .lines(&text.encode_utf16().collect::<Vec<_>>())
}

#[test]
fn book_screen_lines_wrap_at_spaces_and_preserve_explicit_empty_line() {
    assert_eq!(
        book_lines("12345678901234567890 hello\n", &FallbackTextMeasurer),
        [0..19, 19..26, 27..27]
    );
}

#[test]
fn book_screen_wrap_uses_unfiltered_fractional_metrics_and_preserves_cursor_ranges() {
    use bbb_client_render::{HudAsciiGlyph, HudFontGlyphMap};
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('z', 0.0), ('w', 120.0), ('a', 57.0), ('n', -0.5)] {
        glyphs.insert_selected_codepoint_first_wins(
            ch as u32,
            HudAsciiGlyph::empty(advance),
            HudAsciiGlyph::empty(1.0),
            advance,
        );
    }
    let lines = book_lines("zzwa\n", &glyphs);
    assert_eq!(lines, [0..3, 3..4, 5..5]);
    let value: Vec<u16> = "zzwa\n".encode_utf16().collect();
    for (cursor, line) in [(3, 0), (4, 1), (5, 2)] {
        let field = MultilineTextField::new(&value, cursor, cursor, 1024, None);
        assert_eq!(field.cursor_line(&lines), line);
    }
    assert_eq!(book_lines("naa", &glyphs), std::slice::from_ref(&(0..3)));
    assert_eq!(book_lines("", &glyphs), std::slice::from_ref(&(0..0)));
}

#[test]
fn book_screen_writable_use_opens_filtered_editor_and_still_queues_use_item() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    input.set_client_text_filtering_enabled(true);
    let mut world =
        writable_book_world(vec!["raw".to_string()], vec![Some("filtered".to_string())]);

    assert!(super::super::mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        Some(LocalPlayerPoseState::default()),
    ));

    let Some(BookScreenState::Edit(screen)) = current(&input) else {
        panic!("writable book editor should be open");
    };
    assert_eq!(screen.pages[0].display(), "filtered");
    assert!(!world.local_player().interaction.using_item);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItem(_))));
    assert_eq!(
        rx.try_recv(),
        Ok(NetCommand::Swing(InteractionHand::MainHand))
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
}

#[test]
fn written_book_use_swings_but_waits_for_server_open_packet() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.set_local_player_pose(LocalPlayerPoseState::default());
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(VANILLA_ITEM_WRITTEN_BOOK_ID),
            count: 1,
            component_patch: Default::default(),
        },
    });
    let player_pose = world.local_player_pose();

    assert!(super::super::mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        None,
        player_pose,
    ));

    assert_eq!(current(&input), None);
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItem(_))));
    assert_eq!(
        rx.try_recv(),
        Ok(NetCommand::Swing(InteractionHand::MainHand))
    );
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.use_item_commands_queued, 1);
    assert_eq!(counters.swing_commands_queued, 1);
}

#[test]
fn book_screen_editor_applies_text_rules_and_done_sends_unsigned_submission() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec![String::new()], vec![None]);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    sync(&mut input);

    assert!(input.handle_book_screen_text("alpha", &FallbackTextMeasurer));
    let mut clipboard = None;
    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Enter,
        true,
        &FallbackTextMeasurer,
        &mut clipboard,
    ));
    assert!(input.handle_book_screen_text("beta", &FallbackTextMeasurer));
    assert_eq!(book_text(&input), Some("alpha\nbeta"));
    // Only Escape closes a book screen; E is a letter here.
    book_key(&mut input, &mut world, KeyCode::KeyE, &FallbackTextMeasurer);
    assert!(current(&input).is_some());

    assert!(input.handle_book_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(404.0, 197.0)),
        PhysicalSize::new(800, 600),
        &FallbackTextMeasurer,
    ));

    assert_eq!(current(&input), None);
    assert_eq!(counters.edit_book_commands_queued, 1);
    assert_eq!(
        rx.try_recv(),
        Ok(NetCommand::EditBook(EditBook {
            slot: 0,
            pages: vec!["alpha\nbeta".encode_utf16().collect()],
            title: None,
        }))
    );
}

#[test]
fn book_screen_line_limit_measures_existing_format_codes_before_accepting_edit() {
    let mut rows = vec![format!("§a{}", "A".repeat(19)); 13];
    rows.push(format!("§a{}", "A".repeat(18)));
    let original = rows.join("\n");
    let mut world = writable_book_world(vec![original.clone()], vec![None]);
    let mut input = ClientInputState::new(true);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    sync(&mut input);
    assert!(input.handle_book_screen_text("A", &FallbackTextMeasurer));
    let accepted = format!("{original}A");
    assert_eq!(book_text(&input), Some(accepted.as_str()));
    assert!(input.handle_book_screen_text("B", &FallbackTextMeasurer));
    assert_eq!(book_text(&input), Some(accepted.as_str()));
}

#[test]
fn book_screen_selection_and_wrapped_line_limit_match_editor_rules() {
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec![String::new()], vec![None]);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    sync(&mut input);
    assert!(input.handle_book_screen_text("alpha", &FallbackTextMeasurer));

    #[cfg(target_os = "macos")]
    let shortcut_key = KeyCode::SuperLeft;
    #[cfg(not(target_os = "macos"))]
    let shortcut_key = KeyCode::ControlLeft;
    input.set_key_down(shortcut_key, true);
    input.set_control_key(shortcut_key, true);
    let mut clipboard = None;
    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &None,
        KeyCode::KeyA,
        true,
        &FallbackTextMeasurer,
        &mut clipboard,
    ));
    input.set_key_down(shortcut_key, false);
    input.set_control_key(shortcut_key, false);
    assert!(input.handle_book_screen_text("beta", &FallbackTextMeasurer));
    assert_eq!(book_text(&input), Some("beta"));

    // Paste is `MultilineTextField.insertText`, the path that keeps `\n`.
    let fourteen_lines = (0..14).map(|_| "x").collect::<Vec<_>>().join("\n");
    let mut paste = |input: &mut ClientInputState, world: &mut WorldStore, text: &str| {
        let mut board = PasteBoard(text.to_string());
        let mut clipboard: Option<&mut dyn DebugClipboard> = Some(&mut board);
        input.set_key_down(shortcut_key, true);
        input.set_control_key(shortcut_key, true);
        for code in [KeyCode::KeyA, KeyCode::KeyV] {
            assert!(input.handle_book_screen_key(
                &mut counters,
                world,
                &None,
                code,
                true,
                &FallbackTextMeasurer,
                &mut clipboard,
            ));
        }
        input.set_key_down(shortcut_key, false);
        input.set_control_key(shortcut_key, false);
    };
    paste(&mut input, &mut world, &fourteen_lines);
    assert_eq!(book_text(&input), Some(fourteen_lines.as_str()));

    paste(&mut input, &mut world, &format!("{fourteen_lines}\nextra"));
    assert_eq!(book_text(&input), Some(fourteen_lines.as_str()));
    // charTyped never inserts a line break.
    assert!(input.handle_book_screen_text("\n", &FallbackTextMeasurer));
    assert_eq!(book_text(&input), Some(fourteen_lines.as_str()));
}

struct PasteBoard(String);

impl DebugClipboard for PasteBoard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        self.0 = text.to_string();
        true
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        Some(self.0.clone())
    }
}

#[test]
fn book_screen_unsigned_save_updates_local_copy_even_when_queue_unavailable() {
    for failure in ["absent", "closed", "full"] {
        let (tx, mut rx) = mpsc::channel(1);
        let commands = match failure {
            "absent" => None,
            "closed" => {
                rx.close();
                Some(tx)
            }
            "full" => {
                tx.try_send(NetCommand::Swing(InteractionHand::MainHand))
                    .unwrap();
                Some(tx)
            }
            _ => unreachable!(),
        };
        let mut counters = NetCounters::default();
        let mut input = ClientInputState::new(true);
        let mut world = writable_book_world(
            vec!["original".to_string()],
            vec![Some("filtered".to_string())],
        );
        assert!(open_editor(
            &mut input,
            &mut world,
            InteractionHand::MainHand,
            true
        ));
        sync(&mut input);
        assert!(input.handle_book_screen_text(" edit", &FallbackTextMeasurer));

        assert!(input.handle_book_screen_mouse_input(
            &mut counters,
            &mut world,
            &commands,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(404.0, 197.0)),
            PhysicalSize::new(800, 600),
            &FallbackTextMeasurer,
        ));

        assert!(current(&input).is_none(), "{failure}");
        assert!(input.book_input_state().is_none(), "{failure}");
        assert_eq!(counters.edit_book_commands_queued, 0);
        let patch = &world.inventory().hotbar_item_states()[0]
            .unwrap()
            .item
            .component_patch;
        assert_eq!(patch.writable_book_pages, ["filtered edit"], "{failure}");
        assert_eq!(patch.writable_book_page_filters, [None], "{failure}");
        assert!(open_editor(
            &mut input,
            &mut world,
            InteractionHand::MainHand,
            true
        ));
        assert_eq!(book_text(&input), Some("filtered edit"), "{failure}");
        if failure == "full" {
            assert_eq!(
                rx.try_recv().unwrap(),
                NetCommand::Swing(InteractionHand::MainHand)
            );
        }
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn lectern_page_keys_queue_buttons_without_mutating_server_page_and_escape_closes() {
    let (tx, mut rx) = mpsc::channel(3);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
        title: bbb_protocol::RichTextComponent::plain("Lectern"),
    });
    let mut item = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: Default::default(),
    };
    item.component_patch.writable_book_pages = vec!["one".into(), "two".into(), "three".into()];
    assert!(world.apply_container_set_content(ContainerSetContent {
        container_id: 7,
        state_id: 1,
        items: vec![item],
        carried_item: ItemStackSummary::empty(),
    }));
    assert!(world.apply_container_set_data(ContainerSetData {
        container_id: 7,
        id: 0,
        value: 1,
    }));

    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let mut clipboard = None;
    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::PageUp,
        true,
        &FallbackTextMeasurer,
        &mut clipboard,
    ));
    assert_eq!(world.lectern_book_view().unwrap().current_page, 1);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ContainerButtonClick(packet))
            if packet.container_id == 7 && packet.button_id == 1
    ));

    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::PageDown,
        true,
        &FallbackTextMeasurer,
        &mut clipboard,
    ));
    assert_eq!(world.lectern_book_view().unwrap().current_page, 1);
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ContainerButtonClick(packet))
            if packet.container_id == 7 && packet.button_id == 2
    ));

    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Escape,
        true,
        &FallbackTextMeasurer,
        &mut clipboard,
    ));
    assert!(world.lectern_book_view().is_none());
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::ContainerClose(packet)) if packet.container_id == 7
    ));
}

#[test]
fn book_screen_sign_closes_without_queue_and_leaves_item_for_server_update() {
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec!["original".to_string()], vec![None]);
    let original = world.inventory().hotbar_item_states()[0]
        .unwrap()
        .item
        .clone();
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    sync(&mut input);
    assert!(input.handle_book_screen_text(" edit", &FallbackTextMeasurer));
    assert!(book_mut(&mut input).begin_sign(String::new()));
    sync(&mut input);
    assert!(input.handle_book_screen_text("Title", &FallbackTextMeasurer));
    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &None,
        KeyCode::Enter,
        true,
        &FallbackTextMeasurer,
        &mut None,
    ));
    assert!(current(&input).is_none());
    assert!(input.book_input_state().is_none());
    assert_eq!(counters.edit_book_commands_queued, 0);
    assert_eq!(
        world.inventory().hotbar_item_states()[0].unwrap().item,
        &original
    );
}

#[test]
fn book_screen_sign_enter_sends_trimmed_title_without_predicting_signed_inventory() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec!["page".to_string()], vec![None]);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    sync(&mut input);
    assert!(book_mut(&mut input).begin_sign(String::new()));
    sync(&mut input);
    assert!(input.handle_book_screen_text("  Guide  ", &FallbackTextMeasurer));

    let mut clipboard = None;
    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &commands,
        KeyCode::Enter,
        true,
        &FallbackTextMeasurer,
        &mut clipboard,
    ));

    assert_eq!(current(&input), None);
    assert_eq!(
        rx.try_recv(),
        Ok(NetCommand::EditBook(EditBook {
            slot: 0,
            pages: vec!["page".encode_utf16().collect()],
            title: Some("Guide".encode_utf16().collect()),
        }))
    );
    assert_eq!(
        world.inventory().hotbar_item_states()[0]
            .unwrap()
            .item
            .component_patch
            .writable_book_pages,
        ["page"]
    );
}

#[test]
fn book_screen_sign_enter_queues_original_surrogate_units() {
    let (tx, mut rx) = mpsc::channel(1);
    let mut world = writable_book_world(vec!["page".to_string()], vec![None]);
    let mut input = ClientInputState::new(true);
    assert!(open_editor(
        &mut input,
        &mut world,
        InteractionHand::MainHand,
        false
    ));
    assert!(book_mut(&mut input).replace_page(vec![0xd800, 0x20ac]));
    assert!(book_mut(&mut input).begin_sign(String::new()));
    assert!(book_mut(&mut input).replace_title(vec![32, 0xdc00, 32]));
    assert!(input.handle_book_screen_key(
        &mut NetCounters::default(),
        &mut world,
        &Some(tx),
        KeyCode::Enter,
        true,
        &FallbackTextMeasurer,
        &mut None,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::EditBook(EditBook {
            slot: 0,
            pages: vec![vec![0xd800, 0x20ac]],
            title: Some(vec![0xdc00]),
        })
    );
    assert!(current(&input).is_none());
}

fn open_editor(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    hand: InteractionHand,
    should_filter: bool,
) -> bool {
    input.set_client_text_filtering_enabled(should_filter);
    input.open_writable_book_editor(world, &mut NetCounters::default(), &None, hand)
}

fn current(input: &ClientInputState) -> Option<&BookScreenState> {
    input.book_screen().map(|screen| &screen.book)
}

fn book(input: &ClientInputState) -> &BookScreenState {
    current(input).unwrap()
}

fn book_mut(input: &mut ClientInputState) -> &mut BookScreenState {
    &mut input.book_screen_mut().unwrap().book
}

fn sync(input: &mut ClientInputState) {
    input.book_screen_mut().unwrap().sync_input();
}

fn book_text(input: &ClientInputState) -> Option<&str> {
    match current(input)? {
        BookScreenState::Edit(screen) => screen
            .pages
            .get(screen.current_page)
            .map(bbb_client_world::BookEditorText::display),
        BookScreenState::Sign(screen) => Some(screen.title.display()),
        BookScreenState::View(_) => None,
    }
}

fn writable_book_world(pages: Vec<String>, filters: Vec<Option<String>>) -> WorldStore {
    let mut world = WorldStore::new();
    let mut item = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: Default::default(),
    };
    item.component_patch.writable_book_pages = pages.into_iter().map(Into::into).collect();
    item.component_patch.writable_book_page_filters = filters
        .into_iter()
        .map(|value| value.map(Into::into))
        .collect();
    world.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });
    world
}

#[test]
fn book_sign_button_and_enter_use_distinct_blank_rules() {
    for (title, button, submitted) in [
        (vec![], false, None),
        (vec![32], false, Some(vec![])),
        (vec![0x1c], false, Some(vec![])),
        (vec![0xa0], false, Some(vec![0xa0])),
        (vec![0x2007], false, Some(vec![0x2007])),
        (vec![0x85], true, Some(vec![0x85])),
        (vec![0xd800], true, Some(vec![0xd800])),
    ] {
        for click in [true, false] {
            let (tx, mut rx) = mpsc::channel(1);
            let commands = Some(tx);
            let mut world = writable_book_world(vec!["page".to_string()], vec![None]);
            let mut input = ClientInputState::new(true);
            assert!(open_editor(
                &mut input,
                &mut world,
                InteractionHand::MainHand,
                false
            ));
            assert!(book_mut(&mut input).begin_sign(String::new()));
            book_mut(&mut input).replace_title(title.clone());
            let mut counters = NetCounters::default();
            if click {
                input.handle_book_screen_mouse_input(
                    &mut counters,
                    &mut world,
                    &commands,
                    MouseButton::Left,
                    ElementState::Pressed,
                    Some(PhysicalPosition::new(301.0, 197.0)),
                    PhysicalSize::new(800, 600),
                    &FallbackTextMeasurer,
                );
            } else {
                input.handle_book_screen_key(
                    &mut counters,
                    &mut world,
                    &commands,
                    KeyCode::Enter,
                    true,
                    &FallbackTextMeasurer,
                    &mut None,
                );
            }
            let expected = if click && !button {
                None
            } else {
                submitted.clone()
            };
            match expected {
                Some(title) => {
                    assert!(current(&input).is_none());
                    assert_eq!(
                        rx.try_recv().unwrap(),
                        NetCommand::EditBook(EditBook {
                            slot: 0,
                            pages: vec!["page".encode_utf16().collect()],
                            title: Some(title),
                        })
                    );
                }
                None => {
                    assert!(current(&input).is_some());
                    assert!(rx.try_recv().is_err());
                }
            }
        }
    }
}

#[test]
fn dialog_over_book_returns_to_the_book_on_its_page() {
    let mut input = ClientInputState::new(true);
    use bbb_protocol::packets::{DialogHolder, OpenBook, ShowDialog};
    let mut counters = NetCounters::default();
    let mut world = writable_book_world(
        vec!["First".to_string(), "Second".to_string()],
        vec![None, None],
    );
    world.apply_open_book(OpenBook {
        hand: InteractionHand::MainHand,
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert!(book_mut(&mut input).turn_page(1));

    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 1 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert_eq!(current(&input), None);
    // A dialog replacing the dialog keeps the book it returns to.
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Reference { registry_id: 2 },
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);
    world.apply_clear_dialog();
    input.apply_screen_requests(&mut world, &mut counters, &None);

    assert_eq!(current(&input).map(BookScreenState::current_page), Some(1));
}

#[test]
fn book_screen_keys_turn_pages_close_and_block_gameplay_input() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = writable_book_world(vec!["First".into(), "Second".into()], vec![None, None]);
    open_view(&mut input, &mut world);

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(KeyCode::PageDown),
        ElementState::Pressed,
    );
    assert_eq!(input.book_screen().unwrap().book.current_page(), 1);

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert_eq!(input.book_screen(), None);

    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        winit::keyboard::PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );
    assert_eq!(counters.player_input_commands_queued, 1);
    assert!(matches!(rx.try_recv().unwrap(), NetCommand::PlayerInput(_)));
}

#[test]
fn book_screen_mouse_clicks_turn_pages_and_close() {
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec!["First".into(), "Second".into()], vec![None, None]);
    open_view(&mut input, &mut world);
    let surface_size = PhysicalSize::new(800, 600);
    let layout = bbb_client_render::hud_book_layout(surface_size);
    let origin_x = f64::from(layout.origin_x);
    let origin_y = f64::from(layout.origin_y);
    let mut click = |input: &mut ClientInputState, x: i32, y: i32| {
        assert!(input.handle_book_screen_mouse_input(
            &mut NetCounters::default(),
            &mut world,
            &None,
            MouseButton::Left,
            ElementState::Pressed,
            Some(PhysicalPosition::new(
                origin_x + f64::from(x),
                origin_y + f64::from(y),
            )),
            surface_size,
            &FallbackTextMeasurer,
        ));
    };

    click(
        &mut input,
        BOOK_PAGE_FORWARD_BUTTON_X + 1,
        BOOK_PAGE_BUTTON_Y + 1,
    );
    assert_eq!(input.book_screen().unwrap().book.current_page(), 1);

    click(
        &mut input,
        BOOK_MENU_DONE_BUTTON_X + 10,
        BOOK_MENU_BUTTON_Y + 1,
    );
    assert_eq!(input.book_screen(), None);
}

#[test]
fn book_screen_odd_gui_width_shares_integer_draw_and_hit_edges() {
    let surface_size = PhysicalSize::new(427, 240);
    let layout = bbb_client_render::hud_book_layout(surface_size);
    assert_eq!((layout.origin_x, layout.origin_y), (117, 2));

    let world = writable_book_world(vec!["First".into(), "Second".into()], vec![None, None]);
    let mut input = ClientInputState::new(true);
    let mut world = world;
    open_view(&mut input, &mut world);
    let view = &input.book_screen().unwrap().book;

    let done_left = f64::from(layout.origin_x + BOOK_MENU_DONE_BUTTON_X);
    let done_right = done_left + f64::from(BOOK_MENU_BUTTON_WIDTH);
    let done_y = f64::from(layout.origin_y + BOOK_MENU_BUTTON_Y + 1);
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(done_left - 0.1, done_y)),
            surface_size
        ),
        None
    );
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(done_left, done_y)),
            surface_size
        ),
        Some(BookClickTarget::Done)
    );
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(done_right - 0.1, done_y)),
            surface_size
        ),
        Some(BookClickTarget::Done)
    );
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(done_right, done_y)),
            surface_size
        ),
        None
    );

    let forward_left = f64::from(layout.origin_x + BOOK_PAGE_FORWARD_BUTTON_X);
    let forward_right = forward_left + f64::from(BOOK_PAGE_BUTTON_WIDTH);
    let forward_y = f64::from(layout.origin_y + BOOK_PAGE_BUTTON_Y + 1);
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(forward_left - 0.1, forward_y)),
            surface_size
        ),
        None
    );
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(forward_left, forward_y)),
            surface_size
        ),
        Some(BookClickTarget::NextPage)
    );
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(forward_right - 0.1, forward_y)),
            surface_size
        ),
        Some(BookClickTarget::NextPage)
    );
    assert_eq!(
        book_click_target(
            view,
            Some(PhysicalPosition::new(forward_right, forward_y)),
            surface_size
        ),
        None
    );
}

fn open_view(input: &mut ClientInputState, world: &mut WorldStore) {
    world.apply_open_book(bbb_protocol::packets::OpenBook {
        hand: InteractionHand::MainHand,
    });
    input.apply_screen_requests(world, &mut NetCounters::default(), &None);
}
