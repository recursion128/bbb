use super::*;

#[derive(Default)]
pub(super) struct HudMouseHitTargets {
    pub(super) chat: Option<HudChatHitTarget>,
    pub(super) book: Option<HudBookHitTarget>,
    pub(super) disconnected_screen: Option<HudDisconnectedScreenHitTarget>,
    pub(super) command_confirmation: Option<bbb_client_render::HudCommandConfirmationHitTarget>,
    pub(super) death_screen: Option<HudDeathScreenHitTarget>,
    pub(super) game_event: Option<HudGameEventHitTarget>,
    pub(super) social_interactions: Option<HudSocialInteractionsHitTarget>,
}

impl SimulationInputRuntime {
    pub(super) fn apply_control_mouse_button(
        &mut self,
        button: crate::control::InputMouseButtonControl,
        state: ElementState,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        self.apply_mouse_button(
            crate::runtime::input_mouse_button(button),
            state,
            None,
            HudMouseHitTargets::default(),
            context,
        );
    }

    pub(super) fn apply_mouse_button(
        &mut self,
        button: MouseButton,
        state: ElementState,
        position: Option<PhysicalPosition<f64>>,
        targets: HudMouseHitTargets,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        let HudMouseHitTargets {
            chat: chat_target,
            book: book_target,
            disconnected_screen: disconnected_screen_target,
            command_confirmation: command_confirmation_target,
            death_screen: death_screen_target,
            game_event: game_event_target,
            social_interactions: social_interactions_target,
        } = targets;
        self.active_mouse_button = (state == ElementState::Pressed).then_some(button);
        self.mouse_button_events = self.mouse_button_events.saturating_add(1);
        if button == MouseButton::Left {
            self.left_mouse_button_events = self.left_mouse_button_events.saturating_add(1);
            if state == ElementState::Pressed {
                self.left_mouse_button_press_events =
                    self.left_mouse_button_press_events.saturating_add(1);
            }
        }
        // A click re-projects the raw physical cache with the current GUI
        // projection (`MouseHandler.onButton` -> `getScaledXPos`); the screen
        // handlers below read the runtime's logical cursor position, so
        // publish the click projection before they run rather than letting a
        // stale pre-resize logical cache reach them.
        if let Some(position) = position {
            self.cursor_position = Some(position);
        }
        context.input.note_mouse_button_input_type();
        let screen_was_open = context.input.current_screen().is_some()
            || self.code_of_conduct_overlay.is_visible(context.world);
        match context.input.current_screen() {
            Some(Screen::CommandConfirmation(_)) => {
                if matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
                    context.input.click_command_confirmation(
                        command_confirmation_target,
                        context.world,
                        context.net_counters,
                        context.net_commands,
                        &self.text_measurements,
                    );
                }
                if state == ElementState::Pressed {
                    context
                        .platform
                        .submit(PlatformEffect::ScreenNarrationInput {
                            target: ScreenNarrationTarget::CommandConfirmation,
                            input: ScreenNarrationInput::MouseAction,
                        });
                }
                self.request_cursor_capture(
                    !context.input.wants_cursor() && !context.input.command_entry_is_active(),
                    context.platform,
                );
            }
            Some(Screen::Disconnected(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_disconnected_screen_mouse_input(
                    button,
                    state,
                    disconnected_screen_target,
                );
                collect_disconnected_screen_platform_effects(context.input, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Disconnected,
                        input: ScreenNarrationInput::MouseAction,
                    });
            }
            Some(Screen::ChatRestrictions(_)) => {
                self.request_cursor_capture(false, context.platform);
                if matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
                    self.apply_chat_hit_target(chat_target, context);
                }
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Chat,
                        input: ScreenNarrationInput::MouseAction,
                    });
            }
            _ if self.code_of_conduct_overlay.is_visible(context.world) => {
                let overlay_input = self.code_of_conduct_overlay.handle_mouse_input(
                    context.world,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                if let Some(request) = overlay_input.request {
                    self.apply_code_of_conduct_control(request, context);
                }
                self.request_cursor_capture(false, context.platform);
            }
            Some(Screen::GameModeSwitcher(_)) => {
                self.request_cursor_capture(false, context.platform);
            }
            Some(Screen::DebugOptions(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .input
                    .handle_debug_options_screen_mouse_input_with_text_measurer(
                        button,
                        state,
                        self.cursor_position,
                        self.gui_surface_size(),
                        context.input.show_only_reduced_info(context.world),
                        &self.text_measurements,
                    );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::DebugOptions,
                        input: ScreenNarrationInput::MouseAction,
                    });
            }
            Some(Screen::Pause(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_debug_pause_screen_mouse_input(
                    context.net_counters,
                    context.world,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                let queued_effects = RefCell::new(Vec::new());
                for request in context.input.take_pause_screen_link_requests() {
                    let url = match request {
                        PauseScreenLinkRequest::SendFeedback { url }
                        | PauseScreenLinkRequest::ReportBugs { url } => url,
                    };
                    context.input.open_chat_url(url.into());
                }
                collect_dialog_open_uri_effects(context.input, &queued_effects);
                flush_platform_effects(&queued_effects, context.platform);
                if context.input.dialog_screen_is_open() {
                    context.input.apply_screen_requests(
                        context.world,
                        context.net_counters,
                        context.net_commands,
                    );
                }
                if context.input.pause_screen_disconnect_requested() {
                    self.request_local_disconnect(context.net_commands);
                }
                if context.input.social_interactions_runtime().is_some() {
                    context
                        .platform
                        .submit(PlatformEffect::ScreenNarrationInput {
                            target: ScreenNarrationTarget::SocialInteractions,
                            input: ScreenNarrationInput::MouseAction,
                        });
                } else {
                    context
                        .platform
                        .submit(PlatformEffect::ScreenNarrationInput {
                            target: ScreenNarrationTarget::Pause,
                            input: ScreenNarrationInput::MouseAction,
                        });
                }
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::Death(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_death_screen_mouse_input(
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    death_screen_target,
                );
                collect_death_screen_platform_effects(context.input, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Death,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if context.input.death_screen().is_none() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::GameEvent(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_game_event_screen_mouse_input(
                    button,
                    state,
                    game_event_target,
                );
                collect_game_event_screen_platform_effects(context.input, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::GameEvent,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if context.input.game_event_screen_runtime().is_none() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::Social(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .input
                    .refresh_social_interactions_scroll_bounds(self.gui_surface_size());
                context.input.handle_social_interactions_mouse_input(
                    context.world,
                    button,
                    state,
                    social_interactions_target,
                );
                collect_social_interactions_platform_effects(context.input, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::SocialInteractions,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::GameRules(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_game_rules_screen_mouse_input(
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::Stats(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Stats,
                        input: ScreenNarrationInput::MouseAction,
                    });
                context.input.handle_stats_screen_mouse_input(
                    context.net_counters,
                    context.world,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(
                Screen::Dialog(_)
                | Screen::DialogWarning(_)
                | Screen::DialogLink(_)
                | Screen::DialogWaiting(_),
            ) => {
                self.request_cursor_capture(false, context.platform);
                let double_click = context.input.dialog_mouse_click_is_double(
                    button,
                    state,
                    context.input.dialog_screen_revision(),
                );
                let queued_effects = RefCell::new(Vec::new());
                let mut clipboard = SimulationClipboard {
                    read: None,
                    effects: &queued_effects,
                };
                let shift_down = context.input.shift_down();
                handle_dialog_mouse_input_with_modifiers_and_measurer(
                    context.input,
                    context.net_counters,
                    context.world,
                    context.net_commands,
                    Some(&mut clipboard),
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    shift_down,
                    double_click,
                    &self.text_measurements,
                );
                collect_dialog_open_uri_effects(context.input, &queued_effects);
                flush_platform_effects(&queued_effects, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Dialog,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::StructureBlock(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_structure_block_editor_mouse_input(
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::StructureBlock,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::JigsawBlock(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_jigsaw_block_editor_mouse_input(
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::JigsawBlock,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::TestBlock(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_test_block_editor_mouse_input(
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::TestInstanceBlock(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_test_instance_block_editor_mouse_input(
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::CommandEntry(_)) => {
                self.request_cursor_capture(false, context.platform);
                let is_command_block = context.input.command_block_editor_hud_state().is_some();
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: if is_command_block {
                            ScreenNarrationTarget::CommandBlock
                        } else {
                            ScreenNarrationTarget::Chat
                        },
                        input: ScreenNarrationInput::MouseAction,
                    });
                let revision = context.input.chat_input_screen_revision();
                let now = bbb_client_clock::now();
                if context.input.handle_command_suggestion_mouse_input(
                    context.world,
                    button,
                    state,
                    self.cursor_position,
                    &self.text_measurements,
                ) {
                    if let Some(revision) = revision {
                        context
                            .input
                            .note_chat_input_click_at(revision, button, now);
                    }
                } else if context.input.handle_command_block_editor_mouse_input(
                    context.world,
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                ) {
                } else if matches!((button, state), (MouseButton::Left, ElementState::Pressed))
                    && self.apply_chat_hit_target(chat_target, context)
                {
                    if let Some(revision) = revision {
                        context
                            .input
                            .note_chat_input_click_at(revision, button, now);
                        context.input.save_chat_input_initial(revision);
                    }
                    context.input.refresh_chat_command_suggestions(
                        context.net_counters,
                        context.world,
                        context.net_commands,
                    );
                } else {
                    context.input.handle_chat_input_mouse_button_at(
                        button,
                        state,
                        self.cursor_position,
                        self.gui_surface_size(),
                        &self.text_measurements,
                        now,
                    );
                }
            }
            Some(Screen::Sign(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_sign_editor_mouse_input(
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Sign,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if !context.input.wants_cursor() {
                    self.request_cursor_capture(true, context.platform);
                }
            }
            Some(Screen::Book(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Book,
                        input: ScreenNarrationInput::MouseAction,
                    });
                if matches!((button, state), (MouseButton::Left, ElementState::Pressed))
                    && self.apply_book_hit_target(book_target, context)
                {
                } else {
                    context.input.handle_book_screen_mouse_input(
                        context.net_counters,
                        context.world,
                        context.net_commands,
                        button,
                        state,
                        self.cursor_position,
                        self.gui_surface_size(),
                        &self.text_measurements,
                    );
                }
            }
            Some(Screen::Advancements(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_advancements_screen_mouse_input(
                    context.net_counters,
                    context.world,
                    context.net_commands,
                    button,
                    state,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
            }
            Some(Screen::Container(_) | Screen::Creative(_)) => {
                if context.world.lectern_book_view().is_some()
                    && matches!((button, state), (MouseButton::Left, ElementState::Pressed))
                    && self.apply_book_hit_target(book_target, context)
                {
                    context
                        .platform
                        .submit(PlatformEffect::ScreenNarrationInput {
                            target: ScreenNarrationTarget::Book,
                            input: ScreenNarrationInput::MouseAction,
                        });
                } else {
                    self.inventory_mouse_button_events =
                        self.inventory_mouse_button_events.saturating_add(1);
                    if matches!((button, state), (MouseButton::Left, ElementState::Pressed))
                        && recipe_book_button_at_position(
                            context.world,
                            self.cursor_position,
                            self.gui_surface_size(),
                        )
                        .is_some()
                    {
                        self.inventory_recipe_book_button_hit_events = self
                            .inventory_recipe_book_button_hit_events
                            .saturating_add(1);
                    }
                    self.request_cursor_capture(false, context.platform);
                    let target =
                        if matches!(context.input.current_screen(), Some(Screen::Container(_)))
                            && context.world.lectern_book_view().is_some()
                        {
                            ScreenNarrationTarget::Book
                        } else if recipe_book_screen_is_open(context.input, context.world) {
                            ScreenNarrationTarget::RecipeBook
                        } else {
                            ScreenNarrationTarget::Inventory
                        };
                    context
                        .platform
                        .submit(PlatformEffect::ScreenNarrationInput {
                            target,
                            input: ScreenNarrationInput::MouseAction,
                        });
                    let consumed = handle_inventory_mouse_input_with_item_runtime(
                        context.input,
                        context.world,
                        context.net_counters,
                        context.net_commands,
                        context.item_runtime.as_ref(),
                        button,
                        state,
                        self.cursor_position,
                        self.gui_surface_size(),
                    );
                    if consumed {
                        self.inventory_mouse_button_consumed_events = self
                            .inventory_mouse_button_consumed_events
                            .saturating_add(1);
                    }
                    flush_local_interaction_sound_actions(
                        context.world,
                        context.level_random,
                        context.audio_events,
                    );
                }
            }
            None => {
                if context.world.level_load().screen_is_up() {
                    self.request_cursor_capture(false, context.platform);
                    return;
                }
                if matches!(state, ElementState::Pressed) && !self.cursor_captured {
                    if context.input.wants_cursor() {
                        self.request_cursor_capture(false, context.platform);
                    } else {
                        self.request_cursor_capture(true, context.platform);
                    }
                    return;
                }
                if !self.cursor_captured {
                    return;
                }
                handle_mouse_input_at_partial_tick_with_effects_and_presentation(
                    context.input,
                    context.first_person_hand_presentation,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                    button,
                    state,
                    context.entity_partial_tick,
                    context.level_random,
                    context.audio_events,
                );
            }
        }
        // Centralize cursor recapture/release on Screen transitions, matching
        // Vanilla `Minecraft.setScreen` (`Minecraft.java:1146-1158`).
        let screen_is_open = context.input.current_screen().is_some()
            || self.code_of_conduct_overlay.is_visible(context.world);
        if screen_was_open
            && !screen_is_open
            && !self.net_disconnect_requested
            && !self.shutdown_requested
        {
            self.request_cursor_capture(true, context.platform);
        } else if !screen_was_open && screen_is_open {
            self.request_cursor_capture(false, context.platform);
        }
    }

    pub(super) fn apply_chat_hit_target(
        &mut self,
        target: Option<HudChatHitTarget>,
        context: &mut SimulationInputContext<'_, '_>,
    ) -> bool {
        let Some(target) = target else {
            return false;
        };
        if !chat_hit_target_is_current(
            context.input,
            context.world,
            context.item_runtime.as_ref(),
            target,
        ) {
            return false;
        }
        match target.kind {
            HudChatHitTargetKind::Queue => {
                if context.world.client_chat().delayed_messages.is_empty() {
                    return false;
                }
                if let Some(acknowledgement) = context
                    .world
                    .accept_next_delayed_chat_message(crate::runtime::monotonic_millis())
                {
                    crate::runtime::queue_chat_acknowledgement(
                        context.net_commands,
                        context.net_counters,
                        acknowledgement,
                    );
                }
                true
            }
            HudChatHitTargetKind::Restrictions => {
                if context.world.chat_presentation_config().visibility
                    == bbb_client_world::ChatVisibilityState::Full
                {
                    return false;
                }
                context.input.open_chat_restrictions_screen()
            }
            HudChatHitTargetKind::RestrictionsDone => {
                context.input.close_chat_restrictions_screen()
            }
            HudChatHitTargetKind::Message {
                display_sequence,
                target_index,
                click_target_index,
            } => {
                // `ChatScreen.mouseClicked` only scans insertions while shift is
                // held; otherwise the hit style must carry the click event.
                let with_insertions = context.input.shift_down();
                let target_index = if with_insertions {
                    target_index
                } else {
                    let Some(click_target_index) = click_target_index else {
                        return false;
                    };
                    click_target_index
                };
                let Some(interaction) = crate::runtime::hud_chat_component_interaction(
                    context.world,
                    context.item_runtime.as_ref(),
                    display_sequence,
                    target_index,
                ) else {
                    return false;
                };
                apply_chat_component_interaction(
                    interaction,
                    with_insertions,
                    context.input,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                    context.platform,
                    &self.text_measurements,
                )
            }
        }
    }

    pub(super) fn apply_book_hit_target(
        &mut self,
        target: Option<HudBookHitTarget>,
        context: &mut SimulationInputContext<'_, '_>,
    ) -> bool {
        let Some(target) = target else {
            return false;
        };
        let Some(event) = crate::runtime::hud_book_component_click_event(
            context.world,
            context.input,
            context.item_runtime.as_ref(),
            target,
        ) else {
            return false;
        };
        apply_book_component_click_event(
            target.mode,
            event,
            context.input,
            context.world,
            context.net_counters,
            context.net_commands,
            context.platform,
            &self.text_measurements,
        )
    }

    pub(super) fn apply_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        let delta = context.input.scale_scroll_delta(delta);
        match context.input.current_screen() {
            Some(Screen::CommandConfirmation(_)) => {}
            _ if self.code_of_conduct_overlay.is_visible(context.world) => {
                self.code_of_conduct_overlay.handle_mouse_wheel(
                    context.world,
                    delta,
                    self.cursor_position,
                    self.gui_surface_size(),
                    &self.text_measurements,
                );
                self.request_cursor_capture(false, context.platform);
            }
            Some(Screen::DebugOptions(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .input
                    .handle_debug_options_screen_mouse_wheel(delta, self.gui_surface_size());
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::DebugOptions,
                        input: ScreenNarrationInput::MouseAction,
                    });
            }
            Some(Screen::Social(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .input
                    .refresh_social_interactions_scroll_bounds(self.gui_surface_size());
                context
                    .input
                    .handle_social_interactions_mouse_wheel(context.world, delta);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::SocialInteractions,
                        input: ScreenNarrationInput::MouseAction,
                    });
            }
            Some(
                Screen::Dialog(_)
                | Screen::DialogWarning(_)
                | Screen::DialogLink(_)
                | Screen::DialogWaiting(_),
            ) => {
                self.request_cursor_capture(false, context.platform);
                handle_dialog_mouse_wheel(
                    context.input,
                    context.world,
                    delta,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Dialog,
                        input: ScreenNarrationInput::MouseAction,
                    });
            }
            Some(Screen::CommandEntry(_)) => {
                self.request_cursor_capture(false, context.platform);
                if context.input.handle_command_suggestion_mouse_wheel(delta) {
                    return;
                }
                if context.input.command_block_editor_hud_state().is_some() {
                    return;
                }
                let scroll = crate::input::scroll_delta_lines(delta).1.clamp(-1.0, 1.0);
                let scroll = if context.input.shift_down() {
                    scroll
                } else {
                    scroll * 7.0
                } as i32;
                context.input.scroll_chat_lines(scroll);
            }
            Some(Screen::GameRules(_)) => {
                self.request_cursor_capture(false, context.platform);
                context.input.handle_game_rules_screen_mouse_wheel(delta);
            }
            Some(Screen::Stats(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target: ScreenNarrationTarget::Stats,
                        input: ScreenNarrationInput::MouseAction,
                    });
                context.input.handle_stats_screen_mouse_wheel(delta);
            }
            Some(Screen::Advancements(_)) => {
                self.request_cursor_capture(false, context.platform);
                context
                    .input
                    .handle_advancements_screen_mouse_wheel(context.world, delta);
            }
            Some(Screen::Container(_) | Screen::Creative(_)) => {
                self.request_cursor_capture(false, context.platform);
                let target = if recipe_book_screen_is_open(context.input, context.world) {
                    ScreenNarrationTarget::RecipeBook
                } else {
                    ScreenNarrationTarget::Inventory
                };
                context
                    .platform
                    .submit(PlatformEffect::ScreenNarrationInput {
                        target,
                        input: ScreenNarrationInput::MouseAction,
                    });
                handle_inventory_mouse_wheel(
                    context.input,
                    context.world,
                    context.net_counters,
                    context.net_commands,
                    delta,
                    self.cursor_position,
                    self.gui_surface_size(),
                );
            }
            Some(
                Screen::Death(_)
                | Screen::GameEvent(_)
                | Screen::Sign(_)
                | Screen::Book(_)
                | Screen::Disconnected(_)
                | Screen::ChatRestrictions(_)
                | Screen::Pause(_)
                | Screen::GameModeSwitcher(_)
                | Screen::StructureBlock(_)
                | Screen::JigsawBlock(_)
                | Screen::TestBlock(_)
                | Screen::TestInstanceBlock(_),
            ) => {
                self.request_cursor_capture(false, context.platform);
            }
            None => {
                if self.cursor_captured {
                    handle_mouse_wheel(
                        context.input,
                        context.world,
                        context.net_counters,
                        context.net_commands,
                        delta,
                    );
                }
            }
        }
    }

    pub(super) fn apply_mouse_motion(
        &mut self,
        delta: (f64, f64),
        context: &mut SimulationInputContext<'_, '_>,
    ) {
        if self.code_of_conduct_overlay.is_visible(context.world)
            || context.input.wants_cursor()
            || context.input.command_entry_is_active()
        {
            self.request_cursor_capture(false, context.platform);
            return;
        }
        if self.cursor_captured {
            handle_mouse_motion(context.input, delta);
        }
    }
}
