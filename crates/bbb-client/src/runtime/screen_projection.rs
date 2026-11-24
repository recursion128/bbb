use super::*;

pub(super) fn hud_pause_screen(
    input: &ClientInputState,
    _world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudPauseScreen> {
    let state = input.debug_pause_screen()?;
    let feedback_subscreen = state.page == PauseScreenPage::Feedback;
    let return_to_game_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_return_to_game_button_contains(state.cursor_position, surface_size);
    let advancements_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_advancements_button_contains(state.cursor_position, surface_size);
    let stats_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_stats_button_contains(state.cursor_position, surface_size);
    let send_feedback_hovered = state.show_pause_menu
        && if feedback_subscreen {
            pause_screen_feedback_send_feedback_button_contains(state.cursor_position, surface_size)
        } else {
            pause_screen_send_feedback_button_contains(state.cursor_position, surface_size)
        };
    let report_bugs_hovered = state.show_pause_menu
        && pause_screen_report_bugs_button_enabled()
        && if feedback_subscreen {
            pause_screen_feedback_report_bugs_button_contains(state.cursor_position, surface_size)
        } else {
            state.custom_dialog.is_none()
                && pause_screen_report_bugs_button_contains(state.cursor_position, surface_size)
        };
    let custom_dialog_hovered = state.show_pause_menu
        && !feedback_subscreen
        && state.custom_dialog.is_some()
        && pause_screen_report_bugs_button_contains(state.cursor_position, surface_size);
    let feedback_back_hovered = state.show_pause_menu
        && feedback_subscreen
        && pause_screen_feedback_back_button_contains(state.cursor_position, surface_size);
    let options_button_hovered = state.show_pause_menu
        && !feedback_subscreen
        && pause_screen_options_button_contains(state.cursor_position, surface_size);
    let options_hovered = options_button_hovered && pause_screen_options_button_enabled();
    let options_tooltip =
        (options_button_hovered && !pause_screen_options_button_enabled()).then(|| {
            let (x, y) = state.cursor_position.unwrap_or_default();
            HudPauseScreenTooltip {
                text: "Options are configured via CLI flags or options.txt".to_string(),
                x,
                y,
            }
        });
    let disconnect_enabled = !input.pause_screen_disconnect_requested();
    let disconnect_hovered = state.show_pause_menu
        && !feedback_subscreen
        && disconnect_enabled
        && pause_screen_disconnect_button_contains(state.cursor_position, surface_size);
    let custom_dialog_label = state
        .custom_dialog
        .as_ref()
        .map(|dialog| dialog.external_title().text.clone());
    let custom_dialog_tooltip = custom_dialog_hovered.then(|| {
        let (x, y) = state.cursor_position.unwrap_or_default();
        HudPauseScreenTooltip {
            text: localized_gui_text(
                item_runtime,
                "menu.custom_options.tooltip",
                "Note: Custom options are provided by third-party servers and/or content.\nHandle with care!",
            ),
            x,
            y,
        }
    });
    Some(HudPauseScreen {
        title: if feedback_subscreen {
            localized_gui_text(item_runtime, "menu.feedback.title", "Feedback")
        } else if state.show_pause_menu {
            localized_gui_text(item_runtime, "menu.game", "Game Menu")
        } else {
            localized_gui_text(item_runtime, "menu.paused", "Game Paused")
        },
        labels: HudPauseScreenLabels {
            return_to_game: localized_gui_text(item_runtime, "menu.returnToGame", "Back to Game"),
            advancements: localized_gui_text(item_runtime, "gui.advancements", "Advancements"),
            stats: localized_gui_text(item_runtime, "gui.stats", "Statistics"),
            send_feedback: localized_gui_text(item_runtime, "menu.sendFeedback", "Give Feedback"),
            feedback: localized_gui_text(item_runtime, "menu.feedback", "Feedback..."),
            report_bugs: localized_gui_text(item_runtime, "menu.reportBugs", "Report Bugs"),
            back: localized_gui_text(item_runtime, "gui.back", "Back"),
            options: localized_gui_text(item_runtime, "menu.options", "Options..."),
            disconnect: localized_gui_text(item_runtime, "menu.disconnect", "Disconnect"),
        },
        show_pause_menu: state.show_pause_menu,
        feedback_subscreen,
        return_to_game_hovered,
        advancements_hovered,
        stats_hovered,
        send_feedback_hovered,
        report_bugs_hovered,
        report_bugs_enabled: pause_screen_report_bugs_button_enabled(),
        custom_dialog_label,
        custom_dialog_hovered,
        custom_dialog_tooltip,
        feedback_back_hovered,
        options_hovered,
        options_enabled: pause_screen_options_button_enabled(),
        options_tooltip,
        disconnect_hovered,
        disconnect_enabled,
    })
}

pub(super) fn hud_game_event_screen(
    state: &mut HudRenderRuntime,
    input: &ClientInputState,
    resources: &Arc<crate::win_screen_resources::WinScreenResources>,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudGameEventScreen> {
    let screen = match input.game_event_screen_runtime()? {
        GameEventScreenRuntime::Win(screen) => HudGameEventScreen::Win(HudWinScreen {
            generation: screen.generation,
            scroll: screen.scroll,
            show_easter_egg_logo: screen.show_easter_egg_logo,
            lines: state.win_screen_lines(resources),
        }),
        GameEventScreenRuntime::DemoHelp(screen) => {
            let movement = localized_gui_text_with_args(
                item_runtime,
                "demo.help.movementShort",
                "Move by pressing the %1$s, %2$s, %3$s, %4$s keys",
                &["W", "A", "S", "D"],
            );
            let mouse = localized_gui_text(
                item_runtime,
                "demo.help.movementMouse",
                "Look around using the mouse",
            );
            let jump = localized_gui_text_with_args(
                item_runtime,
                "demo.help.jump",
                "Jump by pressing the %1$s key",
                &["Space"],
            );
            let inventory = localized_gui_text_with_args(
                item_runtime,
                "demo.help.inventory",
                "Use the %1$s key to open your inventory",
                &["E"],
            );
            HudGameEventScreen::DemoPopup(HudDemoPopupScreen {
                generation: screen.generation,
                title: localized_gui_text(
                    item_runtime,
                    "demo.help.title",
                    "Minecraft Demo Mode",
                ),
                controls_message: [movement, mouse, jump, inventory].join("\n"),
                duration_message: localized_gui_text(
                    item_runtime,
                    "demo.help.fullWrapped",
                    "This demo will last 5 in-game days (about 1 hour and 40 minutes of real time). Check the advancements for hints! Have fun!",
                ),
                buy_label: localized_gui_text(
                    item_runtime,
                    "demo.help.buy",
                    "Purchase Now!",
                ),
                later_label: localized_gui_text(
                    item_runtime,
                    "demo.help.later",
                    "Continue Playing!",
                ),
                focused: match screen.focus {
                    DemoHelpFocus::Buy => HudGameEventButton::DemoBuy,
                    DemoHelpFocus::Later => HudGameEventButton::DemoLater,
                },
                hovered: screen.hovered.map(|hovered| match hovered {
                    DemoHelpFocus::Buy => HudGameEventButton::DemoBuy,
                    DemoHelpFocus::Later => HudGameEventButton::DemoLater,
                }),
            })
        }
        GameEventScreenRuntime::DemoLinkConfirmation(screen) => {
            HudGameEventScreen::DemoLinkConfirmation(HudDemoLinkConfirmationScreen {
                generation: screen.generation,
                title: localized_gui_text(
                    item_runtime,
                    "chat.link.confirmTrusted",
                    "Do you want to open this link or copy it to your clipboard?",
                ),
                message: crate::input::game_event_screen::BUY_MINECRAFT_JAVA_URL.to_string(),
                open_label: localized_gui_text(item_runtime, "chat.link.open", "Open in Browser"),
                copy_label: localized_gui_text(item_runtime, "chat.copy", "Copy to Clipboard"),
                cancel_label: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
                focused: match screen.focus {
                    DemoLinkConfirmationFocus::Open => HudGameEventButton::LinkOpen,
                    DemoLinkConfirmationFocus::Copy => HudGameEventButton::LinkCopy,
                    DemoLinkConfirmationFocus::Cancel => HudGameEventButton::LinkCancel,
                },
                hovered: screen.hovered.map(|hovered| match hovered {
                    DemoLinkConfirmationFocus::Open => HudGameEventButton::LinkOpen,
                    DemoLinkConfirmationFocus::Copy => HudGameEventButton::LinkCopy,
                    DemoLinkConfirmationFocus::Cancel => HudGameEventButton::LinkCancel,
                }),
            })
        }
    };
    Some(screen)
}

pub(super) fn hud_command_suggestions(
    input: &ClientInputState,
    world: &WorldStore,
    renderer: &Renderer,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudCommandSuggestions> {
    let state = input.command_suggestion_hud_state_ref(world, renderer, surface_size)?;
    let tooltip = state.tooltip.map(|tooltip| HudCommandSuggestionTooltip {
        x: tooltip.x,
        y: tooltip.y,
        runs: hud_command_suggestion_tooltip_runs(tooltip, item_runtime),
    });
    Some(HudCommandSuggestions {
        preedit: input.chat_ime_preedit_hud_state().map(hud_ime_preedit),
        input_text: state.input_text,
        input_view: state.input_view,
        draft: state.draft,
        syntax: state.syntax,
        cursor: state.cursor,
        selection: state.selection,
        cursor_visible: state.cursor_visible,
        suffix: state.suffix,
        list_x: state.rect.x,
        list_y: state.rect.y,
        list_width: state.rect.width,
        list_height: state.rect.height,
        entries: state.entries,
        has_previous: state.has_previous,
        has_next: state.has_next,
        tooltip,
        layout: if state.command_block_layout {
            HudCommandSuggestionsLayout::CommandBlock
        } else {
            HudCommandSuggestionsLayout::Chat
        },
    })
}

pub(super) fn hud_chat_restrictions_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    cursor_position: Option<(i32, i32)>,
) -> Option<HudChatRestrictionsScreen> {
    let generation = input.chat_restrictions_screen_generation()?;
    let visibility = world.chat_presentation_config().visibility;
    let (reason_key, reason_fallback) = match visibility {
        bbb_client_world::ChatVisibilityState::System => (
            "chat_restriction.chat_disabled_by_options",
            "Player chat is restricted in client settings.",
        ),
        bbb_client_world::ChatVisibilityState::Hidden => (
            "chat_restriction.chat_and_commands_disabled_by_options",
            "Chat is restricted in client settings.",
        ),
        bbb_client_world::ChatVisibilityState::Full => return None,
    };
    let permission =
        |id: &str, allowed: bool, allowed_fallback: &str, denied_fallback: &str| HudStyledTextRun {
            shadow_color: None,
            text: localized_gui_text(
                item_runtime,
                &format!(
                    "restrictions_screen.permission.{id}.{}",
                    if allowed { "allowed" } else { "denied" }
                ),
                if allowed {
                    allowed_fallback
                } else {
                    denied_fallback
                },
            ),
            style: Default::default(),
            color: Some(if allowed { 0x55FF55 } else { 0xFF5555 }),
        };
    let can_send_messages = visibility == bbb_client_world::ChatVisibilityState::Full;
    let can_send_commands = visibility != bbb_client_world::ChatVisibilityState::Hidden;
    Some(HudChatRestrictionsScreen {
        generation,
        title: localized_gui_text(item_runtime, "restrictions_screen.title", "Restrictions"),
        reason: HudStyledTextRun {
            shadow_color: None,
            text: localized_gui_text(item_runtime, reason_key, reason_fallback),
            style: bbb_client_render::HudTextStyle {
                bold: true,
                ..Default::default()
            },
            color: Some(0xFF5555),
        },
        permissions: vec![
            permission(
                "send_messages",
                can_send_messages,
                "You can send chat messages",
                "You can't send chat messages",
            ),
            permission(
                "send_commands",
                can_send_commands,
                "You can send commands",
                "You can't send commands",
            ),
            permission(
                "receive_system_messages",
                can_send_commands,
                "You can receive system messages from the server",
                "You can't receive system messages from the server",
            ),
            permission(
                "receive_player_messages",
                can_send_messages,
                "You can receive messages from players",
                "You can't receive messages from players",
            ),
        ],
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cursor_position,
    })
}

pub(super) fn hud_command_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudCommandBlockEditorScreen> {
    let state = input.command_block_editor_hud_state()?;
    let layout = if state.block.is_some() {
        hud_command_block_editor_layout(surface_size)
    } else {
        hud_minecart_command_block_editor_layout(surface_size)
    };
    let hovered = |rect: bbb_client_render::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    Some(HudCommandBlockEditorScreen {
        kind: if state.block.is_some() {
            HudCommandBlockEditorKind::Block
        } else {
            HudCommandBlockEditorKind::Minecart
        },
        title: localized_gui_text(
            item_runtime,
            "advMode.setCommand",
            "Set Console Command for Block",
        ),
        command_label: localized_gui_text(item_runtime, "advMode.command", "Console Command"),
        previous_output_label: localized_gui_text(
            item_runtime,
            "advMode.previousOutput",
            "Previous Output",
        ),
        previous_output: if state.track_output {
            state.previous_output.to_owned()
        } else {
            "-".to_string()
        },
        mode: state
            .block
            .map_or_else(String::new, |block| match block.mode {
                CommandBlockMode::Sequence => {
                    localized_gui_text(item_runtime, "advMode.mode.sequence", "Chain")
                }
                CommandBlockMode::Auto => {
                    localized_gui_text(item_runtime, "advMode.mode.auto", "Repeat")
                }
                CommandBlockMode::Redstone => {
                    localized_gui_text(item_runtime, "advMode.mode.redstone", "Impulse")
                }
            }),
        conditional: state.block.map_or_else(String::new, |block| {
            localized_gui_text(
                item_runtime,
                if block.conditional {
                    "advMode.mode.conditional"
                } else {
                    "advMode.mode.unconditional"
                },
                if block.conditional {
                    "Conditional"
                } else {
                    "Unconditional"
                },
            )
        }),
        automatic: state.block.map_or_else(String::new, |block| {
            localized_gui_text(
                item_runtime,
                if block.automatic {
                    "advMode.mode.autoexec.bat"
                } else {
                    "advMode.mode.redstoneTriggered"
                },
                if block.automatic {
                    "Always Active"
                } else {
                    "Needs Redstone"
                },
            )
        }),
        track_output: if state.track_output { "O" } else { "X" }.to_string(),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        controls_enabled: state.block.is_none_or(|block| block.controls_enabled),
        mode_hovered: state.block.is_some() && hovered(layout.mode),
        conditional_hovered: state.block.is_some() && hovered(layout.conditional),
        automatic_hovered: state.block.is_some() && hovered(layout.automatic),
        track_output_hovered: hovered(layout.track_output),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
    })
}

pub(super) fn hud_structure_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudStructureBlockEditorScreen> {
    let state = input.structure_block_editor_hud_state()?;
    let layout = hud_structure_block_editor_layout(surface_size);
    let hovered = |rect: bbb_client_render::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let fields = std::array::from_fn(|index| {
        state.fields[index].hud(state.focus_index == index, hovered(layout.fields[index]))
    });
    let (mode, mode_key, mode_fallback) = match state.mode {
        StructureBlockMode::Save => (HudStructureBlockMode::Save, "save", "Save"),
        StructureBlockMode::Load => (HudStructureBlockMode::Load, "load", "Load"),
        StructureBlockMode::Corner => (HudStructureBlockMode::Corner, "corner", "Corner"),
        StructureBlockMode::Data => (HudStructureBlockMode::Data, "data", "Data"),
    };
    let (mirror_key, mirror_fallback) = match state.mirror {
        StructureBlockMirror::None => ("mirror.none", "|"),
        StructureBlockMirror::LeftRight => ("mirror.left_right", "< >"),
        StructureBlockMirror::FrontBack => ("mirror.front_back", "^ v"),
    };
    let rotation = match state.rotation {
        StructureBlockRotation::None => "0",
        StructureBlockRotation::Clockwise90 => "90",
        StructureBlockRotation::Clockwise180 => "180",
        StructureBlockRotation::Counterclockwise90 => "270",
    };
    let on_off = |value: bool| {
        localized_gui_text(
            item_runtime,
            if value { "options.on" } else { "options.off" },
            if value { "On" } else { "Off" },
        )
    };
    let (option_key, option_fallback) = if state.mode == StructureBlockMode::Load {
        ("structure_block.show_boundingbox", "Show Bounding Box")
    } else {
        ("structure_block.show_air", "Show Air")
    };

    Some(HudStructureBlockEditorScreen {
        title: localized_gui_text(
            item_runtime,
            "block.minecraft.structure_block",
            "Structure Block",
        ),
        name_label: localized_gui_text(
            item_runtime,
            "structure_block.structure_name",
            "Structure Name",
        ),
        position_label: localized_gui_text(
            item_runtime,
            "structure_block.position",
            "Relative Position",
        ),
        size_label: localized_gui_text(item_runtime, "structure_block.size", "Structure Size"),
        integrity_label: localized_gui_text(
            item_runtime,
            "structure_block.integrity",
            "Structure Integrity and Seed",
        ),
        data_label: localized_gui_text(
            item_runtime,
            "structure_block.custom_data",
            "Custom Data Tag Name",
        ),
        include_entities_label: localized_gui_text(
            item_runtime,
            "structure_block.include_entities",
            "Include Entities",
        ),
        strict_label: localized_gui_text(item_runtime, "structure_block.strict", "Strict"),
        detect_label: localized_gui_text(
            item_runtime,
            "structure_block.detect_size",
            "Detect Structure Size and Position",
        ),
        option_label: localized_gui_text(item_runtime, option_key, option_fallback),
        mode_description: localized_gui_text(
            item_runtime,
            &format!("structure_block.mode_info.{mode_key}"),
            mode_fallback,
        ),
        fields,
        mode,
        mode_value: localized_gui_text(
            item_runtime,
            &format!("structure_block.mode.{mode_key}"),
            mode_fallback,
        ),
        mirror_value: localized_gui_text(item_runtime, mirror_key, mirror_fallback),
        rotation: rotation.to_string(),
        include_entities: on_off(state.include_entities),
        strict: on_off(state.strict),
        option: on_off(if state.mode == StructureBlockMode::Load {
            state.show_bounding_box
        } else {
            state.show_air
        }),
        action: localized_gui_text(
            item_runtime,
            if state.mode == StructureBlockMode::Load {
                "structure_block.button.load"
            } else {
                "structure_block.button.save"
            },
            if state.mode == StructureBlockMode::Load {
                "Load"
            } else {
                "Save"
            },
        ),
        detect: localized_gui_text(item_runtime, "structure_block.button.detect_size", "Detect"),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        mode_hovered: hovered(layout.mode),
        mirror_hovered: hovered(layout.mirror),
        rotation_hovered: layout.rotations.map(hovered),
        include_entities_hovered: hovered(layout.include_entities),
        strict_hovered: hovered(layout.strict),
        option_hovered: hovered(layout.option),
        action_hovered: hovered(layout.action),
        detect_hovered: hovered(layout.detect),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
    })
}

pub(super) fn hud_jigsaw_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudJigsawBlockEditorScreen> {
    let state = input.jigsaw_block_editor_hud_state()?;
    let layout = hud_jigsaw_block_editor_layout(surface_size);
    let hovered = |rect: bbb_client_render::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let fields = std::array::from_fn(|index| {
        state.fields[index].hud(state.focus_index == index, hovered(layout.fields[index]))
    });
    let joint = match state.joint {
        JigsawJoint::Rollable => {
            localized_gui_text(item_runtime, "jigsaw_block.joint.rollable", "Rollable")
        }
        JigsawJoint::Aligned => {
            localized_gui_text(item_runtime, "jigsaw_block.joint.aligned", "Aligned")
        }
    };
    let keep_label = localized_gui_text(item_runtime, "jigsaw_block.keep_jigsaws", "Keep Jigsaws");
    let keep_value = localized_gui_text(
        item_runtime,
        if state.keep_jigsaws {
            "options.on"
        } else {
            "options.off"
        },
        if state.keep_jigsaws { "ON" } else { "OFF" },
    );
    let tooltip = state.mouse_position.and_then(|(x, y)| {
        let (key, fallback) = jigsaw_priority_tooltip_key(layout, (x, y))?;
        Some(HudDebugOptionsTooltip {
            text: localized_gui_text(item_runtime, key, fallback),
            x,
            y,
        })
    });

    Some(HudJigsawBlockEditorScreen {
        labels: [
            localized_gui_text(item_runtime, "jigsaw_block.pool", "Target Pool:"),
            localized_gui_text(item_runtime, "jigsaw_block.name", "Name:"),
            localized_gui_text(item_runtime, "jigsaw_block.target", "Target Name:"),
            localized_gui_text(item_runtime, "jigsaw_block.final_state", "Turns into:"),
            localized_gui_text(
                item_runtime,
                "jigsaw_block.selection_priority",
                "Selection Priority:",
            ),
            localized_gui_text(
                item_runtime,
                "jigsaw_block.placement_priority",
                "Placement Priority:",
            ),
        ],
        joint_label: localized_gui_text(item_runtime, "jigsaw_block.joint_label", "Joint Type:"),
        fields,
        joint_visible: state.joint_editable,
        joint,
        levels: localized_gui_text_with_args(
            item_runtime,
            "jigsaw_block.levels",
            "Levels: %s",
            &[&state.levels.to_string()],
        ),
        slider_value_bits: state.slider_value_bits,
        keep_jigsaws: localized_gui_text_with_args(
            item_runtime,
            "options.generic_value",
            "%s: %s",
            &[&keep_label, &keep_value],
        ),
        generate: localized_gui_text(item_runtime, "jigsaw_block.generate", "Generate"),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        controls_enabled: state.controls_enabled,
        joint_hovered: state.joint_editable && hovered(layout.joint),
        levels_hovered: state.dragging_levels || hovered(layout.levels),
        keep_jigsaws_hovered: hovered(layout.keep_jigsaws),
        generate_hovered: hovered(layout.generate),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
        tooltip,
    })
}

pub(super) fn jigsaw_priority_tooltip_key(
    layout: bbb_client_render::HudJigsawBlockEditorLayout,
    mouse_position: (i32, i32),
) -> Option<(&'static str, &'static str)> {
    let (x, y) = mouse_position;
    if layout.fields[4].contains(x, y) {
        Some((
            "jigsaw_block.selection_priority.tooltip",
            "Controls the order in which this Jigsaw attempts to connect.",
        ))
    } else if layout.fields[5].contains(x, y) {
        Some((
            "jigsaw_block.placement_priority.tooltip",
            "Controls the order in which the connected piece is processed.",
        ))
    } else {
        None
    }
}

pub(super) fn hud_test_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudTestBlockEditorScreen> {
    let state = input.test_block_editor_hud_state()?;
    let layout = hud_test_block_editor_layout(surface_size);
    let hovered = |rect: bbb_client_render::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let (mode_key, mode_fallback, info_key, info_fallback) = match state.mode {
        TestBlockMode::Start => (
            "test_block.mode.start",
            "Start",
            "test_block.mode_info.start",
            "Start Mode - The starting point for a test",
        ),
        TestBlockMode::Log => (
            "test_block.mode.log",
            "Log",
            "test_block.mode_info.log",
            "Log Mode - Log a message",
        ),
        TestBlockMode::Fail => (
            "test_block.mode.fail",
            "Fail",
            "test_block.mode_info.fail",
            "Fail Mode - Fail the test",
        ),
        TestBlockMode::Accept => (
            "test_block.mode.accept",
            "Accept",
            "test_block.mode_info.accept",
            "Accept Mode - Accept success for (part of) a test",
        ),
    };
    let message_visible = state.mode != TestBlockMode::Start;
    Some(HudTestBlockEditorScreen {
        title: localized_gui_text(item_runtime, "block.minecraft.test_block", "Test Block"),
        message_label: localized_gui_text(item_runtime, "test_block.message", "Message:"),
        message: state
            .message
            .hud(message_visible, message_visible && hovered(layout.message)),
        message_visible,
        mode: localized_gui_text(item_runtime, mode_key, mode_fallback),
        mode_info: localized_gui_text(item_runtime, info_key, info_fallback),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        mode_hovered: hovered(layout.mode),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
    })
}

pub(super) fn hud_test_instance_block_editor_screen(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudTestInstanceBlockEditorScreen> {
    let state = input.test_instance_block_editor_hud_state()?;
    let layout = hud_test_instance_block_editor_layout(surface_size);
    let hovered = |rect: bbb_client_render::HudCommandBlockEditorRect| {
        state
            .mouse_position
            .is_some_and(|(x, y)| rect.contains(x, y))
    };
    let field = |index: usize, rect: bbb_client_render::HudCommandBlockEditorRect| {
        state.fields[index].hud(state.focus_index == index, hovered(rect))
    };

    let mut info_runs = Vec::new();
    if state.local_invalid_id {
        info_runs.push(HudStyledTextRun {
            shadow_color: None,
            text: localized_gui_text(
                item_runtime,
                "test_instance.description.invalid_id",
                "Invalid test ID",
            ),
            style: HudTextStyle::default(),
            color: Some(0xFF5555),
        });
    } else if let Some(status) = state.response_status {
        if let Some(error) = state.error_message {
            let failed = localized_gui_text(
                item_runtime,
                "test_instance.description.failed",
                "Failed: %s",
            )
            .replace("%1$s", "")
            .replace("%s", "");
            info_runs.push(HudStyledTextRun {
                shadow_color: None,
                text: failed,
                style: HudTextStyle::default(),
                color: None,
            });
            info_runs.extend(
                hud_styled_runs_from_rich_text(error)
                    .into_iter()
                    .map(|mut run| {
                        if run.color.is_none() {
                            run.color = Some(0xFF5555);
                        }
                        run
                    }),
            );
            info_runs.push(HudStyledTextRun::plain("\n\n"));
        }
        info_runs.extend(hud_styled_runs_from_rich_text(status));
    }

    Some(HudTestInstanceBlockEditorScreen {
        title: localized_gui_text(
            item_runtime,
            "block.minecraft.test_instance_block",
            "Test Instance Block",
        ),
        id_label: localized_gui_text(
            item_runtime,
            "test_instance_block.test_id",
            "Test Instance ID",
        ),
        size_label: localized_gui_text(
            item_runtime,
            "test_instance_block.size",
            "Test Structure Size",
        ),
        rotation_label: localized_gui_text(
            item_runtime,
            "test_instance_block.rotation",
            "Rotation:",
        ),
        entities_label: localized_gui_text(
            item_runtime,
            "test_instance_block.entities",
            "Entities:",
        ),
        id: field(0, layout.id),
        size: [
            field(1, layout.size[0]),
            field(2, layout.size[1]),
            field(3, layout.size[2]),
        ],
        info_runs,
        rotation: match state.rotation {
            TestInstanceBlockRotation::None => "0",
            TestInstanceBlockRotation::Clockwise90 => "90",
            TestInstanceBlockRotation::Clockwise180 => "180",
            TestInstanceBlockRotation::Counterclockwise90 => "270",
        }
        .to_string(),
        include_entities: localized_gui_text(
            item_runtime,
            if state.include_entities {
                "options.on"
            } else {
                "options.off"
            },
            if state.include_entities { "On" } else { "Off" },
        ),
        reset: localized_gui_text(item_runtime, "test_instance.action.reset", "Reset and Load"),
        save: localized_gui_text(item_runtime, "test_instance.action.save", "Save Structure"),
        run: localized_gui_text(item_runtime, "test_instance.action.run", "Load and Run"),
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        cancel: localized_gui_text(item_runtime, "gui.cancel", "Cancel"),
        save_enabled: state.save_enabled,
        rotation_hovered: hovered(layout.rotation),
        include_entities_hovered: hovered(layout.include_entities),
        reset_hovered: hovered(layout.reset),
        save_hovered: hovered(layout.save),
        run_hovered: hovered(layout.run),
        done_hovered: hovered(layout.done),
        cancel_hovered: hovered(layout.cancel),
    })
}

pub(super) fn hud_command_suggestion_tooltip_runs(
    tooltip: CommandSuggestionHudTooltip,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    if let Some(translation_key) = tooltip.translation_key.as_deref() {
        return vec![HudStyledTextRun {
            shadow_color: None,
            text: localized_gui_text(item_runtime, translation_key, &tooltip.text),
            style: HudTextStyle::default(),
            color: None,
        }];
    }
    tooltip
        .runs
        .into_iter()
        .map(|run| HudStyledTextRun {
            shadow_color: run.style.shadow_color,
            text: run.text,
            style: HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
        })
        .collect()
}

pub(super) fn hud_sign_editor_screen(
    input: &ClientInputState,
    world: &WorldStore,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> Option<HudSignEditorScreen> {
    let state = input.sign_editor_hud_state()?;
    let kind = hud_sign_editor_kind(world, state.pos);
    let title = match kind {
        HudSignEditorKind::Hanging { .. } => "Edit Hanging Sign Message",
        HudSignEditorKind::Standing { .. } => "Edit Sign Message",
    }
    .to_string();
    Some(HudSignEditorScreen {
        kind,
        sign_preview: hud_sign_editor_preview(kind),
        title,
        lines: state.lines,
        line: state.line,
        cursor: state.cursor,
        selection: state.selection,
        cursor_visible: state.cursor_visible,
        text_tint: hud_sign_editor_text_tint(world, state.pos, state.is_front_text),
        done: "Done".to_string(),
        done_hovered: input.sign_editor_done_button_hovered(surface_size),
        preedit: state.preedit.map(hud_ime_preedit),
    })
}

fn hud_sign_editor_kind(world: &WorldStore, pos: BlockPos) -> HudSignEditorKind {
    let Some((wood, form)) = world
        .probe_block(pos)
        .and_then(|probe| probe.block_name)
        .and_then(|name| sign_wood_and_form_for_block_name(&name))
    else {
        return HudSignEditorKind::Standing {
            wood: SignModelWood::Oak,
            attachment: SignModelAttachment::Standing,
        };
    };
    let wood = sign_model_wood(wood);
    match form {
        SignBlockForm::Standing => HudSignEditorKind::Standing {
            wood,
            attachment: sign_model_attachment(bbb_client_world::SignModelAttachment::Standing),
        },
        SignBlockForm::Wall => HudSignEditorKind::Standing {
            wood,
            attachment: sign_model_attachment(bbb_client_world::SignModelAttachment::Wall),
        },
        SignBlockForm::HangingCeiling | SignBlockForm::HangingWall => {
            HudSignEditorKind::Hanging { wood }
        }
    }
}

fn hud_sign_editor_preview(kind: HudSignEditorKind) -> Option<HudEntityPreview> {
    const ENTITY_ID: i32 = -1;
    const WIDTH: u32 = 96;
    const HEIGHT: u32 = 102;
    const SIGN_SCALE: f32 = 62.500_004;

    let HudSignEditorKind::Standing { wood, attachment } = kind else {
        return None;
    };
    let mut entity = EntityModelInstance::sign(ENTITY_ID, [0.0, 0.0, 0.0], 0.0, wood, attachment)
        .with(|s| s.light_coords = ENTITY_FULL_BRIGHT_LIGHT_COORDS);
    entity.render_state.outline_color = 0;
    entity.render_state.appears_glowing = false;
    Some(HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::ItemsFlat,
        rect: HudEntityPreviewRect {
            x: 0,
            y: 0,
            width: WIDTH,
            height: HEIGHT,
        },
        scissor: None,
        translation: [0.0, HEIGHT as f32 / (2.0 * SIGN_SCALE) - 0.75, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        override_camera_rotation: None,
        scale: SIGN_SCALE,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: Vec::new(),
    })
}

fn hud_sign_editor_text_tint(world: &WorldStore, pos: BlockPos, is_front_text: bool) -> [f32; 4] {
    let color = world
        .sign_text_state_at(pos)
        .map(|text| {
            let side = text.side(is_front_text);
            sign_text_base_color(side.color.text_color(), side.has_glowing_text)
        })
        .unwrap_or_default();
    let [red, green, blue] = rgb24(color as i32);
    [red, green, blue, 1.0]
}

pub(super) fn release_input_if_screen_opened(
    input_screen_was_open: bool,
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
) {
    if !input_screen_was_open && input.blocks_world_input() {
        input.interrupt_chat_screen();
        release_active_input(input, world, counters, net_commands);
    }
}

fn hud_ime_preedit(preedit: crate::input::ImePreeditHudState) -> HudImePreedit {
    HudImePreedit {
        text: preedit.text,
        focused_start: preedit.focused_start,
        focused_end: preedit.focused_end,
        caret: preedit.caret,
        caret_visible: preedit.caret_visible,
    }
}
