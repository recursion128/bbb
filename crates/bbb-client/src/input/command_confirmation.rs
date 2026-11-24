use super::*;
use bbb_client_render::{HudCommandConfirmationButton as Button, HudCommandConfirmationHitTarget};
use bbb_client_world::UnattendedCommandCheck;

#[derive(Debug, Clone, Copy, PartialEq)]
enum CommandAfter {
    KeepCurrent,
    CloseScreen,
    CloseDialog,
    WaitForDialogResponse,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandConfirmation {
    pub(crate) generation: u64,
    pub(crate) presentation: u64,
    /// The command the click asked for, without its optional `/`
    /// (`Commands.trimOptionalPrefix`), in the units the payload carried.
    pub(crate) command: bbb_protocol::JavaString,
    pub(crate) reason: UnattendedCommandCheck,
    pub(crate) focused: Option<Button>,
    pub(crate) open_chat_on_accept: bool,
    pub(crate) has_parent: bool,
    after: CommandAfter,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandConfirmationNarrationSnapshot {
    pub(crate) generation: u64,
    pub(crate) presentation: u64,
    pub(crate) command: bbb_protocol::JavaString,
    pub(crate) reason: UnattendedCommandCheck,
    pub(crate) focused: Option<Button>,
    pub(crate) open_chat_on_accept: bool,
    pub(crate) has_parent: bool,
    pub(crate) retained: Vec<u64>,
    pub(crate) last_input_type: LastInputType,
    pub(crate) at: Instant,
}

impl ClientInputState {
    pub(super) fn restore_command_confirmation_presentation(&mut self) {
        if self.screen.command_confirmation().is_some() {
            self.command_confirmation_generation =
                self.command_confirmation_generation.saturating_add(1);
            self.screen.command_confirmation_mut().unwrap().presentation =
                self.command_confirmation_generation;
            self.queue_command_confirmation_narration();
        }
    }
    pub(crate) fn retained_command_confirmation_generations(&self) -> Vec<u64> {
        self.screen
            .iter()
            .filter_map(|screen| match screen {
                Screen::CommandConfirmation(screen) => Some(screen.generation),
                _ => None,
            })
            .collect()
    }
    pub(crate) fn command_confirmation_narration_snapshot(
        &self,
    ) -> Option<CommandConfirmationNarrationSnapshot> {
        let screen = self.screen.command_confirmation()?;
        Some(CommandConfirmationNarrationSnapshot {
            generation: screen.generation,
            presentation: screen.presentation,
            command: screen.command.clone(),
            reason: screen.reason,
            focused: screen.focused,
            open_chat_on_accept: screen.open_chat_on_accept,
            has_parent: screen.has_parent,
            retained: self.retained_command_confirmation_generations(),
            last_input_type: self.last_input_type(),
            at: bbb_client_clock::now(),
        })
    }

    fn queue_command_confirmation_narration(&mut self) {
        if let Some(snapshot) = self.command_confirmation_narration_snapshot() {
            self.command_confirmation_narrations.push(snapshot);
        }
    }

    pub(crate) fn take_command_confirmation_narrations(
        &mut self,
    ) -> Vec<CommandConfirmationNarrationSnapshot> {
        std::mem::take(&mut self.command_confirmation_narrations)
    }

    pub(crate) fn command_confirmation(&self) -> Option<&CommandConfirmation> {
        self.screen.command_confirmation()
    }

    pub(crate) fn start_unattended_command(
        &mut self,
        command: &bbb_protocol::JavaString,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.start_command_confirmation(
            command,
            CommandAfter::KeepCurrent,
            world,
            counters,
            commands,
        );
    }

    /// `BookViewScreen.handleClickEvent` passes `screenAfterCommand = null`
    pub(crate) fn start_book_unattended_command(
        &mut self,
        command: &bbb_protocol::JavaString,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.start_command_confirmation(
            command,
            CommandAfter::CloseScreen,
            world,
            counters,
            commands,
        );
    }

    pub(crate) fn start_dialog_unattended_command(
        &mut self,
        command: &bbb_protocol::JavaString,
        after_action: &str,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let after = match after_action {
            "close" => CommandAfter::CloseDialog,
            "wait_for_response" => CommandAfter::WaitForDialogResponse,
            _ => CommandAfter::KeepCurrent,
        };
        self.start_command_confirmation(command, after, world, counters, commands);
    }

    fn start_command_confirmation(
        &mut self,
        command: &bbb_protocol::JavaString,
        after: CommandAfter,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let units = command.units();
        let command = bbb_protocol::JavaString::from(
            units
                .strip_prefix(&[u16::from(b'/')][..])
                .unwrap_or(units)
                .to_vec(),
        );
        let reason = world.check_unattended_command_utf16(command.units());
        if reason == UnattendedCommandCheck::NoIssues {
            send_unsigned(&command, counters, commands);
            self.apply_command_after(after, world, counters, commands);
            return;
        }
        let has_parent = self.screen.is_open();
        let destination_is_empty = match after {
            CommandAfter::KeepCurrent => !has_parent,
            CommandAfter::CloseScreen => true,
            CommandAfter::WaitForDialogResponse => false,
            CommandAfter::CloseDialog => !self.dialog_close_returns_screen(),
        };
        let open_chat_on_accept = destination_is_empty
            && world.chat_presentation_config().visibility
                != bbb_client_world::ChatVisibilityState::Hidden;
        release_active_input(self, world, counters, commands);
        self.reset_chat_scroll();
        self.command_confirmation_generation =
            self.command_confirmation_generation.saturating_add(1);
        self.screen
            .open_child(Screen::CommandConfirmation(CommandConfirmation {
                generation: self.command_confirmation_generation,
                presentation: self.command_confirmation_generation,
                command,
                reason,
                focused: self
                    .last_input_type()
                    .is_keyboard()
                    .then_some(Button::Accept),
                open_chat_on_accept,
                has_parent,
                after,
            }));
        self.queue_command_confirmation_narration();
    }

    pub(crate) fn resolve_command_confirmation(
        &mut self,
        accept: bool,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        let Some(Screen::CommandConfirmation(screen)) = self
            .screen
            .close_if(|screen| matches!(screen, Screen::CommandConfirmation(_)))
        else {
            return;
        };
        if !accept {
            return;
        }
        if screen.reason != UnattendedCommandCheck::SignatureRequired {
            send_unsigned(&screen.command, counters, commands);
            self.apply_command_after(screen.after, world, counters, commands);
        } else if screen.open_chat_on_accept {
            self.apply_command_after(screen.after, world, counters, commands);
            open_chat_entry_with_measurer(
                self,
                world,
                counters,
                commands,
                ChatMethod::Command,
                false,
                measurer,
            );
            self.insert_chat_screen_text(&screen.command, counters, world, commands, measurer);
        } else {
            self.command_confirmation_clipboard
                .push(bbb_protocol::JavaString::from(
                    std::iter::once(u16::from(b'/'))
                        .chain(screen.command.units().iter().copied())
                        .collect::<Vec<_>>(),
                ));
            self.apply_command_after(screen.after, world, counters, commands);
        }
    }

    fn apply_command_after(
        &mut self,
        after: CommandAfter,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        match after {
            CommandAfter::KeepCurrent => {}
            CommandAfter::CloseScreen => {
                self.close_screens(counters, commands);
            }
            CommandAfter::CloseDialog => {
                if self.close_dialog() {
                    self.refresh_dialog_parent_suggestions(world, counters, commands);
                }
            }
            CommandAfter::WaitForDialogResponse => {
                self.wait_for_dialog_response();
            }
        }
    }

    pub(crate) fn handle_command_confirmation_key(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
        code: KeyCode,
        pressed: bool,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) -> bool {
        let shift_down = self.shift_down();
        let Some(screen) = self.screen.command_confirmation_mut() else {
            return false;
        };
        if !pressed {
            return true;
        }
        match code {
            KeyCode::Escape => {
                self.resolve_command_confirmation(false, world, counters, commands, measurer)
            }
            KeyCode::Tab => {
                screen.focused = Some(match screen.focused {
                    Some(Button::Accept) => Button::Cancel,
                    Some(Button::Cancel) => Button::Accept,
                    None if shift_down => Button::Cancel,
                    None => Button::Accept,
                });
            }
            KeyCode::ArrowLeft => {
                screen.focused = Some(match screen.focused {
                    None => Button::Cancel,
                    Some(_) => Button::Accept,
                });
            }
            KeyCode::ArrowRight => {
                screen.focused = Some(match screen.focused {
                    None => Button::Accept,
                    Some(_) => Button::Cancel,
                });
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                screen.focused.get_or_insert(Button::Accept);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(button) = screen.focused {
                    self.resolve_command_confirmation(
                        button == Button::Accept,
                        world,
                        counters,
                        commands,
                        measurer,
                    );
                }
            }
            _ => {}
        }
        true
    }

    pub(crate) fn click_command_confirmation(
        &mut self,
        target: Option<HudCommandConfirmationHitTarget>,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        commands: &Option<mpsc::Sender<NetCommand>>,
        measurer: &impl CommandSuggestionTextMeasurer,
    ) {
        if let Some(target) = target.filter(|target| {
            self.screen
                .command_confirmation()
                .is_some_and(|screen| screen.generation == target.generation)
        }) {
            self.resolve_command_confirmation(
                target.button == Button::Accept,
                world,
                counters,
                commands,
                measurer,
            );
        }
    }

    pub(crate) fn take_command_confirmation_clipboard(&mut self) -> Vec<bbb_protocol::JavaString> {
        std::mem::take(&mut self.command_confirmation_clipboard)
    }
}

fn send_unsigned(
    command: &bbb_protocol::JavaString,
    counters: &mut NetCounters,
    commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if commands.as_ref().is_some_and(|tx| {
        tx.try_send(NetCommand::ChatCommand(
            bbb_client_net::OutboundChatCommand::unsigned(command.clone()),
        ))
        .is_ok()
    }) {
        counters.chat_command_commands_queued += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        CommandArgumentParser, CommandNode, Commands, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
    };

    fn world() -> WorldStore {
        let mut world = WorldStore::new();
        assert!(world.apply_commands(Commands {
            root_index: 0,
            nodes: vec![
                CommandNode::root().with_children(vec![1, 2, 3]),
                CommandNode::literal("help").executes(),
                CommandNode::literal("seed").executes().restrict(),
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
        world
    }

    #[test]
    fn command_confirmation_focus_navigation_preserves_mouse_and_keyboard_contract() {
        let mut input = ClientInputState::new(true);
        let mut world = world();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(16);
        let commands = Some(tx);
        input.last_input_type = LastInputType::Mouse;
        input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &commands);
        assert_eq!(input.command_confirmation().unwrap().focused, None);
        for key in [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space] {
            handle_key_input(
                &mut input,
                &mut counters,
                &mut world,
                &commands,
                PhysicalKey::Code(key),
                ElementState::Pressed,
            );
        }
        assert!(input.command_confirmation().is_some());
        assert!(rx.try_recv().is_err());
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::ShiftLeft),
            ElementState::Pressed,
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
        );
        assert_eq!(
            input.command_confirmation().unwrap().focused,
            Some(Button::Cancel)
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
        );
        assert_eq!(
            input.command_confirmation().unwrap().focused,
            Some(Button::Accept)
        );
        for (key, expected) in [
            (KeyCode::ArrowLeft, Button::Accept),
            (KeyCode::ArrowUp, Button::Accept),
            (KeyCode::ArrowRight, Button::Cancel),
            (KeyCode::ArrowRight, Button::Cancel),
            (KeyCode::ArrowDown, Button::Cancel),
            (KeyCode::ArrowLeft, Button::Accept),
        ] {
            handle_key_input(
                &mut input,
                &mut counters,
                &mut world,
                &commands,
                PhysicalKey::Code(key),
                ElementState::Pressed,
            );
            assert_eq!(
                input.command_confirmation().unwrap().focused,
                Some(expected)
            );
        }
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &commands,
            PhysicalKey::Code(KeyCode::Enter),
            ElementState::Pressed,
        );
        assert!(input.command_confirmation().is_none());
        assert!(matches!(rx.try_recv(), Ok(NetCommand::ChatCommand(_))));
        input.last_input_type = LastInputType::KeyboardTab;
        input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &commands);
        assert_eq!(
            input.command_confirmation().unwrap().focused,
            Some(Button::Accept)
        );
    }

    #[test]
    fn command_confirmation_navigation_enters_unfocused_button_row_from_each_direction() {
        for (key, expected) in [
            (KeyCode::Tab, Button::Accept),
            (KeyCode::ArrowLeft, Button::Cancel),
            (KeyCode::ArrowRight, Button::Accept),
            (KeyCode::ArrowUp, Button::Accept),
            (KeyCode::ArrowDown, Button::Accept),
        ] {
            let mut input = ClientInputState::new(true);
            let mut world = world();
            let mut counters = NetCounters::default();
            input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &None);
            input.handle_command_confirmation_key(
                &mut world,
                &mut counters,
                &None,
                key,
                true,
                &FallbackTextMeasurer,
            );
            assert_eq!(
                input.command_confirmation().unwrap().focused,
                Some(expected)
            );
        }
    }

