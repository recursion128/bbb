use super::*;
use bbb_client_render::{HudBookMode, HudBookScreen};

pub(super) fn chat_screen_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let status = input.chat_screen_status(world.chat_presentation_config().visibility)?;
    if status.screen == crate::control::ChatScreenControlKind::Restrictions {
        let screen = hud_chat_restrictions_screen(input, world, item_runtime, None)?;
        let mut frame = ScreenNarrationFrame::default();
        frame.add(NarratedElementKind::Title, 0, screen.title);
        frame.add(NarratedElementKind::Hint, 0, screen.reason.text);
        frame.add(
            NarratedElementKind::Hint,
            1,
            screen
                .permissions
                .iter()
                .map(|permission| permission.text.as_str())
                .collect::<Vec<_>>()
                .join(". "),
        );
        add_game_event_button_narration(&mut frame, item_runtime, &screen.done, 1, 1);
        return Some(frame);
    }

    let mut frame = ScreenNarrationFrame::default();
    frame.add(
        NarratedElementKind::Title,
        0,
        localized_gui_text(item_runtime, "chat_screen.title", "Chat screen"),
    );
    let usage = localized_gui_text(
        item_runtime,
        "chat_screen.usage",
        "Input message and press Enter to send",
    );
    let usage = if status.can_send_messages && status.can_send_commands {
        usage
    } else {
        [
            usage,
            localized_gui_text(
                item_runtime,
                "chat_screen.restricted.narration",
                "Chat is restricted. Go to the Restrictions screen in World Options.",
            ),
        ]
        .join(". ")
    };
    frame.add(NarratedElementKind::Usage, 0, usage);
    if !status.input_text.is_empty() {
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "chat_screen.message",
                "Message to send: %1$s",
                &[&status.input_text],
            ),
        );
    }
    let edit_box_label = localized_gui_text(item_runtime, "chat.editBox", "chat");
    let mut edit_box = localized_gui_text_with_args(
        item_runtime,
        "gui.narrate.editBox",
        "%1$s edit box: %2$s",
        &[&edit_box_label, &status.input_text],
    );
    if let Some(suggestion) = input.command_suggestion_narration() {
        edit_box.push_str(". ");
        edit_box.push_str(&localized_command_suggestion_narration(
            item_runtime,
            suggestion,
        ));
    }
    frame.add(NarratedElementKind::Title, 2, edit_box);
    Some(frame)
}

pub(super) fn disconnected_screen_narration_frame(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let screen = hud_disconnected_screen(input, item_runtime, 0.0)?;
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());
    let reason = screen
        .reason_runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    if !reason.is_empty() {
        frame.add(NarratedElementKind::Hint, 0, reason);
    }
    let buttons = [
        screen.report_to_server_label.as_deref().map(|label| {
            (
                bbb_client_render::HudDisconnectedScreenButton::ReportToServer,
                label,
            )
        }),
        screen.open_report_directory_label.as_deref().map(|label| {
            (
                bbb_client_render::HudDisconnectedScreenButton::OpenReportDirectory,
                label,
            )
        }),
        screen.reconnect_label.as_deref().map(|label| {
            (
                bbb_client_render::HudDisconnectedScreenButton::Reconnect,
                label,
            )
        }),
        Some((
            bbb_client_render::HudDisconnectedScreenButton::Back,
            screen.back_label.as_str(),
        )),
    ];
    let visible: Vec<_> = buttons.into_iter().flatten().collect();
    let total = visible.len();
    let (position, label) = visible
        .iter()
        .enumerate()
        .find_map(|(idx, (button, label))| {
            (*button == screen.focused).then_some((idx + 1, *label))
        })?;
    add_game_event_button_narration(&mut frame, item_runtime, label, position, total);
    Some(frame)
}

pub(super) fn death_screen_narration_frame(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let death = input.death_screen()?;
    let (screen, runtime) = (&death.screen, &death.runtime);
    let mut frame = ScreenNarrationFrame::default();
    match runtime {
        DeathScreenRuntime::Main(state) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                localized_gui_text(
                    item_runtime,
                    if screen.hardcore {
                        "deathScreen.title.hardcore"
                    } else {
                        "deathScreen.title"
                    },
                    if screen.hardcore {
                        "Game Over!"
                    } else {
                        "You Died!"
                    },
                ),
            );
            let (label, position) = match state.focus {
                bbb_client_render::HudDeathScreenButton::Respawn => (
                    localized_gui_text(
                        item_runtime,
                        if screen.hardcore {
                            "deathScreen.spectate"
                        } else {
                            "deathScreen.respawn"
                        },
                        if screen.hardcore {
                            "Spectate World"
                        } else {
                            "Respawn"
                        },
                    ),
                    1,
                ),
                bbb_client_render::HudDeathScreenButton::TitleScreen => (
                    localized_gui_text(item_runtime, "deathScreen.titleScreen", "Title Screen"),
                    2,
                ),
                _ => return Some(frame),
            };
            add_game_event_button_narration(&mut frame, item_runtime, &label, position, 2);
        }
        DeathScreenRuntime::QuitConfirmation(state) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                localized_gui_text(
                    item_runtime,
                    "deathScreen.quit.confirm",
                    "Are you sure you want to quit?",
                ),
            );
            let (label, position) = match state.focus {
                bbb_client_render::HudDeathScreenButton::QuitTitleScreen => (
                    localized_gui_text(item_runtime, "deathScreen.titleScreen", "Title Screen"),
                    1,
                ),
                bbb_client_render::HudDeathScreenButton::QuitRespawn => (
                    localized_gui_text(item_runtime, "deathScreen.respawn", "Respawn"),
                    2,
                ),
                _ => return Some(frame),
            };
            add_game_event_button_narration(&mut frame, item_runtime, &label, position, 2);
        }
        DeathScreenRuntime::LinkConfirmation(state) => {
            let confirmation = &state.confirmation;
            frame.add(
                NarratedElementKind::Title,
                0,
                [
                    localized_gui_text(
                        item_runtime,
                        "chat.link.confirmTrusted",
                        "Do you want to open this link or copy it to your clipboard?",
                    ),
                    confirmation.url.to_display_string(),
                ]
                .join(". "),
            );
            let (key, fallback, position) = match state.focus {
                bbb_client_render::HudDeathScreenButton::LinkOpen => {
                    ("chat.link.open", "Open in Browser", 1)
                }
                bbb_client_render::HudDeathScreenButton::LinkCopy => {
                    ("chat.copy", "Copy to Clipboard", 2)
                }
                bbb_client_render::HudDeathScreenButton::LinkCancel => ("gui.cancel", "Cancel", 3),
                _ => return Some(frame),
            };
            let label = localized_gui_text(item_runtime, key, fallback);
            add_game_event_button_narration(&mut frame, item_runtime, &label, position, 3);
        }
    }
    Some(frame)
}

