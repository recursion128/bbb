use super::*;

pub(crate) fn hud_command_confirmation(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<bbb_client_render::HudCommandConfirmation> {
    hud_command_confirmation_snapshot(
        &input.command_confirmation_narration_snapshot()?,
        item_runtime,
    )
}

pub(crate) fn hud_command_confirmation_snapshot(
    screen: &crate::input::CommandConfirmationNarrationSnapshot,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<bbb_client_render::HudCommandConfirmation> {
    use bbb_client_world::UnattendedCommandCheck;
    let key = match screen.reason {
        UnattendedCommandCheck::ParseErrors => "multiplayer.confirm_command.parse_errors",
        UnattendedCommandCheck::SignatureRequired => {
            "multiplayer.confirm_command.signature_required"
        }
        UnattendedCommandCheck::PermissionsRequired => {
            "multiplayer.confirm_command.permissions_required"
        }
        UnattendedCommandCheck::NoIssues => return None,
    };
    let argument = bbb_protocol::component_styled_literal_nbt(
        &screen.command,
        &bbb_protocol::ComponentStyle {
            color: Some(0xffff55),
            ..Default::default()
        },
    )
    .ok()?;
    let message =
        bbb_protocol::component_translatable_nbt(key, vec![argument], &Default::default()).ok()?;
    let message = bbb_protocol::RichTextComponent::try_from_component_nbt(message).ok()?;
    let accept_label = if screen.reason != UnattendedCommandCheck::SignatureRequired {
        localized_gui_text(
            item_runtime,
            "multiplayer.confirm_command.run_command",
            "Run command",
        )
    } else if screen.open_chat_on_accept {
        localized_gui_text(
            item_runtime,
            "multiplayer.confirm_command.suggest_command",
            "Suggest command",
        )
    } else {
        localized_gui_text(item_runtime, "chat.copy", "Copy to clipboard")
    };
    Some(bbb_client_render::HudCommandConfirmation {
        generation: screen.generation,
        title: localized_gui_text(
            item_runtime,
            "multiplayer.confirm_command.title",
            "Run command?",
        ),
        message_runs: hud_component_text_runs(&message, item_runtime),
        accept_label,
        cancel_label: if screen.has_parent {
            localized_gui_text(item_runtime, "gui.back", "Back")
        } else {
            localized_gui_text(item_runtime, "gui.cancel", "Cancel")
        },
        focused: screen.focused,
        hovered: None,
    })
}

pub(crate) fn hud_death_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudDeathScreen> {
    let death = input.death_screen()?;
    let (canonical, runtime) = (&death.screen, &death.runtime);
    let screen = match runtime {
        DeathScreenRuntime::Main(state) => {
            let localized_cause = localized_component_runs(
                &canonical.cause_of_death,
                &bbb_protocol::ComponentStyle::default(),
                item_runtime,
            );
            let cause_link_run_indices = localized_cause
                .iter()
                .map(|run| {
                    let bbb_protocol::ComponentClickEvent::OpenUrl { .. } =
                        run.style.click_event.as_ref()?
                    else {
                        return None;
                    };
                    canonical
                        .cause_of_death
                        .styled()
                        .iter()
                        .position(|canonical_run| {
                            canonical_run.style.click_event == run.style.click_event
                        })
                        .and_then(|index| u16::try_from(index).ok())
                })
                .collect();
            HudDeathScreen::Main(HudDeathMainScreen {
                generation: state.generation,
                title: localized_gui_text(
                    item_runtime,
                    if canonical.hardcore {
                        "deathScreen.title.hardcore"
                    } else {
                        "deathScreen.title"
                    },
                    if canonical.hardcore {
                        "Game Over!"
                    } else {
                        "You Died!"
                    },
                ),
                cause_runs: localized_cause
                    .iter()
                    .map(|run| HudComponentTextRun {
                        visual: hud_chat_run(&run.text, &run.style),
                        font: run.style.font.clone(),
                    })
                    .collect(),
                cause_link_run_indices,
                score_runs: death_score_runs(canonical.score, item_runtime),
                respawn_label: localized_gui_text(
                    item_runtime,
                    if canonical.hardcore {
                        "deathScreen.spectate"
                    } else {
                        "deathScreen.respawn"
                    },
                    if canonical.hardcore {
                        "Spectate World"
                    } else {
                        "Respawn"
                    },
                ),
                title_screen_label: localized_gui_text(
                    item_runtime,
                    "deathScreen.titleScreen",
                    "Title Screen",
                ),
                buttons_active: state.buttons_active(),
                focused: state.focus,
                hovered: state.hovered,
            })
        }
        DeathScreenRuntime::QuitConfirmation(state) => {
            HudDeathScreen::QuitConfirmation(HudDeathQuitConfirmationScreen {
                generation: state.generation,
                title: localized_gui_text(
                    item_runtime,
                    "deathScreen.quit.confirm",
                    "Are you sure you want to quit?",
                ),
                title_screen_label: localized_gui_text(
                    item_runtime,
                    "deathScreen.titleScreen",
                    "Title Screen",
                ),
                respawn_label: localized_gui_text(item_runtime, "deathScreen.respawn", "Respawn"),
                buttons_active: state.buttons_active(),
                focused: state.focus,
                hovered: state.hovered,
            })
        }
        DeathScreenRuntime::LinkConfirmation(state) => {
            let confirmation = &state.confirmation;
            HudDeathScreen::LinkConfirmation(HudDeathLinkConfirmationScreen {
                generation: state.generation,
                title: localized_gui_text(
                    item_runtime,
                    "chat.link.confirmTrusted",
                    "Do you want to open this link or copy it to your clipboard?",
                ),
                message: confirmation.url.to_display_string(),
                open_label: localized_gui_text(item_runtime, "chat.link.open", "Open in Browser"),
                copy_label: localized_gui_text(item_runtime, "chat.copy", "Copy to Clipboard"),
                cancel_label: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
                focused: state.focus,
                hovered: state.hovered,
            })
        }
    };
    Some(screen)
}

pub(crate) fn hud_disconnected_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    panorama_rotation_degrees: f32,
) -> Option<HudDisconnectedScreen> {
    let runtime = input.disconnected_screen_runtime()?;
    Some(HudDisconnectedScreen {
        generation: runtime.generation,
        panorama_rotation_degrees,
        title: localized_gui_text(
            item_runtime,
            runtime.title.translation_key(),
            match runtime.title {
                crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectFailed => {
                    "Failed to Connect to the Server"
                }
                crate::input::disconnected_screen::DisconnectedScreenTitle::ConnectionLost => {
                    "Connection Lost"
                }
            },
        ),
        reason_runs: runtime
            .details
            .reason
            .runs
            .iter()
            .map(|run| HudComponentTextRun {
                visual: HudStyledTextRun {
                    shadow_color: run.shadow_color,
                    text: run.text.clone(),
                    style: HudTextStyle {
                        bold: run.bold,
                        italic: run.italic,
                        underlined: run.underlined,
                        strikethrough: run.strikethrough,
                        obfuscated: run.obfuscated,
                    },
                    color: run.color,
                },
                font: run.font.clone(),
            })
            .collect(),
        report_to_server_label: runtime
            .details
            .bug_report_link
            .as_ref()
            .map(|_| localized_gui_text(item_runtime, "gui.report_to_server", "Report to Server")),
        open_report_directory_label: runtime.details.report_directory.as_ref().map(|_| {
            localized_gui_text(item_runtime, "gui.open_report_dir", "Open Report Directory")
        }),
        #[cfg(not(target_arch = "wasm32"))]
        reconnect_label: Some(localized_gui_text(
            item_runtime,
            "bbb.disconnect.reconnect",
            "Reconnect",
        )),
        #[cfg(target_arch = "wasm32")]
        reconnect_label: None,
        #[cfg(not(target_arch = "wasm32"))]
        back_label: localized_gui_text(item_runtime, "menu.quit", "Quit Game"),
        #[cfg(target_arch = "wasm32")]
        back_label: localized_gui_text(item_runtime, "bbb.disconnect.reconnect", "Reconnect"),
        focused: runtime.focus,
        hovered: runtime.hovered,
    })
}

pub(crate) fn hud_social_interactions_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
    now: Option<bbb_client_clock::Instant>,
) -> Option<HudSocialInteractionsScreen> {
    let runtime = input.social_interactions_runtime()?;
    if runtime.link_confirmation_is_open() {
        return Some(HudSocialInteractionsScreen::LinkConfirmation(
            HudSocialInteractionsLinkConfirmationScreen {
                generation: runtime.generation(),
                title: localized_gui_text(
                    item_runtime,
                    "chat.link.confirmTrusted",
                    "Do you want to open this link or copy it to your clipboard?",
                ),
                message: crate::input::social::BLOCKING_HELP_URL.to_string(),
                open_label: localized_gui_text(item_runtime, "chat.link.open", "Open in Browser"),
                copy_label: localized_gui_text(item_runtime, "chat.copy", "Copy to Clipboard"),
                cancel_label: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
                focus: runtime.focus(),
                hovered: runtime.hovered(),
            },
        ));
    }

    let player_count = world.player_info().entries.len();
    let server_label = localized_gui_text_with_args(
        item_runtime,
        if player_count > 1 {
            "gui.socialInteractions.server_label.multiple"
        } else {
            "gui.socialInteractions.server_label.single"
        },
        if player_count > 1 {
            "%s - %s players"
        } else {
            "%s - %s player"
        },
        &[
            world.player_social().server_label.as_str(),
            &player_count.to_string(),
        ],
    );
    let local_player = world.local_player_profile_id();
    let rows = runtime
        .rows()
        .iter()
        .map(|player| {
            let action = (Some(player.profile.uuid) != local_player).then_some(if player.hidden {
                HudSocialInteractionsPlayerAction::Show
            } else {
                HudSocialInteractionsPlayerAction::Hide
            });
            HudSocialInteractionsPlayer {
                profile_id: *player.profile.uuid.as_bytes(),
                name: player.profile.name.clone(),
                skin: spectator_player_skin(world, &player.profile, item_runtime),
                removed: player.removed,
                hidden: player.hidden,
                blocked: player.blocked,
                action,
                status: social_player_status(player, item_runtime),
            }
        })
        .collect::<Vec<_>>();
    let page = match runtime.page() {
        bbb_client_world::SocialInteractionsPage::All => HudSocialInteractionsPage::All,
        bbb_client_world::SocialInteractionsPage::Hidden => HudSocialInteractionsPage::Hidden,
        bbb_client_world::SocialInteractionsPage::Blocked => HudSocialInteractionsPage::Blocked,
    };
    let empty_text = rows.is_empty().then(|| {
        if !runtime.search().is_empty() {
            localized_gui_text(
                item_runtime,
                "gui.socialInteractions.search_empty",
                "Couldn't find any players with that name",
            )
        } else {
            match page {
                HudSocialInteractionsPage::Hidden => localized_gui_text(
                    item_runtime,
                    "gui.socialInteractions.empty_hidden",
                    "No players hidden in chat",
                ),
                HudSocialInteractionsPage::Blocked => localized_gui_text(
                    item_runtime,
                    "gui.socialInteractions.empty_blocked",
                    "No blocked players in chat",
                ),
                HudSocialInteractionsPage::All => String::new(),
            }
        }
    });
    let tooltip = match now
        .is_some_and(|now| runtime.tooltip_is_ready(now))
        .then_some(runtime.hovered())
        .flatten()
    {
        Some(HudSocialInteractionsFocus::Player { action, .. }) => {
            cursor_position.map(|(x, y)| bbb_client_render::HudDebugOptionsTooltip {
                text: localized_gui_text(
                    item_runtime,
                    match action {
                        HudSocialInteractionsPlayerAction::Hide => {
                            "gui.socialInteractions.tooltip.hide"
                        }
                        HudSocialInteractionsPlayerAction::Show => {
                            "gui.socialInteractions.tooltip.show"
                        }
                    },
                    match action {
                        HudSocialInteractionsPlayerAction::Hide => "Hide messages",
                        HudSocialInteractionsPlayerAction::Show => "Show messages",
                    },
                ),
                x,
                y,
            })
        }
        _ => None,
    };
    Some(HudSocialInteractionsScreen::Main(
        HudSocialInteractionsMainScreen {
            generation: runtime.generation(),
            title: localized_gui_text(
                item_runtime,
                "gui.socialInteractions.title",
                "Social Interactions",
            ),
            server_label,
            page,
            tab_labels: [
                localized_gui_text(item_runtime, "gui.socialInteractions.tab_all", "All"),
                localized_gui_text(item_runtime, "gui.socialInteractions.tab_hidden", "Hidden"),
                localized_gui_text(
                    item_runtime,
                    "gui.socialInteractions.tab_blocked",
                    "Blocked",
                ),
            ],
            search: runtime
                .search()
                .hud(runtime.focus() == HudSocialInteractionsFocus::Search, false),
            search_hint: localized_gui_text(
                item_runtime,
                "gui.socialInteractions.search_hint",
                "Search...",
            ),
            rows,
            scroll_amount: runtime.scroll_amount(),
            empty_text: empty_text.filter(|text| !text.is_empty()),
            blocking_help_label: localized_gui_text(
                item_runtime,
                "gui.socialInteractions.blocking_hint",
                "Manage with Microsoft account",
            ),
            done_label: localized_gui_text(item_runtime, "gui.done", "Done"),
            focus: runtime.focus(),
            hovered: runtime.hovered(),
            tooltip,
        },
    ))
}