    #[test]
    fn unattended_confirmation_never_sends_until_accept_and_sends_unsigned() {
        let mut input = ClientInputState::new(true);
        let mut world = world();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(16);
        let commands = Some(tx);
        input.start_unattended_command(&"/help".into(), &mut world, &mut counters, &commands);
        assert!(input.command_confirmation().is_none());
        assert!(matches!(rx.try_recv(), Ok(NetCommand::ChatCommand(command))
            if command.command == "help" && command.signing.is_none()));
        for (text, reason) in [
            ("/seed", UnattendedCommandCheck::PermissionsRequired),
            ("/missing", UnattendedCommandCheck::ParseErrors),
        ] {
            input.start_unattended_command(&text.into(), &mut world, &mut counters, &commands);
            assert_eq!(input.command_confirmation().unwrap().reason, reason);
            assert!(rx.try_recv().is_err());
            let generation = input.command_confirmation().unwrap().generation;
            input.click_command_confirmation(
                Some(HudCommandConfirmationHitTarget {
                    generation: generation - 1,
                    button: Button::Accept,
                }),
                &mut world,
                &mut counters,
                &commands,
                &FallbackTextMeasurer,
            );
            assert!(input.command_confirmation().is_some());
            input.resolve_command_confirmation(
                true,
                &mut world,
                &mut counters,
                &commands,
                &FallbackTextMeasurer,
            );
            assert!(
                matches!(rx.try_recv(), Ok(NetCommand::ChatCommand(command)) if command.signing.is_none())
            );
        }
    }