pub(super) fn social_interactions_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let runtime = input.social_interactions_runtime()?;
    let screen = hud_social_interactions_screen(input, world, item_runtime, None, None)?;
    let mut frame = ScreenNarrationFrame::default();
    match screen {
        HudSocialInteractionsScreen::LinkConfirmation(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [screen.title, screen.message].join(". "),
            );
            let label = match runtime.focus() {
                HudSocialInteractionsFocus::LinkOpen => screen.open_label,
                HudSocialInteractionsFocus::LinkCopy => screen.copy_label,
                HudSocialInteractionsFocus::LinkCancel => screen.cancel_label,
                _ => return Some(frame),
            };
            frame.add(NarratedElementKind::Title, 1, label);
            frame.add(
                NarratedElementKind::Usage,
                1,
                localized_gui_text(
                    item_runtime,
                    "narration.button.usage.focused",
                    "Press Enter to activate",
                ),
            );
        }
        HudSocialInteractionsScreen::Main(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [screen.title.clone(), screen.server_label.clone()].join(". "),
            );
            match runtime.focus() {
                HudSocialInteractionsFocus::Search => {
                    let search = if screen.search.text.is_empty() {
                        screen.search_hint
                    } else {
                        screen.search.text
                    };
                    frame.add(NarratedElementKind::Title, 1, search);
                    if let Some(empty) = screen.empty_text {
                        frame.add(NarratedElementKind::Hint, 1, empty);
                    }
                    frame.add(
                        NarratedElementKind::Usage,
                        1,
                        localized_gui_text(
                            item_runtime,
                            "narration.edit_box.usage",
                            "Type to search",
                        ),
                    );
                }
                HudSocialInteractionsFocus::Tab(page) => {
                    let index = match page {
                        HudSocialInteractionsPage::All => 0,
                        HudSocialInteractionsPage::Hidden => 1,
                        HudSocialInteractionsPage::Blocked => 2,
                    };
                    frame.add(
                        NarratedElementKind::Title,
                        1,
                        screen.tab_labels[index].clone(),
                    );
                    frame.add(
                        NarratedElementKind::Position,
                        1,
                        localized_gui_text_with_args(
                            item_runtime,
                            "narrator.position.object_list",
                            "%1$s out of %2$s",
                            &[&(index + 1).to_string(), "3"],
                        ),
                    );
                }
                HudSocialInteractionsFocus::Player { profile_id, action } => {
                    let Some((index, player)) = screen
                        .rows
                        .iter()
                        .enumerate()
                        .find(|(_, player)| player.profile_id == profile_id)
                    else {
                        return Some(frame);
                    };
                    let action_text = localized_gui_text_with_args(
                        item_runtime,
                        match action {
                            HudSocialInteractionsPlayerAction::Hide => {
                                "gui.socialInteractions.narration.hide"
                            }
                            HudSocialInteractionsPlayerAction::Show => {
                                "gui.socialInteractions.narration.show"
                            }
                        },
                        match action {
                            HudSocialInteractionsPlayerAction::Hide => "Hide messages from %s",
                            HudSocialInteractionsPlayerAction::Show => "Show messages from %s",
                        },
                        &[&player.name],
                    );
                    frame.add(
                        NarratedElementKind::Title,
                        1,
                        [
                            Some(player.name.clone()),
                            player.status.clone(),
                            Some(action_text),
                        ]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(", "),
                    );
                    frame.add(
                        NarratedElementKind::Position,
                        1,
                        localized_gui_text_with_args(
                            item_runtime,
                            "narrator.position.object_list",
                            "%1$s out of %2$s",
                            &[&(index + 1).to_string(), &screen.rows.len().to_string()],
                        ),
                    );
                    frame.add(
                        NarratedElementKind::Usage,
                        1,
                        localized_gui_text(
                            item_runtime,
                            "narration.button.usage.focused",
                            "Press Enter to activate",
                        ),
                    );
                }
                HudSocialInteractionsFocus::BlockingHelp => {
                    frame.add(NarratedElementKind::Title, 1, screen.blocking_help_label);
                }
                HudSocialInteractionsFocus::Done => {
                    frame.add(NarratedElementKind::Title, 1, screen.done_label);
                }
                HudSocialInteractionsFocus::LinkOpen
                | HudSocialInteractionsFocus::LinkCopy
                | HudSocialInteractionsFocus::LinkCancel => {}
            }
        }
    }
    Some(frame)
}