pub(super) fn social_player_status(
    player: &bbb_client_world::SocialPlayerState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let (key, fallback) = match (player.blocked, player.hidden, player.removed) {
        (true, _, true) => (
            "gui.socialInteractions.status_blocked_offline",
            "Blocked - Offline",
        ),
        (false, true, true) => (
            "gui.socialInteractions.status_hidden_offline",
            "Hidden - Offline",
        ),
        (true, _, false) => ("gui.socialInteractions.status_blocked", "Blocked"),
        (false, true, false) => ("gui.socialInteractions.status_hidden", "Hidden"),
        (false, false, true) => ("gui.socialInteractions.status_offline", "Offline"),
        (false, false, false) => return None,
    };
    Some(localized_gui_text(item_runtime, key, fallback))
}

pub(super) fn death_score_runs(
    score: i32,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudComponentTextRun> {
    let template = item_runtime
        .and_then(|runtime| runtime.localized_text_if_present("deathScreen.score.value"))
        .unwrap_or("Score: %s");
    let score = score.to_string();
    let mut runs = Vec::new();
    visit_localized_template(template, 1, |part| match part {
        LocalizedTemplatePart::Literal(text) => {
            if !text.is_empty() {
                runs.push(HudComponentTextRun::plain(text));
            }
        }
        LocalizedTemplatePart::Argument { index: 0, .. } => {
            runs.push(HudComponentTextRun {
                visual: HudStyledTextRun {
                    shadow_color: None,
                    text: score.clone(),
                    style: HudTextStyle::default(),
                    color: Some(0xFF_FF_55),
                },
                font: None,
            });
        }
        LocalizedTemplatePart::Argument { raw, .. } => {
            runs.push(HudComponentTextRun::plain(raw));
        }
    });
    runs
}

pub(crate) fn command_confirmation_narration_frame(
    input: &ClientInputState,
    hud: &bbb_client_render::HudCommandConfirmation,
    item_runtime: Option<&NativeItemRuntime>,
) -> crate::narrator_runtime::command_confirmation::CommandConfirmationNarrationFrame {
    command_confirmation_snapshot_narration_frame(
        &input
            .command_confirmation_narration_snapshot()
            .expect("confirmation projection"),
        hud,
        item_runtime,
    )
}

pub(crate) fn command_confirmation_snapshot_narration_frame(
    screen: &crate::input::CommandConfirmationNarrationSnapshot,
    hud: &bbb_client_render::HudCommandConfirmation,
    item_runtime: Option<&NativeItemRuntime>,
) -> crate::narrator_runtime::command_confirmation::CommandConfirmationNarrationFrame {
    use crate::narrator_runtime::{NarratedElementKind, ScreenNarrationFrame};
    use bbb_client_render::HudCommandConfirmationButton as Button;
    let button_index = |button| if button == Button::Accept { 0 } else { 1 };
    let message = hud
        .message_runs
        .iter()
        .map(|run| run.visual.text.as_str())
        .collect::<String>();
    let build = |index: usize, focused: bool| {
        let mut frame = ScreenNarrationFrame::default();
        frame.add(
            NarratedElementKind::Title,
            0,
            format!("{}. {}", hud.title, message),
        );
        frame.add(
            NarratedElementKind::Position,
            0,
            localized_gui_text_with_args(
                item_runtime,
                "narrator.position.screen",
                "Screen element %1$s out of %2$s",
                &[&(index + 1).to_string(), "2"],
            ),
        );
        if focused {
            frame.add(
                NarratedElementKind::Usage,
                0,
                localized_gui_text(
                    item_runtime,
                    "narration.component_list.usage",
                    "Press Tab to navigate to the next element",
                ),
            );
        }
        let label = if index == 0 {
            &hud.accept_label
        } else {
            &hud.cancel_label
        };
        frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.button",
                "%1$s button",
                &[label],
            ),
        );
        frame.add(
            NarratedElementKind::Usage,
            1,
            if focused {
                localized_gui_text(
                    item_runtime,
                    "narration.button.usage.focused",
                    "Press Enter to activate",
                )
            } else {
                localized_gui_text(
                    item_runtime,
                    "narration.button.usage.hovered",
                    "Click to activate",
                )
            },
        );
        frame
    };
    let mut unfocused_frame = ScreenNarrationFrame::default();
    unfocused_frame.add(
        NarratedElementKind::Title,
        0,
        format!("{}. {}", hud.title, message),
    );
    unfocused_frame.add(
        NarratedElementKind::Usage,
        0,
        localized_gui_text(
            item_runtime,
            "narrator.screen.usage",
            "Use mouse cursor or Tab button to select element",
        ),
    );
    crate::narrator_runtime::command_confirmation::CommandConfirmationNarrationFrame {
        generation: screen.generation,
        presentation: screen.presentation,
        retained: screen.retained.clone(),
        focused: screen.focused.map(button_index),
        hovered: hud.hovered.map(button_index),
        unfocused_frame,
        focused_frames: [build(0, true), build(1, true)],
        hovered_frames: [build(0, false), build(1, false)],
    }
}