    #[test]
    fn unattended_signed_confirmation_opens_chat_or_copies_and_restores_parent() {
        let mut input = ClientInputState::new(true);
        let mut world = world();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(16);
        let commands = Some(tx);
        input.start_unattended_command(&"/say hello".into(), &mut world, &mut counters, &commands);
        assert_eq!(
            input.command_confirmation().unwrap().reason,
            UnattendedCommandCheck::SignatureRequired
        );
        input.resolve_command_confirmation(
            true,
            &mut world,
            &mut counters,
            &commands,
            &FallbackTextMeasurer,
        );
        assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/say hello");
        assert!(rx.try_recv().is_err());
        input.start_unattended_command(&"/say other".into(), &mut world, &mut counters, &commands);
        assert!(input.screen.chat_entry().is_none());
        input.resolve_command_confirmation(
            true,
            &mut world,
            &mut counters,
            &commands,
            &FallbackTextMeasurer,
        );
        assert_eq!(input.take_command_confirmation_clipboard(), ["/say other"]);
        assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/say hello");
        assert!(rx.try_recv().is_err());
        input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &commands);
        input.handle_command_confirmation_key(
            &mut world,
            &mut counters,
            &commands,
            KeyCode::Escape,
            true,
            &FallbackTextMeasurer,
        );
        assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "/say hello");
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn unattended_confirmation_consumes_text_and_releases_modifiers_before_cancel() {
        let mut input = ClientInputState::new(true);
        let mut world = world();
        let mut counters = NetCounters::default();
        open_chat_entry_with_measurer(
            &mut input,
            &mut world,
            &mut counters,
            &None,
            ChatMethod::Message,
            false,
            &FallbackTextMeasurer,
        );
        input.replace_chat_entry_text("draft".into());
        input.set_control_key(KeyCode::ControlLeft, true);
        input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &None);
        handle_text_input_with_item_runtime(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            None,
            "leak",
            &FallbackTextMeasurer,
        );
        assert!(!input.text_input_is_active(&world));
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::ControlLeft),
            ElementState::Released,
        );
        input.resolve_command_confirmation(
            false,
            &mut world,
            &mut counters,
            &None,
            &FallbackTextMeasurer,
        );
        assert!(!input.control_down());
        assert_eq!(input.screen.chat_entry().unwrap().edit.text(), "draft");
    }

    #[test]
    fn unattended_confirmation_does_not_restore_chat_after_server_screen_replacement() {
        let mut input = ClientInputState::new(true);
        let mut world = world();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(16);
        let commands = Some(tx);
        open_chat_entry_with_measurer(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            ChatMethod::Message,
            false,
            &FallbackTextMeasurer,
        );
        input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &commands);
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 2,
            menu_type_id: 0,
            title: bbb_protocol::RichTextComponent::plain("Chest"),
        });
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        input.resolve_command_confirmation(
            true,
            &mut world,
            &mut counters,
            &commands,
            &FallbackTextMeasurer,
        );
        assert!(input.command_confirmation().is_none());
        assert!(input.screen.chat_entry().is_none());
        assert!(rx.try_recv().is_err());

        input.start_unattended_command(&"/seed".into(), &mut world, &mut counters, &commands);
        world.apply_open_screen(bbb_protocol::packets::OpenScreen {
            container_id: 2,
            menu_type_id: 0,
            title: bbb_protocol::RichTextComponent::plain("Replacement chest"),
        });
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        input.resolve_command_confirmation(
            true,
            &mut world,
            &mut counters,
            &commands,
            &FallbackTextMeasurer,
        );
        assert!(input.command_confirmation().is_none());
        assert!(rx.try_recv().is_err());
    }
}