pub(super) fn game_event_screen_narration_frame(
    screen: &HudGameEventScreen,
    resources: &crate::win_screen_resources::WinScreenResources,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    match screen {
        HudGameEventScreen::Win(_) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                resources.narration_components.join(". "),
            );
            frame.add(
                NarratedElementKind::Usage,
                0,
                localized_gui_text(
                    item_runtime,
                    "narrator.screen.usage",
                    "Use mouse cursor or Tab button to select element",
                ),
            );
        }
        HudGameEventScreen::DemoPopup(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [
                    screen.title.as_str(),
                    screen.controls_message.as_str(),
                    screen.duration_message.as_str(),
                ]
                .join(". "),
            );
            add_game_event_button_narration(
                &mut frame,
                item_runtime,
                match screen.focused {
                    HudGameEventButton::DemoBuy => &screen.buy_label,
                    HudGameEventButton::DemoLater => &screen.later_label,
                    _ => return frame,
                },
                match screen.focused {
                    HudGameEventButton::DemoBuy => 4,
                    HudGameEventButton::DemoLater => 5,
                    _ => return frame,
                },
                5,
            );
        }
        HudGameEventScreen::DemoLinkConfirmation(screen) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [screen.title.as_str(), screen.message.as_str()].join(". "),
            );
            let (label, position) = match screen.focused {
                HudGameEventButton::LinkOpen => (&screen.open_label, 3),
                HudGameEventButton::LinkCopy => (&screen.copy_label, 4),
                HudGameEventButton::LinkCancel => (&screen.cancel_label, 5),
                _ => return frame,
            };
            add_game_event_button_narration(&mut frame, item_runtime, label, position, 5);
        }
    }
    frame
}

pub(super) fn pause_screen_narration_frame(
    screen: &HudPauseScreen,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());
    if !screen.show_pause_menu {
        return frame;
    }
    let hovered_label = if screen.return_to_game_hovered {
        Some(localized_gui_text(
            item_runtime,
            "menu.returnToGame",
            "Back to Game",
        ))
    } else if screen.advancements_hovered {
        Some(localized_gui_text(
            item_runtime,
            "gui.advancements",
            "Advancements",
        ))
    } else if screen.stats_hovered {
        Some(localized_gui_text(item_runtime, "gui.stats", "Statistics"))
    } else if screen.send_feedback_hovered {
        Some(localized_gui_text(
            item_runtime,
            "menu.sendFeedback",
            "Send Feedback",
        ))
    } else if screen.report_bugs_hovered {
        Some(localized_gui_text(
            item_runtime,
            "menu.reportBugs",
            "Report Bugs",
        ))
    } else if screen.custom_dialog_hovered {
        screen.custom_dialog_label.clone()
    } else if screen.feedback_back_hovered {
        Some(localized_gui_text(item_runtime, "gui.back", "Back"))
    } else if screen.options_hovered {
        Some(localized_gui_text(
            item_runtime,
            "menu.options",
            "Options...",
        ))
    } else if screen.disconnect_hovered {
        Some(localized_gui_text(
            item_runtime,
            "menu.disconnect",
            "Disconnect",
        ))
    } else {
        None
    };
    if let Some(label) = hovered_label {
        add_hovered_button_narration(&mut frame, item_runtime, &label);
        if screen.custom_dialog_hovered {
            if let Some(tooltip) = screen.custom_dialog_tooltip.as_ref() {
                frame.add(NarratedElementKind::Hint, 1, tooltip.text.clone());
            }
        }
    } else if let Some(tooltip) = screen.options_tooltip.as_ref() {
        frame.add(NarratedElementKind::Hint, 1, tooltip.text.clone());
    } else {
        frame.add(
            NarratedElementKind::Usage,
            0,
            localized_gui_text(
                item_runtime,
                "narrator.screen.usage",
                "Use mouse cursor or Tab button to select element",
            ),
        );
    }
    frame
}

pub(super) fn container_inventory_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    screen: &HudInventoryScreen,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    let title = if world.local_inventory_is_open() {
        localized_gui_text(item_runtime, "container.inventory", "Inventory")
    } else if let Some(container) = world.inventory().open_container.as_ref() {
        let resolved = container
            .title
            .as_ref()
            .map(RichTextComponent::text)
            .unwrap_or("");
        if resolved.is_empty() {
            localized_gui_text(item_runtime, "container.inventory", "Container")
        } else {
            resolved.to_string()
        }
    } else {
        localized_gui_text(item_runtime, "container.inventory", "Inventory")
    };
    frame.add(NarratedElementKind::Title, 0, title);
    if let Some(tooltip) = screen.tooltip.as_ref() {
        let hovered_item = tooltip
            .lines
            .first()
            .map(|line| {
                if line.runs.is_empty() {
                    line.text.clone()
                } else {
                    line.runs
                        .iter()
                        .map(|run| run.visual.text.as_str())
                        .collect::<String>()
                }
            })
            .unwrap_or_default();
        if !hovered_item.is_empty() {
            frame.add(NarratedElementKind::Title, 1, hovered_item);
        }
    } else if let Some(rename_field) = input.anvil_rename_field() {
        let text = rename_field.text();
        if !text.is_empty() {
            let label = localized_gui_text(item_runtime, "container.repair", "Repair & Name");
            frame.add(
                NarratedElementKind::Title,
                1,
                localized_gui_text_with_args(
                    item_runtime,
                    "gui.narrate.editBox",
                    "%1$s edit box: %2$s",
                    &[&label, &text],
                ),
            );
        }
    }
    frame
}

