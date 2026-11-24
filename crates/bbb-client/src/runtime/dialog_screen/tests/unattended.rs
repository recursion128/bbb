use super::*;
use crate::input::FallbackTextMeasurer;
use bbb_protocol::packets::{
    CommandArgumentParser, CommandNode, Commands, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
};

fn install_commands(world: &mut WorldStore) {
    assert!(world.apply_commands(Commands {
        root_index: 0,
        nodes: vec![
            CommandNode::root().with_children(vec![1, 2, 3]),
            CommandNode::literal("help").executes(),
            CommandNode::literal("seed").restrict().executes(),
            CommandNode::literal("say").with_children(vec![4]),
            CommandNode::argument(
                "message",
                CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                    name: "minecraft:message".into(),
                    properties: vec![],
                }
            )
            .executes(),
        ],
    }));
}

#[test]
fn unattended_dialog_after_action_runs_only_after_accept() {
    for after in ["none", "close", "wait_for_response"] {
        for command in ["/help", "/seed", "/missing", "/say hello"] {
            let mut input = crate::input::ClientInputState::new(true);
            let mut world = world_with_notice(&mut input, vec![]);
            install_commands(&mut world);
            let mut counters = NetCounters::default();
            let (tx, mut rx) = mpsc::channel(8);
            let commands = Some(tx);
            let action = DialogActionState::RunCommand {
                command: command.into(),
            };
            let revision = input.dialog_screen_revision();
            assert!(run_dialog_action(
                &mut input,
                &mut counters,
                &mut world,
                &commands,
                None,
                Some(&action),
                after
            ));
            if command != "/help" {
                assert!(input.command_confirmation().is_some());
                assert!(rx.try_recv().is_err());
                assert_eq!(input.dialog_screen_revision(), revision);
                assert_eq!(input.dialog_waiting_ticks(), None);
                input.resolve_command_confirmation(
                    false,
                    &mut world,
                    &mut counters,
                    &commands,
                    &FallbackTextMeasurer,
                );
                assert!(input.current_dialog().is_some());
                assert_eq!(input.dialog_screen_revision(), revision);
                assert!(rx.try_recv().is_err());
                assert!(run_dialog_action(
                    &mut input,
                    &mut counters,
                    &mut world,
                    &commands,
                    None,
                    Some(&action),
                    after
                ));
                input.resolve_command_confirmation(
                    true,
                    &mut world,
                    &mut counters,
                    &commands,
                    &FallbackTextMeasurer,
                );
            }
            assert!(input.command_confirmation().is_none());
            assert_eq!(input.current_dialog().is_some(), after == "none");
            assert_eq!(
                input.dialog_waiting_ticks(),
                (after == "wait_for_response").then_some(0)
            );
            if command == "/say hello" {
                assert!(rx.try_recv().is_err());
                if after == "close" {
                    assert_eq!(
                        input
                            .chat_screen_status(bbb_client_world::ChatVisibilityState::Full)
                            .unwrap()
                            .input_text,
                        "/say hello"
                    );
                    assert!(input.take_command_confirmation_clipboard().is_empty());
                } else {
                    assert!(!input.chat_entry_is_active());
                    assert_eq!(input.take_command_confirmation_clipboard(), ["/say hello"]);
                }
            } else {
                assert!(matches!(rx.try_recv(), Ok(NetCommand::ChatCommand(packet))
                    if packet.command == command.trim_start_matches('/') && packet.signing.is_none()));
            }
        }
    }
}

#[test]
fn unattended_dialog_signature_close_copies_when_dialog_parent_exists() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    input.set_screen_for_test(crate::input::Screen::Advancements(Default::default()));
    apply_notice(&mut input, &mut world, vec![]);
    install_commands(&mut world);
    let mut counters = NetCounters::default();
    input.start_dialog_unattended_command(
        &"/say hello".into(),
        "close",
        &mut world,
        &mut counters,
        &None,
    );
    assert!(!input.command_confirmation().unwrap().open_chat_on_accept);
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &None,
        &FallbackTextMeasurer,
    );
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert!(input.advancements_screen_is_open());
    assert_eq!(input.take_command_confirmation_clipboard(), ["/say hello"]);
    assert!(!input.chat_entry_is_active());
}

