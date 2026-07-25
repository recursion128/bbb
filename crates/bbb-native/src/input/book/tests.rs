use bbb_net::NetCommand;
use bbb_protocol::{
    item_types::{VANILLA_ITEM_WRITABLE_BOOK_ID, VANILLA_ITEM_WRITTEN_BOOK_ID},
    packets::{
        InteractionHand, ItemStackSummary, SetPlayerInventory as ProtocolSetPlayerInventory,
    },
};
use bbb_world::LocalPlayerPoseState;

use super::*;
use crate::input::FallbackTextMeasurer;

#[test]
fn book_screen_lines_wrap_at_spaces_and_preserve_explicit_empty_line() {
    let lines = book_text_lines("12345678901234567890 hello\n", &FallbackTextMeasurer);

    assert_eq!(
        lines,
        vec![
            BookTextLine { start: 0, end: 19 },
            BookTextLine { start: 19, end: 26 },
            BookTextLine { start: 27, end: 27 },
        ]
    );
}

#[test]
fn book_screen_allowed_characters_match_vanilla_chat_filter_with_page_newlines() {
    assert!(allowed_book_char('A', false));
    assert!(!allowed_book_char('\n', false));
    assert!(allowed_book_char('\n', true));
    assert!(!allowed_book_char('\u{a7}', true));
    assert!(!allowed_book_char('\u{7f}', true));
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

    let Some(BookScreenState::Edit(screen)) = world.current_book() else {
        panic!("writable book editor should be open");
    };
    assert_eq!(screen.pages, ["filtered"]);
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

    assert_eq!(world.current_book(), None);
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
    assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
    input.sync_book_screen(&world);

    assert!(input.handle_book_screen_text(&mut world, "alpha", &FallbackTextMeasurer));
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
    assert!(input.handle_book_screen_text(&mut world, "beta", &FallbackTextMeasurer));
    assert_eq!(current_book_text(&world), Some("alpha\nbeta"));

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

    assert_eq!(world.current_book(), None);
    assert_eq!(counters.edit_book_commands_queued, 1);
    assert_eq!(
        rx.try_recv(),
        Ok(NetCommand::EditBook(EditBook {
            slot: 0,
            pages: vec!["alpha\nbeta".to_string()],
            title: None,
        }))
    );
}

#[test]
fn book_screen_selection_and_wrapped_line_limit_match_editor_rules() {
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec![String::new()], vec![None]);
    assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
    input.sync_book_screen(&world);
    assert!(input.handle_book_screen_text(&mut world, "alpha", &FallbackTextMeasurer));

    input.set_control_key(KeyCode::ControlLeft, true);
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
    input.set_control_key(KeyCode::ControlLeft, false);
    assert!(input.handle_book_screen_text(&mut world, "beta", &FallbackTextMeasurer));
    assert_eq!(current_book_text(&world), Some("beta"));

    let fourteen_lines = (0..14).map(|_| "x").collect::<Vec<_>>().join("\n");
    input.set_control_key(KeyCode::ControlLeft, true);
    assert!(input.handle_book_screen_key(
        &mut counters,
        &mut world,
        &None,
        KeyCode::KeyA,
        true,
        &FallbackTextMeasurer,
        &mut clipboard,
    ));
    input.set_control_key(KeyCode::ControlLeft, false);
    assert!(input.handle_book_screen_text(&mut world, &fourteen_lines, &FallbackTextMeasurer));
    assert_eq!(current_book_text(&world), Some(fourteen_lines.as_str()));

    assert!(input.handle_book_screen_text(&mut world, "\nextra", &FallbackTextMeasurer));
    assert_eq!(current_book_text(&world), Some(fourteen_lines.as_str()));
}

#[test]
fn book_screen_failed_unsigned_queue_retains_editor_and_local_inventory() {
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec!["original".to_string()], vec![None]);
    assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
    input.sync_book_screen(&world);
    assert!(input.handle_book_screen_text(&mut world, " edit", &FallbackTextMeasurer));

    assert!(input.handle_book_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(404.0, 197.0)),
        PhysicalSize::new(800, 600),
        &FallbackTextMeasurer,
    ));

    assert!(world.current_book().is_some_and(BookScreenState::is_edit));
    assert_eq!(counters.edit_book_commands_queued, 0);
    assert_eq!(
        world.inventory().hotbar_item_states()[0]
            .item
            .component_patch
            .writable_book_pages,
        ["original"]
    );
}

#[test]
fn book_screen_sign_enter_sends_trimmed_title_without_predicting_signed_inventory() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();
    let mut input = ClientInputState::new(true);
    let mut world = writable_book_world(vec!["page".to_string()], vec![None]);
    assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
    input.sync_book_screen(&world);
    let generation = world.current_book().unwrap().generation();
    assert!(world.begin_sign_current_book(generation));
    input.sync_book_screen(&world);
    assert!(input.handle_book_screen_text(&mut world, "  Guide  ", &FallbackTextMeasurer));

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

    assert_eq!(world.current_book(), None);
    assert_eq!(
        rx.try_recv(),
        Ok(NetCommand::EditBook(EditBook {
            slot: 0,
            pages: vec!["page".to_string()],
            title: Some("Guide".to_string()),
        }))
    );
    assert_eq!(
        world.inventory().hotbar_item_states()[0]
            .item
            .component_patch
            .writable_book_pages,
        ["page"]
    );
}

fn writable_book_world(pages: Vec<String>, filters: Vec<Option<String>>) -> WorldStore {
    let mut world = WorldStore::new();
    let mut item = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WRITABLE_BOOK_ID),
        count: 1,
        component_patch: Default::default(),
    };
    item.component_patch.writable_book_pages = pages;
    item.component_patch.writable_book_page_filters = filters;
    world.apply_set_player_inventory(ProtocolSetPlayerInventory { slot: 0, item });
    world
}