pub(super) fn book_screen_narration_frame(
    screen: &HudBookScreen,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    let title = match screen.mode {
        HudBookMode::View | HudBookMode::Lectern => [
            localized_gui_text(item_runtime, "book.view.title", "Book"),
            screen.page_indicator.clone(),
        ]
        .join(". "),
        HudBookMode::Edit => [
            localized_gui_text(item_runtime, "book.edit.title", "Edit Book"),
            screen.page_indicator.clone(),
        ]
        .join(". "),
        HudBookMode::Sign => [screen.title.as_str(), screen.author.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(". "),
    };
    frame.add(NarratedElementKind::Title, 0, title);
    let body = screen
        .runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    let trimmed = body.trim();
    if !trimmed.is_empty() {
        frame.add(NarratedElementKind::Hint, 0, trimmed);
    }
    if screen.mode == HudBookMode::Sign && !screen.warning.is_empty() {
        frame.add(NarratedElementKind::Hint, 1, screen.warning.clone());
    }
    frame
}

pub(super) fn sign_editor_narration_frame(
    screen: &HudSignEditorScreen,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());
    let line_number = screen.line.saturating_add(1);
    let total_lines = screen.lines.len().max(1);
    frame.add(
        NarratedElementKind::Position,
        0,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.object_list",
            "%1$s out of %2$s",
            &[&line_number.to_string(), &total_lines.to_string()],
        ),
    );
    if screen.done_hovered {
        add_hovered_button_narration(&mut frame, item_runtime, &screen.done);
    } else if let Some(line_text) = screen.lines.get(screen.line) {
        let line_label = format!("Line {line_number}");
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%1$s edit box: %2$s",
                &[&line_label, line_text.as_str()],
            ),
        );
    }
    frame
}

pub(super) fn command_block_editor_narration_frame(
    input: &ClientInputState,
    screen: &HudCommandBlockEditorScreen,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());
    let hovered_label = if screen.mode_hovered && !screen.mode.is_empty() {
        Some(screen.mode.as_str())
    } else if screen.conditional_hovered && !screen.conditional.is_empty() {
        Some(screen.conditional.as_str())
    } else if screen.automatic_hovered && !screen.automatic.is_empty() {
        Some(screen.automatic.as_str())
    } else if screen.track_output_hovered {
        Some(screen.previous_output_label.as_str())
    } else if screen.done_hovered {
        Some(screen.done.as_str())
    } else if screen.cancel_hovered {
        Some(screen.cancel.as_str())
    } else {
        None
    };
    if let Some(label) = hovered_label {
        add_hovered_button_narration(&mut frame, item_runtime, label);
    } else {
        let command_text = input
            .command_block_editor_command_text()
            .unwrap_or_default();
        let mut edit_box = localized_gui_text_with_args(
            item_runtime,
            "gui.narrate.editBox",
            "%1$s edit box: %2$s",
            &[&screen.command_label, &command_text],
        );
        if let Some(suggestion) = input.command_suggestion_narration() {
            edit_box.push_str(". ");
            edit_box.push_str(&localized_command_suggestion_narration(
                item_runtime,
                suggestion,
            ));
        }
        frame.add(NarratedElementKind::Title, 1, edit_box);
    }
    let summary = [
        screen.mode.as_str(),
        screen.conditional.as_str(),
        screen.automatic.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
    if !summary.is_empty() {
        frame.add(NarratedElementKind::Hint, 0, summary);
    }
    frame
}

pub(super) fn structure_block_editor_narration_frame(
    screen: &HudStructureBlockEditorScreen,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(
        NarratedElementKind::Title,
        0,
        [screen.title.as_str(), screen.mode_value.as_str()].join(". "),
    );
    if !screen.mode_description.is_empty() {
        frame.add(
            NarratedElementKind::Hint,
            0,
            screen.mode_description.clone(),
        );
    }
    let hovered_label = if screen.mode_hovered {
        Some(screen.mode_value.as_str())
    } else if screen.mirror_hovered {
        Some(screen.mirror_value.as_str())
    } else if screen.rotation_hovered.iter().any(|hovered| *hovered) {
        Some(screen.rotation.as_str())
    } else if screen.include_entities_hovered {
        Some(screen.include_entities_label.as_str())
    } else if screen.strict_hovered {
        Some(screen.strict_label.as_str())
    } else if screen.option_hovered {
        Some(screen.option_label.as_str())
    } else if screen.action_hovered {
        Some(screen.action.as_str())
    } else if screen.detect_hovered {
        Some(screen.detect.as_str())
    } else if screen.done_hovered {
        Some(screen.done.as_str())
    } else if screen.cancel_hovered {
        Some(screen.cancel.as_str())
    } else {
        None
    };
    if let Some(label) = hovered_label {
        add_hovered_button_narration(&mut frame, item_runtime, label);
    } else if let Some((index, field)) = screen
        .fields
        .iter()
        .enumerate()
        .find(|(_, field)| field.focused)
    {
        let label = match index {
            0 => screen.name_label.as_str(),
            1..=3 => screen.position_label.as_str(),
            4..=6 => screen.size_label.as_str(),
            7 | 8 => screen.integrity_label.as_str(),
            _ => screen.data_label.as_str(),
        };
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%1$s edit box: %2$s",
                &[label, field.text.as_str()],
            ),
        );
    }
    frame
}

pub(super) fn jigsaw_block_editor_narration_frame(
    screen: &HudJigsawBlockEditorScreen,
    item_runtime: Option<&NativeItemRuntime>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(
        NarratedElementKind::Title,
        0,
        localized_gui_text(item_runtime, "block.minecraft.jigsaw", "Jigsaw Block"),
    );
    let hovered_label = if screen.joint_hovered {
        Some(screen.joint.as_str())
    } else if screen.levels_hovered {
        Some(screen.levels.as_str())
    } else if screen.keep_jigsaws_hovered {
        Some(screen.keep_jigsaws.as_str())
    } else if screen.generate_hovered {
        Some(screen.generate.as_str())
    } else if screen.done_hovered {
        Some(screen.done.as_str())
    } else if screen.cancel_hovered {
        Some(screen.cancel.as_str())
    } else {
        None
    };
    if let Some(label) = hovered_label {
        add_hovered_button_narration(&mut frame, item_runtime, label);
    } else if let Some((index, field)) = screen
        .fields
        .iter()
        .enumerate()
        .find(|(_, field)| field.focused)
    {
        let label = screen
            .labels
            .get(index)
            .map(String::as_str)
            .unwrap_or("Field");
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%1$s edit box: %2$s",
                &[label, field.text.as_str()],
            ),
        );
    }
    if let Some(tooltip) = screen.tooltip.as_ref() {
        frame.add(NarratedElementKind::Hint, 0, tooltip.text.clone());
    } else {
        frame.add(
            NarratedElementKind::Hint,
            0,
            [screen.levels.as_str(), screen.keep_jigsaws.as_str()].join(". "),
        );
    }
    frame
}