#[test]
fn unattended_container_replacement_invalidates_pending_dialog_after_action() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, vec![]);
    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    input.start_dialog_unattended_command(
        &"/seed".into(),
        "wait_for_response",
        &mut world,
        &mut counters,
        &commands,
    );
    world.apply_open_screen(bbb_protocol::packets::OpenScreen {
        container_id: 2,
        menu_type_id: 0,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let revision = input.dialog_screen_revision();
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &commands,
        &FallbackTextMeasurer,
    );
    assert_eq!(input.dialog_screen_revision(), revision);
    assert!(input.current_dialog().is_none());
    assert_eq!(input.dialog_waiting_ticks(), None);
    assert!(rx.try_recv().is_err());
}

#[test]
fn unattended_dialog_escape_command_cancels_to_dialog_then_forces_close_on_accept() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(
        &mut input,
        vec![
            entry(
                "after_action",
                NbtSummaryValue::String("wait_for_response".into()),
            ),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("label", NbtSummaryValue::String("Run".into())),
                    entry(
                        "action",
                        NbtSummaryValue::Compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:run_command".into()),
                            ),
                            entry("command", NbtSummaryValue::String("/seed".into())),
                        ]),
                    ),
                ]),
            ),
        ],
    );
    let mut counters = NetCounters::default();
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    for accept in [false, true] {
        assert!(handle_dialog_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            None,
            PhysicalKey::Code(KeyCode::Escape),
            ElementState::Pressed
        ));
        assert!(input.command_confirmation().is_some());
        assert!(input.current_dialog().is_some());
        assert!(rx.try_recv().is_err());
        input.resolve_command_confirmation(
            accept,
            &mut world,
            &mut counters,
            &commands,
            &FallbackTextMeasurer,
        );
        assert_eq!(input.current_dialog().is_some(), !accept);
        assert_eq!(input.dialog_waiting_ticks(), None);
    }
    assert!(
        matches!(rx.try_recv(), Ok(NetCommand::ChatCommand(packet)) if packet.signing.is_none())
    );
}

#[test]
fn unattended_dialog_signature_close_restores_native_chat_and_copies() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();
    crate::input::handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::KeyT),
        ElementState::Pressed,
    );
    assert!(input.replace_chat_entry_text("parent draft".into()));
    apply_notice(&mut input, &mut world, vec![]);
    install_commands(&mut world);
    input.start_dialog_unattended_command(
        &"/say hello".into(),
        "close",
        &mut world,
        &mut counters,
        &None,
    );
    assert!(!input.command_confirmation().unwrap().open_chat_on_accept);
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &None,
        &FallbackTextMeasurer,
    );
    assert_eq!(input.take_command_confirmation_clipboard(), ["/say hello"]);
    assert_eq!(
        input
            .chat_screen_status(bbb_client_world::ChatVisibilityState::Full)
            .unwrap()
            .input_text,
        "parent draft"
    );
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert_eq!(
        input
            .chat_screen_status(bbb_client_world::ChatVisibilityState::Full)
            .unwrap()
            .input_text,
        "parent draft"
    );
}

#[test]
fn unattended_dialog_signature_copies_when_chat_is_hidden() {
    let mut input = crate::input::ClientInputState::new(true);
    let mut world = world_with_notice(&mut input, vec![]);
    install_commands(&mut world);
    let mut config = world.chat_presentation_config().clone();
    config.visibility = bbb_client_world::ChatVisibilityState::Hidden;
    world.set_chat_presentation_config(config);
    let mut counters = NetCounters::default();
    input.start_dialog_unattended_command(
        &"/say hello".into(),
        "close",
        &mut world,
        &mut counters,
        &None,
    );
    assert!(!input.command_confirmation().unwrap().open_chat_on_accept);
    input.resolve_command_confirmation(
        true,
        &mut world,
        &mut counters,
        &None,
        &FallbackTextMeasurer,
    );
    assert!(input.current_dialog().is_none());
    assert!(!input.chat_entry_is_active());
    assert_eq!(input.take_command_confirmation_clipboard(), ["/say hello"]);
}