#[cfg(test)]
mod command_confirmation_tests {
    use super::*;

    #[test]
    fn command_confirmation_narration_projects_full_message_and_button_usage() {
        let mut input = ClientInputState::default();
        input.start_unattended_command(
            &"/missing".into(),
            &mut WorldStore::new(),
            &mut NetCounters::default(),
            &None,
        );
        let mut hud = hud_command_confirmation(&input, None).unwrap();
        let message = (0..20)
            .map(|index| format!("line {index}\n"))
            .collect::<String>();
        hud.message_runs = vec![bbb_client_render::HudComponentTextRun::plain(&message)];
        hud.hovered = Some(bbb_client_render::HudCommandConfirmationButton::Cancel);
        let frame = command_confirmation_narration_frame(&input, &hud, None);
        assert_eq!(frame.hovered, Some(1));
        assert_eq!(hud.focused, None);
        assert_eq!(frame.focused, None);
        let unfocused = frame.unfocused_frame.all_text_for_test();
        assert!(unfocused.contains("Use mouse cursor or Tab button to select element"));
        assert!(!unfocused.contains("button. "));
        let focused = frame.focused_frames[0].all_text_for_test();
        assert!(focused.starts_with(&format!("Run command?. {message}")));
        assert!(focused.contains("Run command button"));
        assert!(focused.contains("Press Tab to navigate"));
        assert!(focused.contains("Press Enter to activate"));
        let hovered = frame.hovered_frames[1].all_text_for_test();
        assert!(hovered.contains("Cancel button"));
        assert!(hovered.contains("Click to activate"));
        assert!(!hovered.contains("Press Tab"));
    }
}