pub(super) fn dialog_screen_narration_frame(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<ScreenNarrationFrame> {
    let mut frame = ScreenNarrationFrame::default();
    match input.current_screen()? {
        crate::input::Screen::DialogLink(link) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                [
                    localized_gui_text(
                        item_runtime,
                        "chat.link.confirm",
                        "Are you sure you want to open the following website?",
                    ),
                    link.url.to_display_string(),
                ]
                .join(". "),
            );
            frame.add(
                NarratedElementKind::Hint,
                0,
                localized_gui_text(
                    item_runtime,
                    "chat.link.warning",
                    "Never open links from people that you don't trust!",
                ),
            );
            if let Some((label, position)) = match link.focused_button {
                Some(0) => Some((localized_gui_text(item_runtime, "gui.yes", "Yes"), 1)),
                Some(1) => Some((
                    localized_gui_text(item_runtime, "chat.copy", "Copy to Clipboard"),
                    2,
                )),
                Some(2) => Some((localized_gui_text(item_runtime, "gui.no", "No"), 3)),
                _ => None,
            } {
                add_game_event_button_narration(&mut frame, item_runtime, &label, position, 3);
            }
            Some(frame)
        }
        crate::input::Screen::DialogWarning(focused) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                localized_gui_text(
                    item_runtime,
                    "menu.custom_screen_info.title",
                    "Custom Server Dialog",
                ),
            );
            if let Some((label, position)) = match focused {
                Some(true) => Some((
                    localized_gui_text(item_runtime, "menu.disconnect", "Disconnect"),
                    1,
                )),
                Some(false) => Some((localized_gui_text(item_runtime, "gui.back", "Back"), 2)),
                None => None,
            } {
                add_game_event_button_narration(&mut frame, item_runtime, &label, position, 2);
            }
            Some(frame)
        }
        crate::input::Screen::DialogWaiting(_) => {
            frame.add(
                NarratedElementKind::Title,
                0,
                localized_gui_text(
                    item_runtime,
                    "gui.waitingForResponse.title",
                    "Waiting for Server Response...",
                ),
            );
            Some(frame)
        }
        crate::input::Screen::Dialog(_) => {
            let dialog = input.current_dialog()?;
            let data = dialog.data.as_ref()?;
            let (common, action_labels) = dialog_common_and_action_labels(data, world);
            frame.add(NarratedElementKind::Title, 0, common.title.text.clone());
            let body_summary = common
                .body_data
                .iter()
                .filter_map(|body| match body {
                    bbb_client_world::DialogBodyState::PlainMessage { message } => {
                        (!message.contents.text.is_empty())
                            .then_some(message.contents.text.as_str())
                    }
                    bbb_client_world::DialogBodyState::Item {
                        description: Some(description),
                        ..
                    } => (!description.contents.text.is_empty())
                        .then_some(description.contents.text.as_str()),
                    bbb_client_world::DialogBodyState::Item {
                        description: None, ..
                    } => None,
                })
                .collect::<Vec<_>>()
                .join(". ");
            if !body_summary.is_empty() {
                frame.add(NarratedElementKind::Hint, 0, body_summary);
            }
            match input.dialog_focus() {
                Some(crate::input::dialog::DialogFocusState::WarningButton) => {
                    let label = localized_gui_text(
                        item_runtime,
                        "menu.custom_screen_info.button_narration",
                        "Custom Server Dialog Info",
                    );
                    add_hovered_button_narration(&mut frame, item_runtime, &label);
                }
                Some(crate::input::dialog::DialogFocusState::Action { index }) => {
                    if let Some(label) = action_labels.get(index) {
                        add_game_event_button_narration(
                            &mut frame,
                            item_runtime,
                            label,
                            index + 1,
                            action_labels.len(),
                        );
                    }
                }
                Some(crate::input::dialog::DialogFocusState::Input { index }) => {
                    if let Some(definition) = common.input_data.get(index) {
                        let current_value = input.current_dialog_input_values().get(index);
                        let (label, value_text) = match (&definition.control, current_value) {
                            (
                                bbb_client_world::DialogInputControlState::Boolean {
                                    label, ..
                                },
                                Some(Some(crate::input::dialog::DialogInputValueState::Boolean {
                                    selected,
                                })),
                            ) => (
                                label.text.as_str(),
                                localized_gui_text(
                                    item_runtime,
                                    if *selected {
                                        "options.on"
                                    } else {
                                        "options.off"
                                    },
                                    if *selected { "On" } else { "Off" },
                                ),
                            ),
                            (
                                bbb_client_world::DialogInputControlState::NumberRange {
                                    label,
                                    range,
                                    ..
                                },
                                Some(Some(
                                    crate::input::dialog::DialogInputValueState::NumberRange {
                                        slider_value_bits,
                                    },
                                )),
                            ) => {
                                let t = f64::from_bits(*slider_value_bits);
                                let computed = f64::from(range.start())
                                    + t * f64::from(range.end() - range.start());
                                (label.text.as_str(), format!("{computed:.2}"))
                            }
                            (
                                bbb_client_world::DialogInputControlState::SingleOption {
                                    label,
                                    options,
                                    ..
                                },
                                Some(Some(
                                    crate::input::dialog::DialogInputValueState::SingleOption {
                                        selected_index,
                                    },
                                )),
                            ) => {
                                let selected_text = options
                                    .get(*selected_index)
                                    .map(|option| {
                                        option.display.as_ref().map_or_else(
                                            || option.id.to_display_string(),
                                            |display| display.text.clone(),
                                        )
                                    })
                                    .unwrap_or_default();
                                (label.text.as_str(), selected_text)
                            }
                            (
                                bbb_client_world::DialogInputControlState::Text { label, .. },
                                Some(Some(crate::input::dialog::DialogInputValueState::Text {
                                    value,
                                })),
                            ) => (label.text.as_str(), value.to_display_string()),
                            _ => ("", String::new()),
                        };
                        if !label.is_empty() || !value_text.is_empty() {
                            frame.add(
                                NarratedElementKind::Title,
                                1,
                                localized_gui_text_with_args(
                                    item_runtime,
                                    "gui.narrate.editBox",
                                    "%1$s edit box: %2$s",
                                    &[label, &value_text],
                                ),
                            );
                        }
                    }
                }
                Some(crate::input::dialog::DialogFocusState::Body { index, .. }) => {
                    if let Some(body) = common.body_data.get(index) {
                        let text = match body {
                            bbb_client_world::DialogBodyState::PlainMessage { message } => {
                                message.contents.text.as_str()
                            }
                            bbb_client_world::DialogBodyState::Item {
                                description: Some(description),
                                ..
                            } => description.contents.text.as_str(),
                            bbb_client_world::DialogBodyState::Item {
                                description: None, ..
                            } => "",
                        };
                        if !text.is_empty() {
                            frame.add(NarratedElementKind::Title, 1, text);
                        }
                    }
                }
                None => {}
            }
            Some(frame)
        }
        _ => None,
    }
}

