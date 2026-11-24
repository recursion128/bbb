use super::*;
use crate::input::dialog::DialogInputValueState;

pub(super) fn run_dialog_confirm_link_button(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
    index: usize,
) -> bool {
    let resolved = match index {
        0 => input.resolve_dialog_link_confirmation(true),
        1 => {
            let url = input
                .dialog_link_confirmation()
                .map(|confirmation| confirmation.url.clone());
            if let Some(url) = url {
                if !url.units().is_empty() {
                    if let Some(clipboard) = clipboard.as_deref_mut() {
                        clipboard.set_debug_clipboard_java_text(&url);
                    }
                }
            }
            input.resolve_dialog_link_confirmation(false)
        }
        2 => input.resolve_dialog_link_confirmation(false),
        _ => false,
    };
    if resolved {
        input.refresh_dialog_parent_suggestions(world, counters, commands);
    }
    resolved
}

pub(super) fn run_dialog_warning_button(
    world: &mut WorldStore,
    counters: &mut NetCounters,
    input: &mut crate::input::ClientInputState,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    disconnect: bool,
) {
    if disconnect {
        input.mark_local_disconnect_requested();
        if let Some(tx) = net_commands {
            let _ = tx.try_send(NetCommand::Disconnect);
        }
    } else {
        if input.close_dialog_warning() {
            input.refresh_dialog_parent_suggestions(world, counters, net_commands);
        }
    }
}

pub(super) fn set_dialog_number_range_from_cursor(
    input: &mut crate::input::ClientInputState,
    layout: DialogInputLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let (origin_x, _) = centered_screen_origin(surface_size, screen_width, screen_height);
    let slider_value = (cursor.x
        - (origin_x + i64::from(layout.x) + i64::from(SLIDER_HANDLE_HALF_WIDTH)) as f64)
        / f64::from(
            layout
                .width
                .saturating_sub(i32::try_from(SLIDER_HANDLE_WIDTH).unwrap_or(i32::MAX)),
        );
    input.set_dialog_number_range_slider_value(layout.index, slider_value)
}

pub(super) fn run_dialog_control_activation(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    activation: DialogControlActivation,
    after_action: &str,
) -> bool {
    match activation {
        DialogControlActivation::Action(action) => run_dialog_action(
            input,
            counters,
            world,
            net_commands,
            clipboard,
            action.as_ref(),
            after_action,
        ),
        DialogControlActivation::DialogListEntry(index) => {
            input.show_dialog_list_entry(world, index)
        }
    }
}

pub(super) fn run_dialog_component_click_event(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    event: &ComponentClickEvent,
    after_action: &str,
) -> bool {
    if input
        .current_dialog()
        .is_some_and(|dialog| dialog.is_resource_pack_prompt())
    {
        return false;
    }
    let action = match event {
        ComponentClickEvent::OpenFile { .. } => return false,
        ComponentClickEvent::OpenUrl { url } => DialogActionState::OpenUrl { url: url.clone() },
        ComponentClickEvent::RunCommand { command } => DialogActionState::RunCommand {
            command: command.clone(),
        },
        ComponentClickEvent::SuggestCommand { command } => DialogActionState::SuggestCommand {
            command: command.clone(),
        },
        ComponentClickEvent::ShowDialog { dialog } => {
            return input.show_dialog_from_component_click(world, dialog);
        }
        ComponentClickEvent::ChangePage { page } => DialogActionState::ChangePage { page: *page },
        ComponentClickEvent::CopyToClipboard { value } => DialogActionState::CopyToClipboard {
            value: value.clone(),
        },
        ComponentClickEvent::Custom { id, payload } => DialogActionState::Custom {
            id: id.clone(),
            payload: payload.clone(),
        },
    };
    run_dialog_action(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        Some(&action),
        after_action,
    )
}

pub(super) fn run_dialog_action(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    action: Option<&DialogActionState>,
    after_action: &str,
) -> bool {
    match action {
        None => {}
        Some(DialogActionState::ResourcePackConsent {
            accepted,
            latest_sequence,
        }) => {
            if !world.answer_resource_pack_prompt(*latest_sequence, *accepted) {
                return false;
            }
        }
        Some(DialogActionState::Custom { id, payload }) => {
            if let Some(tx) = net_commands {
                if tx
                    .try_send(NetCommand::CustomClickAction(CustomClickAction {
                        id: id.clone(),
                        payload: payload.clone(),
                    }))
                    .is_ok()
                {
                    counters.custom_click_action_commands_queued += 1;
                }
            }
        }
        Some(DialogActionState::RunCommand { command }) => {
            input.start_dialog_unattended_command(
                command,
                after_action,
                world,
                counters,
                net_commands,
            );
            return true;
        }
        Some(DialogActionState::DynamicRunCommand { template }) => {
            let Some(values) = bound_dialog_input_values(input, world) else {
                return false;
            };
            let command = instantiate_dialog_command_template(template, &values);
            input.start_dialog_unattended_command(
                &command,
                after_action,
                world,
                counters,
                net_commands,
            );
            return true;
        }
        Some(DialogActionState::DynamicCustom { id, additions }) => {
            let Some(values) = bound_dialog_input_values(input, world) else {
                return false;
            };
            let payload = dynamic_custom_payload(additions.as_ref(), &values);
            if let Some(tx) = net_commands {
                if tx
                    .try_send(NetCommand::CustomClickAction(CustomClickAction {
                        id: id.clone(),
                        payload: Some(payload),
                    }))
                    .is_ok()
                {
                    counters.custom_click_action_commands_queued += 1;
                }
            }
        }
        Some(DialogActionState::ShowDialog { dialog }) => {
            // Vanilla computes an active screen from `after_action`, then
            // `showDialog` unwraps an active Dialog/Waiting screen back to its
            // original previous screen. All three after-action variants
            // therefore replace the current dialog while retaining that owner.
            input.show_dialog((**dialog).clone());
            return true;
        }
        Some(DialogActionState::CopyToClipboard { value }) => {
            // KeyboardHandler.setClipboard intentionally ignores empty text.
            if !value.units().is_empty() {
                if let Some(clipboard) = clipboard {
                    clipboard.set_debug_clipboard_java_text(value);
                }
            }
        }
        Some(DialogActionState::OpenUrl { url }) => {
            return input.open_dialog_url(url.clone(), after_action.to_string());
        }
        Some(DialogActionState::ChangePage { .. }) => {
            // DialogScreen delegates this event to Screen.defaultHandleClickEvent,
            // which has no ChangePage arm. The BookViewScreen override is not
            // consulted, but the computed after-action screen is still applied.
        }
        Some(DialogActionState::SuggestCommand { .. }) => {
            // Screen.defaultHandleClickEvent calls `activeScreen.insertText` for
            // SuggestCommand. DialogScreen does not override Screen.insertText,
            // so the suggestion has no text side effect, but vanilla still
            // activates the screen computed from `after_action` below.
        }
    }
    match after_action {
        "close" => {
            let closed = input.close_dialog();
            if closed {
                input.refresh_dialog_parent_suggestions(world, counters, net_commands);
            }
            closed
        }
        "none" => true,
        "wait_for_response" => input.wait_for_dialog_response(),
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BoundDialogInputValue {
    pub(super) template_substitution: bbb_protocol::JavaString,
    pub(super) tag: NbtSummaryValue,
}

pub(super) fn bound_dialog_input_values(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
) -> Option<BTreeMap<String, BoundDialogInputValue>> {
    let controls = input
        .current_dialog()?
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))?;
    let values = input.current_dialog_input_values();
    let mut bound = BTreeMap::new();
    for (index, input) in controls.common.input_data.iter().enumerate() {
        let value = match (&input.control, values.get(index)?) {
            (
                DialogInputControlState::Boolean {
                    on_true, on_false, ..
                },
                Some(DialogInputValueState::Boolean { selected }),
            ) => BoundDialogInputValue {
                template_substitution: if *selected { on_true } else { on_false }.clone(),
                tag: NbtSummaryValue::Byte(i8::from(*selected)),
            },
            (
                DialogInputControlState::NumberRange { range, .. },
                Some(DialogInputValueState::NumberRange { slider_value_bits }),
            ) => {
                let value =
                    dialog_number_range_scaled_value(range, f64::from_bits(*slider_value_bits));
                BoundDialogInputValue {
                    template_substitution: dialog_number_range_value_string(value).into(),
                    tag: NbtSummaryValue::Float(value.to_bits()),
                }
            }
            (
                DialogInputControlState::SingleOption { options, .. },
                Some(DialogInputValueState::SingleOption { selected_index }),
            ) => {
                let selected = options.get(*selected_index)?;
                BoundDialogInputValue {
                    template_substitution: selected.id.clone(),
                    tag: selected.id.to_nbt(),
                }
            }
            (DialogInputControlState::Text { .. }, Some(DialogInputValueState::Text { value })) => {
                BoundDialogInputValue {
                    template_substitution: escape_dialog_text_template(value),
                    tag: value.to_nbt(),
                }
            }
            _ => return None,
        };
        bound.insert(input.key.clone(), value);
    }
    Some(bound)
}