fn dialog_common_and_action_labels<'a>(
    data: &'a bbb_client_world::DialogDataState,
    world: &'a WorldStore,
) -> (&'a bbb_client_world::CommonDialogDataState, Vec<&'a str>) {
    match data {
        bbb_client_world::DialogDataState::Notice { common, action } => {
            (common, vec![action.label.text.as_str()])
        }
        bbb_client_world::DialogDataState::Confirmation { common, yes, no } => (
            common,
            vec![yes.label.text.as_str(), no.label.text.as_str()],
        ),
        bbb_client_world::DialogDataState::MultiAction {
            common,
            actions,
            exit_action,
            ..
        } => {
            let mut labels: Vec<&'a str> = actions
                .iter()
                .map(|button| button.label.text.as_str())
                .collect();
            if let Some(exit) = exit_action {
                labels.push(exit.label.text.as_str());
            }
            (common, labels)
        }
        bbb_client_world::DialogDataState::DialogList {
            common,
            dialogs,
            exit_action,
            ..
        } => {
            let mut labels: Vec<&'a str> = dialogs
                .entries
                .iter()
                .map(|entry| entry.external_title.text.as_str())
                .collect();
            if let Some(exit) = exit_action {
                labels.push(exit.label.text.as_str());
            }
            (common, labels)
        }
        bbb_client_world::DialogDataState::ServerLinks {
            common,
            exit_action,
            ..
        } => {
            let mut labels: Vec<&'a str> = world
                .server_links()
                .iter()
                .map(|link| match link.known_type.as_deref() {
                    Some("report_bug") => "Report Server Bug",
                    Some("community_guidelines") => "Community Guidelines",
                    Some("support") => "Support",
                    Some("status") => "Status",
                    Some("feedback") => "Feedback",
                    Some("community") => "Community",
                    Some("website") => "Website",
                    Some("forums") => "Forums",
                    Some("news") => "News",
                    Some("announcements") => "Announcements",
                    _ => link.label.as_str(),
                })
                .collect();
            if let Some(exit) = exit_action {
                labels.push(exit.label.text.as_str());
            }
            (common, labels)
        }
    }
}

fn add_hovered_button_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    label: &str,
) {
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%1$s button", &[label]),
    );
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            "narration.button.usage.hovered",
            "Left click to activate",
        ),
    );
}

fn add_game_event_button_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    label: &str,
    position: usize,
    total: usize,
) {
    frame.add(
        NarratedElementKind::Position,
        0,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.screen",
            "Screen element %1$s out of %2$s",
            &[&position.to_string(), &total.to_string()],
        ),
    );
    frame.add(
        NarratedElementKind::Usage,
        0,
        localized_gui_text(
            item_runtime,
            "narration.component_list.usage",
            "Press Tab to navigate to the next element",
        ),
    );
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%1$s button", &[label]),
    );
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            "narration.button.usage.focused",
            "Press Enter to activate",
        ),
    );
}

pub(super) fn localized_gui_text_with_args(
    item_runtime: Option<&NativeItemRuntime>,
    translation_key: &str,
    fallback: &str,
    args: &[&str],
) -> String {
    format_localized_string_arguments(
        &localized_gui_text(item_runtime, translation_key, fallback),
        args,
    )
}

pub(super) fn format_localized_string_arguments(template: &str, args: &[&str]) -> String {
    let bytes = template.as_bytes();
    let mut result = String::with_capacity(template.len());
    let mut offset = 0;
    let mut sequential = 0usize;
    while offset < bytes.len() {
        let Some(relative_percent) = bytes[offset..].iter().position(|byte| *byte == b'%') else {
            result.push_str(&template[offset..]);
            break;
        };
        let percent = offset + relative_percent;
        result.push_str(&template[offset..percent]);
        if bytes.get(percent + 1) == Some(&b'%') {
            result.push('%');
            offset = percent + 2;
            continue;
        }
        if bytes.get(percent + 1) == Some(&b's') {
            if let Some(argument) = args.get(sequential) {
                result.push_str(argument);
            } else {
                result.push_str("%s");
            }
            sequential += 1;
            offset = percent + 2;
            continue;
        }

        let mut cursor = percent + 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        let positional = (cursor > percent + 1
            && bytes.get(cursor) == Some(&b'$')
            && bytes.get(cursor + 1) == Some(&b's'))
        .then(|| {
            template[percent + 1..cursor]
                .parse::<usize>()
                .ok()
                .and_then(|index| index.checked_sub(1))
        })
        .flatten();
        if let Some(index) = positional {
            if let Some(argument) = args.get(index) {
                result.push_str(argument);
            } else {
                result.push_str(&template[percent..cursor + 2]);
            }
            offset = cursor + 2;
        } else {
            result.push('%');
            offset = percent + 1;
        }
    }
    result
}