/// Every escape this adds is ASCII, so the units it does not escape pass
/// through as they were typed.
pub(super) fn escape_dialog_text_template(
    value: impl Into<bbb_protocol::JavaString>,
) -> bbb_protocol::JavaString {
    let mut escaped = Vec::new();
    let value = value.into();
    for &unit in value.units() {
        match char::from_u32(u32::from(unit)) {
            Some(ch @ ('"' | '\'' | '\\')) => {
                escaped.push(u16::from(b'\\'));
                escaped.extend(ch.encode_utf16(&mut [0; 2]).iter().copied());
            }
            Some('\u{0008}') => escaped.extend("\\b".encode_utf16()),
            Some('\t') => escaped.extend("\\t".encode_utf16()),
            Some('\n') => escaped.extend("\\n".encode_utf16()),
            Some('\u{000c}') => escaped.extend("\\f".encode_utf16()),
            Some('\r') => escaped.extend("\\r".encode_utf16()),
            _ if unit < 0x20 => escaped.extend(format!("\\x{unit:02x}").encode_utf16()),
            _ => escaped.push(unit),
        }
    }
    bbb_protocol::JavaString::from(escaped)
}

pub(super) fn instantiate_dialog_command_template(
    template: &DialogCommandTemplateState,
    values: &BTreeMap<String, BoundDialogInputValue>,
) -> bbb_protocol::JavaString {
    let mut command = Vec::new();
    for (index, variable) in template.variables.iter().enumerate() {
        if let Some(segment) = template.segments.get(index) {
            command.extend_from_slice(segment.units());
        }
        if let Some(value) = values.get(variable) {
            command.extend_from_slice(value.template_substitution.units());
        }
    }
    if let Some(trailing) = template.segments.get(template.variables.len()) {
        command.extend_from_slice(trailing.units());
    }
    bbb_protocol::JavaString::from(command)
}

pub(super) fn dynamic_custom_payload(
    additions: Option<&NbtSummaryValue>,
    values: &BTreeMap<String, BoundDialogInputValue>,
) -> NbtSummaryValue {
    let mut fields = BTreeMap::new();
    if let Some(NbtSummaryValue::Compound(entries)) = additions {
        for entry in entries {
            fields.insert(entry.name.clone(), entry.value.clone());
        }
    }
    for (key, value) in values {
        fields.insert(key.clone().into(), value.tag.clone());
    }
    NbtSummaryValue::Compound(
        fields
            .into_iter()
            .map(|(name, value)| bbb_protocol::NbtSummaryEntry { name, value })
            .collect(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn screen_button_contains(
    screen_width: u32,
    screen_height: u32,
    button_x: i32,
    button_y: i32,
    button_width: u32,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let button_left = origin_x + i64::from(button_x);
    let button_top = origin_y + i64::from(button_y);
    cursor.x >= button_left as f64
        && cursor.x < (button_left + i64::from(button_width)) as f64
        && cursor.y >= button_top as f64
        && cursor.y < (button_top + i64::from(BUTTON_HEIGHT)) as f64
}