pub(crate) fn narrate_command_suggestion_change(
    narration: Option<&CommandSuggestionNarrationState>,
    last_narration: &mut Option<CommandSuggestionNarrationState>,
    item_runtime: Option<&NativeItemRuntime>,
    narrator: Option<&mut dyn SystemNarrator>,
) {
    let current = narration.cloned();
    if current == *last_narration {
        return;
    }
    if let (Some(narrator), Some(narration)) = (narrator, current.as_ref()) {
        narrator.say_system_now(&localized_command_suggestion_narration(
            item_runtime,
            narration,
        ));
    }
    *last_narration = current;
}

pub(crate) fn narrate_player_list_visibility_change(
    visible: bool,
    was_visible: &mut bool,
    narrator: Option<&mut dyn SystemNarrator>,
    narration: impl FnOnce() -> String,
) {
    if visible && !*was_visible {
        if let Some(narrator) = narrator {
            narrator.say_system_now(&narration());
        }
    }
    *was_visible = visible;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_render::{
        HudCommandBlockEditorKind, HudCommandBlockEditorScreen, HudComponentTextRun,
        HudInventoryScreen, HudInventoryTooltip, HudInventoryTooltipLine,
        HudJigsawBlockEditorScreen, HudPauseScreen, HudSignEditorKind, HudSignEditorScreen,
        HudStructureBlockEditorScreen, HudStructureBlockMode, HudStyledTextRun, HudTextField,
        SignModelAttachment, SignModelWood,
    };
    use bbb_protocol::packets::OpenScreen;
    use bbb_protocol::RichTextComponent;

    #[test]
    fn pause_screen_narration_covers_title_and_hovered_button() {
        let mut screen = HudPauseScreen {
            title: "Game Menu".to_string(),
            labels: bbb_client_render::HudPauseScreenLabels::default(),
            show_pause_menu: true,
            feedback_subscreen: false,
            return_to_game_hovered: false,
            advancements_hovered: false,
            stats_hovered: true,
            send_feedback_hovered: false,
            report_bugs_hovered: false,
            report_bugs_enabled: true,
            custom_dialog_label: None,
            custom_dialog_hovered: false,
            custom_dialog_tooltip: None,
            feedback_back_hovered: false,
            options_hovered: false,
            options_enabled: false,
            options_tooltip: None,
            disconnect_hovered: false,
            disconnect_enabled: true,
        };
        let text = pause_screen_narration_frame(&screen, None).all_text_for_test();
        assert!(text.contains("Game Menu"));
        assert!(text.contains("Statistics button"));

        screen.stats_hovered = false;
        let idle = pause_screen_narration_frame(&screen, None).all_text_for_test();
        assert!(idle.contains("Game Menu"));
        assert!(idle.contains("Use mouse cursor or Tab button"));
    }

    #[test]
    fn container_inventory_narration_covers_title_and_hovered_item_tooltip() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 3,
            menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID,
            title: RichTextComponent::plain("Treasure Chest"),
        });
        let input = ClientInputState::default();
        let screen = HudInventoryScreen {
            width: 176,
            height: 166,
            background_layers: Vec::new(),
            foreground_layers: Vec::new(),
            fill_layers: Vec::new(),
            slots: Vec::new(),
            floating_items: Vec::new(),
            foreground_items: Vec::new(),
            ghost_items: Vec::new(),
            entity_previews: Vec::new(),
            map_previews: Vec::new(),
            banner_pattern_previews: Vec::new(),
            text_labels: Vec::new(),
            scroll_area: None,
            preedit_overlay: None,
            hovered_slot_id: Some(0),
            tooltip: Some(HudInventoryTooltip {
                slot_id: 0,
                x: 10,
                y: 10,
                lines: vec![HudInventoryTooltipLine {
                    text: "Diamond Sword".into(),
                    tint: [1.0; 4],
                    runs: vec![HudComponentTextRun {
                        visual: HudStyledTextRun::plain("Diamond Sword"),
                        font: None,
                    }],
                }],
                style: None,
            }),
        };
        let text =
            container_inventory_narration_frame(&input, &world, &screen, None).all_text_for_test();
        assert!(text.contains("Treasure Chest"));
        assert!(text.contains("Diamond Sword"));
    }

    #[test]
    fn book_and_sign_editor_narration_include_page_line_and_text() {
        let book = HudBookScreen {
            editor_text: Vec::new(),
            editor_display_start: 0,
            generation: 1,
            page: 0,
            page_count: 2,
            resource_generation: 0,
            resources_loaded: false,
            text_filtering_enabled: false,
            advanced_item_tooltips: false,
            mode: HudBookMode::View,
            runs: vec![HudComponentTextRun {
                visual: HudStyledTextRun::plain("Chapter One"),
                font: None,
            }],
            interactions: Vec::new(),
            cursor: 0,
            selection: 0,
            cursor_visible: false,
            cursor_position: None,
            page_indicator: "Page 1 of 2".to_string(),
            title: String::new(),
            author: String::new(),
            warning: String::new(),
            done: "Done".to_string(),
            take_book: "Take Book".to_string(),
            may_build: false,
            sign: "Sign".to_string(),
            finalize: "Sign and Close".to_string(),
            cancel: "Cancel".to_string(),
            finalize_enabled: false,
            preedit: None,
        };
        let book_text = book_screen_narration_frame(&book, None).all_text_for_test();
        assert!(book_text.contains("Page 1 of 2"));
        assert!(book_text.contains("Chapter One"));

        let sign = HudSignEditorScreen {
            kind: HudSignEditorKind::Standing {
                wood: SignModelWood::Oak,
                attachment: SignModelAttachment::Standing,
            },
            sign_preview: None,
            title: "Edit Sign Message".to_string(),
            lines: [
                "Welcome".to_string(),
                "Home".to_string(),
                String::new(),
                String::new(),
            ],
            line: 1,
            cursor: 4,
            selection: 4,
            cursor_visible: true,
            text_tint: [0.0, 0.0, 0.0, 1.0],
            done: "Done".to_string(),
            done_hovered: false,
            preedit: None,
        };
        let sign_text = sign_editor_narration_frame(&sign, None).all_text_for_test();
        assert!(sign_text.contains("Edit Sign Message"));
        assert!(sign_text.contains("Line 2 edit box: Home"));
        assert!(sign_text.contains("2 out of 4"));
    }

    #[test]
    fn block_editors_and_dialog_narration_cover_title_and_focus() {
        let input = ClientInputState::default();
        let cmd = HudCommandBlockEditorScreen {
            kind: HudCommandBlockEditorKind::Block,
            title: "Set Console Command for Block".to_string(),
            command_label: "Console Command".to_string(),
            previous_output_label: "Previous Output".to_string(),
            previous_output: "-".to_string(),
            mode: "Repeat".to_string(),
            conditional: "Unconditional".to_string(),
            automatic: "Always Active".to_string(),
            track_output: "O".to_string(),
            done: "Done".to_string(),
            cancel: "Cancel".to_string(),
            controls_enabled: true,
            mode_hovered: false,
            conditional_hovered: false,
            automatic_hovered: false,
            track_output_hovered: false,
            done_hovered: true,
            cancel_hovered: false,
        };
        let cmd_text = command_block_editor_narration_frame(&input, &cmd, None).all_text_for_test();
        assert!(cmd_text.contains("Set Console Command for Block"));
        assert!(cmd_text.contains("Done button"));
        assert!(cmd_text.contains("Repeat, Unconditional, Always Active"));

        let mut structure_fields = std::array::from_fn(|_| HudTextField::default());
        structure_fields[0].focused = true;
        structure_fields[0].text = "minecraft:house".to_string();
        let structure = HudStructureBlockEditorScreen {
            title: "Structure Block".to_string(),
            name_label: "Structure Name".to_string(),
            position_label: "Relative Position".to_string(),
            size_label: "Structure Size".to_string(),
            integrity_label: "Structure Integrity and Seed".to_string(),
            data_label: "Custom Data Tag Name".to_string(),
            include_entities_label: "Include Entities".to_string(),
            strict_label: "Strict".to_string(),
            detect_label: "Detect".to_string(),
            option_label: "Show Air".to_string(),
            mode_description: "Save Mode".to_string(),
            fields: structure_fields,
            mode: HudStructureBlockMode::Save,
            mode_value: "Save".to_string(),
            mirror_value: "|".to_string(),
            rotation: "0".to_string(),
            include_entities: "On".to_string(),
            strict: "Off".to_string(),
            option: "Off".to_string(),
            action: "Save".to_string(),
            detect: "Detect".to_string(),
            done: "Done".to_string(),
            cancel: "Cancel".to_string(),
            mode_hovered: false,
            mirror_hovered: false,
            rotation_hovered: [false; 4],
            include_entities_hovered: false,
            strict_hovered: false,
            option_hovered: false,
            action_hovered: false,
            detect_hovered: false,
            done_hovered: false,
            cancel_hovered: false,
        };
        let structure_text =
            structure_block_editor_narration_frame(&structure, None).all_text_for_test();
        assert!(structure_text.contains("Structure Block. Save"));
        assert!(structure_text.contains("Structure Name edit box: minecraft:house"));

        let mut jigsaw_fields = std::array::from_fn(|_| HudTextField::default());
        jigsaw_fields[0].focused = true;
        jigsaw_fields[0].text = "minecraft:empty".to_string();
        let jigsaw = HudJigsawBlockEditorScreen {
            labels: [
                "Target Pool:".to_string(),
                "Name:".to_string(),
                "Target Name:".to_string(),
                "Turns into:".to_string(),
                "Selection Priority:".to_string(),
                "Placement Priority:".to_string(),
            ],
            joint_label: "Joint Type:".to_string(),
            fields: jigsaw_fields,
            joint_visible: true,
            joint: "Rollable".to_string(),
            levels: "Levels: 3".to_string(),
            slider_value_bits: 0,
            keep_jigsaws: "Keep Jigsaws: ON".to_string(),
            generate: "Generate".to_string(),
            done: "Done".to_string(),
            cancel: "Cancel".to_string(),
            controls_enabled: true,
            joint_hovered: false,
            levels_hovered: false,
            keep_jigsaws_hovered: false,
            generate_hovered: false,
            done_hovered: false,
            cancel_hovered: false,
            tooltip: None,
        };
        let jigsaw_text = jigsaw_block_editor_narration_frame(&jigsaw, None).all_text_for_test();
        assert!(jigsaw_text.contains("Jigsaw Block"));
        assert!(jigsaw_text.contains("Target Pool: edit box: minecraft:empty"));
        assert!(jigsaw_text.contains("Levels: 3"));

        let mut dialog_input = ClientInputState::default();
        dialog_input.set_screen_for_test(crate::input::Screen::DialogLink(
            crate::input::dialog::DialogLinkConfirmationState {
                url: "https://example.invalid/rules".into(),
                after_action: "none".into(),
                focused_button: Some(0),
            },
        ));
        let world = WorldStore::new();
        let dialog_text = dialog_screen_narration_frame(&dialog_input, &world, None)
            .expect("dialog link narration")
            .all_text_for_test();
        assert!(dialog_text.contains("https://example.invalid/rules"));
        assert!(dialog_text.contains("Yes button"));
    }
}
