use super::*;
use crate::input::dialog::DialogBodyFocusPartState;
use crate::input::dialog::DialogFocusState;
use crate::input::dialog::DialogInputValueState;

#[cfg(test)]
pub(crate) fn handle_dialog_key_input(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
) -> bool {
    handle_dialog_key_input_with_modifiers(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        physical_key,
        state,
        false,
        false,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_key_input_with_modifiers(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
    shift_down: bool,
    control_down: bool,
) -> bool {
    handle_dialog_key_input_with_modifiers_for_surface(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        physical_key,
        state,
        shift_down,
        control_down,
        PhysicalSize::new(320, 240),
    )
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn handle_dialog_key_input_with_modifiers_for_surface(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
    shift_down: bool,
    control_down: bool,
    surface_size: PhysicalSize<u32>,
) -> bool {
    handle_dialog_key_input_with_modifiers_for_surface_and_measurer(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        physical_key,
        state,
        shift_down,
        control_down,
        surface_size,
        &ApproximateDialogTextMeasurer,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_key_input_with_modifiers_for_surface_and_measurer(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    mut clipboard: Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    state: ElementState,
    shift_down: bool,
    control_down: bool,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if input.dialog_link_confirmation().is_some() {
        if state != ElementState::Pressed {
            return true;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        match code {
            KeyCode::Escape => {
                if input.resolve_dialog_link_confirmation(false) {
                    input.refresh_dialog_parent_suggestions(world, counters, net_commands);
                }
            }
            KeyCode::Tab => {
                let current = input.dialog_link_focused_button();
                let next = if shift_down {
                    current.map_or(2, |index| index.checked_sub(1).unwrap_or(2))
                } else {
                    current.map_or(0, |index| index.saturating_add(1) % 3)
                };
                input.focus_dialog_link_button(next);
            }
            KeyCode::ArrowLeft => {
                let next = input
                    .dialog_link_focused_button()
                    .map_or(2, |index| index.saturating_sub(1));
                input.focus_dialog_link_button(next);
            }
            KeyCode::ArrowRight => {
                let next = input
                    .dialog_link_focused_button()
                    .map_or(0, |index| index.saturating_add(1).min(2));
                input.focus_dialog_link_button(next);
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                if input.dialog_link_focused_button().is_none() {
                    input.focus_dialog_link_button(0);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(index) = input.dialog_link_focused_button() {
                    run_dialog_confirm_link_button(
                        input,
                        world,
                        counters,
                        net_commands,
                        &mut clipboard,
                        index,
                    );
                }
            }
            _ => {}
        }
        return true;
    }
    if input.dialog_warning_is_open() {
        if state != ElementState::Pressed {
            return true;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        match code {
            KeyCode::Escape => {
                if input.close_dialog_warning() {
                    input.refresh_dialog_parent_suggestions(world, counters, net_commands);
                }
            }
            KeyCode::Tab => {
                let current = input.dialog_warning_focused_button();
                let next = if shift_down {
                    match current {
                        None | Some(true) => false,
                        Some(false) => true,
                    }
                } else {
                    match current {
                        None | Some(false) => true,
                        Some(true) => false,
                    }
                };
                input.focus_dialog_warning_button(next);
            }
            KeyCode::ArrowLeft => match input.dialog_warning_focused_button() {
                None => {
                    input.focus_dialog_warning_button(false);
                }
                Some(false) => {
                    input.focus_dialog_warning_button(true);
                }
                Some(true) => {}
            },
            KeyCode::ArrowRight => match input.dialog_warning_focused_button() {
                None => {
                    input.focus_dialog_warning_button(true);
                }
                Some(true) => {
                    input.focus_dialog_warning_button(false);
                }
                Some(false) => {}
            },
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                if input.dialog_warning_focused_button().is_none() {
                    input.focus_dialog_warning_button(true);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(disconnect) = input.dialog_warning_focused_button() {
                    run_dialog_warning_button(world, counters, input, net_commands, disconnect);
                }
            }
            _ => {}
        }
        return true;
    }
    if input.dialog_waiting_ticks().is_some() {
        if state == ElementState::Pressed
            && physical_key == PhysicalKey::Code(KeyCode::Escape)
            && input.dialog_waiting_can_close()
            && input.close_dialog_waiting_response()
        {
            input.refresh_dialog_parent_suggestions(world, counters, net_commands);
        }
        return true;
    }
    if input.current_dialog().is_none() {
        return false;
    }
    if state != ElementState::Pressed {
        return true;
    }
    if handle_focused_dialog_text_key(
        input,
        world,
        &mut clipboard,
        physical_key,
        shift_down,
        control_down,
        text_measurer,
    ) {
        return true;
    }
    if handle_focused_dialog_input_key(input, world, physical_key, shift_down) {
        return true;
    }
    let PhysicalKey::Code(code) = physical_key else {
        return true;
    };
    if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) {
        match input.dialog_focus() {
            Some(DialogFocusState::WarningButton) => {
                input.open_dialog_warning();
                return true;
            }
            Some(DialogFocusState::Action { index }) => {
                let action = input
                    .current_dialog()
                    .and_then(|dialog| dialog.data.as_ref())
                    .and_then(|data| dialog_controls(data, world.server_links()))
                    .and_then(|controls| {
                        Some((
                            controls.actions.get(index)?.activation(),
                            controls.common.after_action.clone(),
                        ))
                    });
                if let Some((activation, after_action)) = action {
                    run_dialog_control_activation(
                        input,
                        counters,
                        world,
                        net_commands,
                        clipboard,
                        activation,
                        &after_action,
                    );
                }
                return true;
            }
            _ => {}
        }
    }
    let direction = match code {
        KeyCode::ArrowUp => Some(DialogFocusDirection::Up),
        KeyCode::ArrowDown => Some(DialogFocusDirection::Down),
        KeyCode::ArrowLeft => Some(DialogFocusDirection::Left),
        KeyCode::ArrowRight => Some(DialogFocusDirection::Right),
        _ => None,
    };
    if code == KeyCode::Tab || direction.is_some() {
        let text_ctx = DialogTextContext::from_measurer(None, text_measurer);
        let scroll_amount = input.dialog_body_scroll_amount();
        let next = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let content = dialog_content_layout_with(&controls, text_ctx)?;
                let layout = dialog_body_scroll_layout_with_content(
                    &controls,
                    &content,
                    surface_size,
                    scroll_amount,
                )?;
                dialog_focus_graph_with_content(
                    &controls,
                    &content,
                    surface_size,
                    layout.scroll_amount,
                    text_ctx,
                )
            })
            .and_then(|graph| {
                if code == KeyCode::Tab {
                    next_dialog_tab_focus(&graph, input.dialog_focus(), !shift_down)
                } else {
                    next_dialog_focus_in_direction(
                        &graph,
                        input.dialog_focus(),
                        direction.expect("direction checked above"),
                    )
                }
            });
        if let Some(target) = next {
            input.focus_dialog_navigation_target(world, target, code == KeyCode::Tab);
            reveal_dialog_keyboard_focus_with(input, world, target, surface_size, text_ctx);
        }
        return true;
    }
    if physical_key != PhysicalKey::Code(KeyCode::Escape) {
        return true;
    }
    let Some(controls) = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
    else {
        return true;
    };
    if !controls.common.can_close_with_escape {
        return true;
    }
    let action = controls
        .cancel
        .and_then(|cancel| cancel.action_data.clone());
    run_dialog_action(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        action.as_ref(),
        "close",
    );
    true
}

#[cfg(test)]
pub(super) fn reveal_dialog_keyboard_focus(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    target: DialogFocusState,
    surface_size: PhysicalSize<u32>,
) -> bool {
    reveal_dialog_keyboard_focus_with(
        input,
        world,
        target,
        surface_size,
        DialogTextContext::approximate(None),
    )
}

pub(super) fn reveal_dialog_keyboard_focus_with(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    target: DialogFocusState,
    surface_size: PhysicalSize<u32>,
    text_ctx: DialogTextContext<'_>,
) -> bool {
    let scroll_amount = input.dialog_body_scroll_amount();
    let Some((layout, focused)) = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .and_then(|controls| {
            let content = dialog_content_layout_with(&controls, text_ctx)?;
            let layout = dialog_body_scroll_layout_with_content(
                &controls,
                &content,
                surface_size,
                scroll_amount,
            )?;
            let graph = dialog_focus_graph_with_content(
                &controls,
                &content,
                surface_size,
                layout.scroll_amount,
                text_ctx,
            )?;
            let focused = dialog_focus_node(&graph.body, target)?.rectangle;
            Some((layout, focused))
        })
    else {
        return false;
    };
    let top_delta = focused.y.saturating_sub(layout.rectangle.y);
    let bottom_delta = focused
        .y
        .saturating_add(focused.height)
        .saturating_sub(layout.rectangle.y.saturating_add(layout.rectangle.height));
    let next = if top_delta < 0 {
        scroll_amount + f64::from(top_delta) - 10.0
    } else if bottom_delta > 0 {
        scroll_amount + f64::from(bottom_delta) + 10.0
    } else {
        return false;
    };
    input.set_dialog_body_scroll_amount(next, f64::from(layout.max_scroll))
}

pub(super) fn handle_focused_dialog_input_key(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    physical_key: PhysicalKey,
    shift_down: bool,
) -> bool {
    let PhysicalKey::Code(code) = physical_key else {
        return false;
    };
    let Some(index) = input.dialog_focused_input() else {
        return false;
    };
    let control = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .and_then(|controls| controls.common.input_data.get(index))
        .map(|definition| definition.control.clone());
    let selection = matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space);
    match control {
        Some(DialogInputControlState::Boolean { .. }) if selection => {
            input.toggle_dialog_boolean_input(index);
            true
        }
        Some(DialogInputControlState::SingleOption { .. }) if selection => {
            cycle_dialog_single_option(input, world, index, shift_down);
            true
        }
        Some(DialogInputControlState::NumberRange { .. }) if selection => {
            input.toggle_dialog_number_range_keyboard_change(index);
            true
        }
        Some(DialogInputControlState::NumberRange { width, .. })
            if input.dialog_number_range_keyboard_can_change(index)
                && matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight) =>
        {
            let Some(Some(DialogInputValueState::NumberRange { slider_value_bits })) =
                input.current_dialog_input_values().get(index)
            else {
                return true;
            };
            let direction = if code == KeyCode::ArrowLeft {
                -1.0
            } else {
                1.0
            };
            let value =
                f64::from_bits(*slider_value_bits) + direction / f64::from(width.saturating_sub(8));
            input.set_dialog_number_range_slider_value(index, value);
            true
        }
        _ => false,
    }
}

pub(super) fn handle_focused_dialog_text_key(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    clipboard: &mut Option<&mut dyn DebugClipboard>,
    physical_key: PhysicalKey,
    shift_down: bool,
    control_down: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let PhysicalKey::Code(code) = physical_key else {
        return false;
    };
    if focused_dialog_text_snapshot(input, world).is_none() {
        return false;
    }
    let modifiers = TextFieldModifiers {
        shift: shift_down,
        word: control_down,
        shortcut: control_down,
    };
    edit_focused_dialog_text(input, world, text_measurer, |field| match field {
        DialogTextField::Single(field) => field.key(code, modifiers, clipboard, None),
        DialogTextField::Multiline(field, font) => field.key(code, modifiers, clipboard, font),
    })
    .flatten()
    .is_some()
}

/// The focused dialog input as Vanilla's `EditBox` or `MultiLineEditBox`.
pub(super) enum DialogTextField<'a> {
    Single(&'a mut TextField),
    Multiline(&'a mut MultilineTextField, &'a DialogTextFont<'a>),
}

pub(super) struct DialogTextFont<'a> {
    inner_width: u32,
    measurer: &'a dyn DialogTextMeasurer,
}

impl TextLineFont for DialogTextFont<'_> {
    fn lines(&self, value: &[u16]) -> Vec<std::ops::Range<usize>> {
        dialog_multiline_lines(value.to_vec(), self.inner_width, self.measurer)
            .into_iter()
            .map(|line| line.begin..line.end)
            .collect()
    }

    fn width(&self, units: &[u16]) -> i32 {
        self.measurer
            .width(&bbb_protocol::JavaString::from(units.to_vec()).to_display_string())
    }

    fn head_len(&self, units: &[u16], width: i32) -> usize {
        let value = bbb_protocol::JavaString::from(units.to_vec());
        DialogTextRaw::new(&value)
            .cursor_for_width(0, width, self.measurer)
            .min(units.len())
    }
}

/// Runs `edit` on the focused input and writes value and selection back.
pub(super) fn edit_focused_dialog_text<R>(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    text_measurer: &dyn DialogTextMeasurer,
    edit: impl FnOnce(DialogTextField<'_>) -> R,
) -> Option<R> {
    let snapshot = focused_dialog_text_snapshot(input, world)?;
    let units = snapshot.value.units();
    let (result, value, cursor, selection) = if let Some(multiline) = &snapshot.multiline {
        let font = DialogTextFont {
            inner_width: snapshot.inner_width(),
            measurer: text_measurer,
        };
        let mut field = MultilineTextField::new(
            units,
            snapshot.cursor,
            snapshot.selection,
            snapshot.max_length,
            multiline
                .max_lines
                .and_then(|max_lines| usize::try_from(max_lines).ok()),
        );
        let result = edit(DialogTextField::Multiline(&mut field, &font));
        (
            result,
            field.value().to_vec(),
            field.cursor(),
            field.selection(),
        )
    } else {
        let mut field = TextField::from_parts(
            units,
            snapshot.cursor,
            snapshot.selection,
            0,
            snapshot.max_length,
        );
        let result = edit(DialogTextField::Single(&mut field));
        (
            result,
            field.value().to_vec(),
            field.cursor(),
            field.selection(),
        )
    };
    if (value.as_slice(), cursor, selection) != (units, snapshot.cursor, snapshot.selection) {
        set_focused_dialog_text(
            input,
            &snapshot,
            value.into(),
            cursor,
            selection,
            text_measurer,
        );
    }
    Some(result)
}

#[derive(Debug, Clone)]
pub(super) struct FocusedDialogTextSnapshot {
    pub(super) index: usize,
    pub(super) value: bbb_protocol::JavaString,
    pub(super) cursor: usize,
    pub(super) selection: usize,
    pub(super) scroll_amount: i32,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) max_length: usize,
    pub(super) multiline: Option<DialogTextMultilineState>,
}

impl FocusedDialogTextSnapshot {
    pub(super) fn inner_width(&self) -> u32 {
        u32::try_from(
            self.width
                .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                .max(1),
        )
        .unwrap_or(1)
    }
}

pub(super) fn focused_dialog_text_snapshot(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
) -> Option<FocusedDialogTextSnapshot> {
    let edit = input.dialog_text_edit_state()?;
    let Some(Some(DialogInputValueState::Text { value })) =
        input.current_dialog_input_values().get(edit.input_index)
    else {
        return None;
    };
    let (width, max_length, multiline) = input
        .current_dialog()?
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))?
        .common
        .input_data
        .get(edit.input_index)
        .and_then(|definition| match &definition.control {
            DialogInputControlState::Text {
                width,
                max_length,
                multiline,
                ..
            } => Some((
                *width,
                usize::try_from(*max_length).ok()?,
                multiline.clone(),
            )),
            _ => None,
        })?;
    let height = multiline.as_ref().map_or(TEXT_INPUT_HEIGHT, |multiline| {
        multiline.height.unwrap_or_else(|| {
            multiline
                .max_lines
                .unwrap_or(4)
                .saturating_mul(TEXT_LINE_HEIGHT)
                .saturating_add(8)
                .min(512)
        })
    });
    Some(FocusedDialogTextSnapshot {
        index: edit.input_index,
        value: value.clone(),
        cursor: edit.cursor,
        selection: edit.selection,
        scroll_amount: edit.scroll_amount,
        width,
        height,
        max_length,
        multiline,
    })
}

pub(super) fn set_focused_dialog_text(
    input: &mut crate::input::ClientInputState,
    snapshot: &FocusedDialogTextSnapshot,
    value: bbb_protocol::JavaString,
    cursor: usize,
    selection: usize,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let lines = snapshot
        .multiline
        .as_ref()
        .map(|_| dialog_multiline_lines(&value, snapshot.inner_width(), text_measurer));
    if !input.set_dialog_text_input_raw_value_and_selection(
        snapshot.index,
        value,
        cursor,
        selection,
    ) {
        return false;
    }
    if let Some(lines) = lines {
        let max_scroll = dialog_multiline_max_scroll(lines.len(), snapshot.height);
        let scroll_amount =
            dialog_multiline_reveal_cursor(snapshot.scroll_amount, snapshot.height, &lines, cursor);
        input.set_dialog_multiline_text_scroll_amount(snapshot.index, scroll_amount, max_scroll);
    }
    true
}

pub(super) fn dialog_char_slice(
    text: impl Into<bbb_protocol::JavaString>,
    begin: usize,
    end: usize,
) -> String {
    DialogTextRaw::new(&text.into()).slice(begin, end)
}

#[cfg(test)]
pub(crate) fn handle_dialog_text_input(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    text: &str,
) -> bool {
    handle_dialog_text_input_with_measurer(input, world, text, &ApproximateDialogTextMeasurer)
}

pub(crate) fn handle_dialog_text_input_with_measurer(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    text: &str,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if input.dialog_link_confirmation().is_some() || input.dialog_warning_is_open() {
        return true;
    }
    if input.dialog_text_edit_state().is_none() {
        return input.dialog_screen_is_open();
    }
    edit_focused_dialog_text(input, world, text_measurer, |field| match field {
        DialogTextField::Single(field) => field.type_text(text, None),
        DialogTextField::Multiline(field, font) => field.type_text(text, font),
    });
    true
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_mouse_input(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    handle_dialog_mouse_input_with_modifiers_and_measurer(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        button,
        state,
        cursor_position,
        surface_size,
        false,
        false,
        &ApproximateDialogTextMeasurer,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_mouse_input_with_modifiers(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    shift_down: bool,
) -> bool {
    handle_dialog_mouse_input_with_modifiers_and_measurer(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        button,
        state,
        cursor_position,
        surface_size,
        shift_down,
        false,
        &ApproximateDialogTextMeasurer,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_dialog_mouse_input_with_modifiers_and_measurer(
    input: &mut crate::input::ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    clipboard: Option<&mut dyn DebugClipboard>,
    button: MouseButton,
    state: ElementState,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    shift_down: bool,
    double_click: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if button == MouseButton::Left && state == ElementState::Released {
        input.end_dialog_number_range_drag();
        input.end_dialog_text_input_drag();
        input.end_dialog_multiline_scroll_drag();
        input.end_dialog_body_scroll_drag();
        return input.dialog_screen_is_open();
    }
    let text_ctx = DialogTextContext::from_measurer(None, text_measurer);
    if let Some(confirmation) = input.dialog_link_confirmation() {
        if button == MouseButton::Left && state == ElementState::Pressed {
            let message_width = surface_size
                .width
                .max(1)
                .saturating_sub(CONFIRM_MESSAGE_HORIZONTAL_PADDING)
                .max(1);
            let message_line_count = plain_dialog_text_lines_with(
                &confirmation.url.to_display_string(),
                message_width,
                text_ctx.glyphs,
            )
            .len()
            .min(CONFIRM_MESSAGE_MAX_ROWS);
            if let Some(index) = DialogConfirmLinkLayout::new(message_line_count, surface_size)
                .button_at(cursor_position, surface_size)
            {
                input.focus_dialog_link_button(index);
                let mut clipboard = clipboard;
                run_dialog_confirm_link_button(
                    input,
                    world,
                    counters,
                    net_commands,
                    &mut clipboard,
                    index,
                );
            }
        }
        return true;
    }
    if input.dialog_warning_is_open() {
        if button == MouseButton::Left && state == ElementState::Pressed {
            let message_line_count =
                dialog_warning_message_lines_with(surface_size, text_ctx.glyphs).len();
            if let Some(disconnect) =
                DialogWarningScreenLayout::new(message_line_count, surface_size)
                    .button_at(cursor_position, surface_size)
            {
                input.focus_dialog_warning_button(disconnect);
                run_dialog_warning_button(world, counters, input, net_commands, disconnect);
            }
        }
        return true;
    }
    if input.dialog_waiting_ticks().is_some() {
        if button == MouseButton::Left
            && state == ElementState::Pressed
            && input.dialog_waiting_can_close()
            && WaitingLayout::new(surface_size).button_contains(cursor_position, surface_size)
            && input.close_dialog_waiting_response()
        {
            input.refresh_dialog_parent_suggestions(world, counters, net_commands);
        }
        return true;
    }
    if button != MouseButton::Left || state != ElementState::Pressed {
        return input.current_dialog().is_some();
    }
    input.end_dialog_number_range_drag();
    input.end_dialog_text_input_drag();
    input.end_dialog_multiline_scroll_drag();
    input.end_dialog_body_scroll_drag();
    let Some(dialog) = input.current_dialog() else {
        return false;
    };
    let Some(controls) = dialog
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))
    else {
        return true;
    };
    if controls.show_server_dialog_warning()
        && DialogWarningButtonLayout::new_with(&controls.common.title, surface_size, text_ctx)
            .is_some_and(|layout| layout.contains(cursor_position, surface_size))
    {
        input.focus_dialog_navigation_target(world, DialogFocusState::WarningButton, false);
        input.open_dialog_warning();
        return true;
    }
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let Some(content) = dialog_content_layout_with(&controls, text_ctx) else {
        return true;
    };
    let scroll_layout = dialog_body_scroll_layout_with_content(
        &controls,
        &content,
        surface_size,
        input.dialog_body_scroll_amount(),
    );
    if let Some(cursor_y) = scroll_layout.and_then(|scroll_layout| {
        let scrollbar = dialog_scrollbar_layout(scroll_layout)?;
        dialog_scrollbar_contains(
            scrollbar,
            cursor_position,
            surface_size,
            screen_width,
            screen_height,
        )
        .then_some(())?;
        dialog_cursor_screen_y(cursor_position, surface_size, screen_height)
    }) {
        input.begin_dialog_body_scroll_drag(cursor_y);
        return true;
    }
    let Some(scroll_layout) = scroll_layout else {
        return true;
    };
    let cursor_in_body = dialog_body_viewport_contains(
        scroll_layout,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    );
    let input_hit = dialog_input_layouts_with_content(
        &controls,
        &content,
        surface_size,
        scroll_layout.scroll_amount,
    )
    .and_then(|layouts| {
        layouts.into_iter().find_map(|layout| {
            let control = &controls.common.input_data.get(layout.index)?.control;
            let hit_layout = match control {
                DialogInputControlState::Text {
                    label_visible,
                    multiline: None,
                    ..
                } => dialog_single_line_text_box_layout(layout, *label_visible),
                DialogInputControlState::Text {
                    label_visible,
                    multiline: Some(_),
                    ..
                } => dialog_multiline_text_box_layout(layout, *label_visible),
                _ => layout,
            };
            let hit_width = if matches!(
                control,
                DialogInputControlState::Text {
                    multiline: Some(_),
                    ..
                }
            ) {
                hit_layout.width.saturating_add(6)
            } else {
                hit_layout.width
            };
            if !cursor_in_body
                || !dialog_widget_contains(
                    hit_layout.x,
                    hit_layout.y,
                    hit_width,
                    hit_layout.height,
                    cursor_position,
                    surface_size,
                    screen_width,
                    screen_height,
                )
            {
                return None;
            }
            match control {
                DialogInputControlState::Boolean { .. } => {
                    Some(DialogInputMouseHit::Boolean(layout.index))
                }
                DialogInputControlState::NumberRange { .. } => {
                    Some(DialogInputMouseHit::NumberRange(layout))
                }
                DialogInputControlState::SingleOption { .. } => {
                    Some(DialogInputMouseHit::SingleOption(layout))
                }
                DialogInputControlState::Text {
                    multiline: None, ..
                } => Some(DialogInputMouseHit::Text {
                    layout: hit_layout,
                    multiline: false,
                }),
                DialogInputControlState::Text {
                    multiline: Some(_), ..
                } => Some(DialogInputMouseHit::Text {
                    layout: hit_layout,
                    multiline: true,
                }),
            }
        })
    });
    let body_navigation_hit = dialog_focus_graph_with_content(
        &controls,
        &content,
        surface_size,
        scroll_layout.scroll_amount,
        text_ctx,
    )
    .and_then(|graph| {
        graph.body.into_iter().find(|node| {
            cursor_in_body
                && dialog_widget_contains(
                    node.rectangle.x,
                    node.rectangle.y,
                    node.rectangle.width,
                    node.rectangle.height,
                    cursor_position,
                    surface_size,
                    screen_width,
                    screen_height,
                )
        })
    });
    let footer_action = DialogFooterLayout::new(
        &controls
            .footer_actions()
            .iter()
            .map(|action| action.width())
            .collect::<Vec<_>>(),
        controls.footer_height,
        surface_size,
    )
    .and_then(|layout| layout.button_at(cursor_position, surface_size))
    .and_then(|footer_index| {
        let action_index = controls.body_action_count.saturating_add(footer_index);
        Some((
            action_index,
            controls.actions.get(action_index)?.activation(),
            controls.common.after_action.clone(),
        ))
    });
    match input_hit {
        Some(DialogInputMouseHit::Boolean(index)) => {
            input.focus_dialog_mouse_input(index);
            input.toggle_dialog_boolean_input(index);
            return true;
        }
        Some(DialogInputMouseHit::NumberRange(layout)) => {
            input.focus_dialog_mouse_input(layout.index);
            input.begin_dialog_number_range_drag(layout.index);
            set_dialog_number_range_from_cursor(input, layout, cursor_position, surface_size);
            return true;
        }
        Some(DialogInputMouseHit::SingleOption(layout)) => {
            input.focus_dialog_mouse_input(layout.index);
            cycle_dialog_single_option(input, world, layout.index, shift_down);
            return true;
        }
        Some(DialogInputMouseHit::Text { layout, multiline }) => {
            let scrollbar_drag = multiline
                && begin_dialog_multiline_scroll_drag_from_cursor(
                    input,
                    world,
                    layout,
                    cursor_position,
                    surface_size,
                    text_measurer,
                );
            if !scrollbar_drag {
                if double_click {
                    if multiline {
                        input.focus_dialog_navigation_target(
                            world,
                            DialogFocusState::Input {
                                index: layout.index,
                            },
                            false,
                        );
                    } else {
                        focus_dialog_text_from_cursor(
                            input,
                            layout,
                            cursor_position,
                            surface_size,
                            false,
                            text_measurer,
                        );
                    }
                    select_focused_dialog_text_word(input, world, text_measurer);
                } else {
                    focus_dialog_text_from_cursor(
                        input,
                        layout,
                        cursor_position,
                        surface_size,
                        shift_down,
                        text_measurer,
                    );
                }
                input.begin_dialog_text_input_drag(layout.index);
            }
            return true;
        }
        None => {}
    }
    if let Some(node) = body_navigation_hit {
        let target = node.target;
        let action = match target {
            DialogFocusState::Action { index } => controls
                .actions
                .get(index)
                .map(|action| (action.activation(), controls.common.after_action.clone())),
            _ => None,
        };
        let component_click = match target {
            DialogFocusState::Body {
                index,
                part: DialogBodyFocusPartState::Primary,
            } => match controls.common.body_data.get(index) {
                Some(DialogBodyState::PlainMessage { message }) => dialog_text_click_event_at_with(
                    message,
                    node.rectangle,
                    cursor_position,
                    surface_size,
                    text_ctx,
                ),
                _ => None,
            },
            DialogFocusState::Body {
                index,
                part: DialogBodyFocusPartState::Description,
            } => match controls.common.body_data.get(index) {
                Some(DialogBodyState::Item {
                    description: Some(description),
                    ..
                }) => dialog_text_click_event_at_with(
                    description,
                    node.rectangle,
                    cursor_position,
                    surface_size,
                    text_ctx,
                ),
                _ => None,
            },
            _ => None,
        };
        let component_after_action = controls.common.after_action.clone();
        input.focus_dialog_navigation_target(world, target, false);
        if let Some(event) = component_click {
            run_dialog_component_click_event(
                input,
                counters,
                world,
                net_commands,
                clipboard,
                &event,
                &component_after_action,
            );
        } else if let Some((activation, after_action)) = action {
            run_dialog_control_activation(
                input,
                counters,
                world,
                net_commands,
                clipboard,
                activation,
                &after_action,
            );
        }
        return true;
    }
    let Some((action_index, action, after_action)) = footer_action else {
        return true;
    };
    input.focus_dialog_navigation_target(
        world,
        DialogFocusState::Action {
            index: action_index,
        },
        false,
    );
    run_dialog_control_activation(
        input,
        counters,
        world,
        net_commands,
        clipboard,
        action,
        &after_action,
    );
    true
}

pub(super) fn select_focused_dialog_text_word(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    edit_focused_dialog_text(input, world, text_measurer, |field| match field {
        DialogTextField::Single(field) => {
            // EditBox.selectWord with the click already at `cursor`.
            let start = field.word_position(field.cursor(), -1);
            let end = field.word_position(field.cursor(), 1);
            field.move_cursor(end, false, None);
            field.set_selection(start, None);
        }
        DialogTextField::Multiline(field, _) => field.select_word_at_cursor(),
    })
    .is_some()
}

pub(super) fn focus_dialog_text_from_cursor(
    input: &mut crate::input::ClientInputState,
    layout: DialogInputLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    extend_selection: bool,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(cursor_position) = cursor_position else {
        return false;
    };
    let Some(Some(DialogInputValueState::Text { value })) =
        input.current_dialog_input_values().get(layout.index)
    else {
        return false;
    };
    let value = value.clone();
    let multiline = input.dialog_multiline_text_settings(layout.index).is_some();
    let text_len = value.units().len();
    if multiline {
        let inner_width = layout
            .width
            .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
            .max(1);
        let lines = dialog_multiline_lines(&value, inner_width.max(0) as u32, text_measurer);
        let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
        let scroll_amount = input
            .dialog_text_edit_state()
            .filter(|edit| edit.input_index == layout.index)
            .map_or(max_scroll, |edit| edit.scroll_amount.clamp(0, max_scroll));
        let screen_width = surface_size.width.max(1);
        let screen_height = surface_size.height.max(1);
        let (origin_x, origin_y) =
            centered_screen_origin(surface_size, screen_width, screen_height);
        let relative_x = (cursor_position.x.floor()
            - (origin_x + i64::from(layout.x) + i64::from(TEXT_INPUT_INSET)) as f64)
            as i32;
        let relative_y = (cursor_position.y.floor()
            - (origin_y + i64::from(layout.y) + i64::from(TEXT_INPUT_INSET)) as f64)
            as i32;
        let line_index = usize::try_from(
            relative_y
                .saturating_add(scroll_amount)
                .max(0)
                .saturating_div(TEXT_LINE_HEIGHT),
        )
        .unwrap_or(0)
        .min(lines.len().saturating_sub(1));
        let line = lines[line_index];
        let line_value =
            bbb_protocol::JavaString::from(value.units()[line.begin..line.end].to_vec());
        let line_text = DialogTextRaw::new(&line_value);
        let cursor = line.begin.saturating_add(
            line_text
                .cursor_for_width(0, relative_x, text_measurer)
                .min(line.end.saturating_sub(line.begin)),
        );
        let focused = input.focus_dialog_text_input(layout.index, cursor, extend_selection);
        let scroll_amount =
            dialog_multiline_reveal_cursor(scroll_amount, layout.height, &lines, cursor);
        input.set_dialog_multiline_text_scroll_amount(layout.index, scroll_amount, max_scroll);
        return focused;
    }
    let scroll_to = input
        .dialog_text_edit_state()
        .filter(|edit| edit.input_index == layout.index)
        .map_or(text_len, |edit| {
            if edit.selection != edit.cursor {
                edit.selection
            } else {
                edit.cursor
            }
        });
    let inner_width = layout
        .width
        .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
        .max(1);
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let (origin_x, _) = centered_screen_origin(surface_size, screen_width, screen_height);
    let text_x = origin_x + i64::from(layout.x) + i64::from(TEXT_INPUT_INSET);
    let position_in_text =
        (cursor_position.x.floor() - text_x as f64).min(f64::from(inner_width)) as i32;
    let projection = DialogTextRaw::new(&value);
    let display_start = projection.display_start(scroll_to, inner_width, text_measurer);
    let cursor = projection.cursor_for_width(display_start, position_in_text, text_measurer);
    input.focus_dialog_text_input(layout.index, cursor, extend_selection)
}

pub(super) fn begin_dialog_multiline_scroll_drag_from_cursor(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    layout: DialogInputLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    let Some(Some(DialogInputValueState::Text { value })) =
        input.current_dialog_input_values().get(layout.index)
    else {
        return false;
    };
    let lines = dialog_multiline_lines(
        value,
        u32::try_from(
            layout
                .width
                .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                .max(1),
        )
        .unwrap_or(1),
        text_measurer,
    );
    let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
    let current_scroll = input
        .dialog_text_edit_state()
        .filter(|edit| edit.input_index == layout.index)
        .map_or(max_scroll, |edit| edit.scroll_amount.clamp(0, max_scroll));
    let Some(scrollbar) = dialog_multiline_scrollbar_layout(layout, lines.len(), current_scroll)
    else {
        return false;
    };
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    if !dialog_scrollbar_contains(
        scrollbar,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    ) {
        return false;
    }
    let Some(cursor_y) = dialog_cursor_screen_y(cursor_position, surface_size, screen_height)
    else {
        return false;
    };
    input.focus_dialog_navigation_target(
        world,
        DialogFocusState::Input {
            index: layout.index,
        },
        false,
    );
    let travel = scrollbar
        .height
        .saturating_sub(scrollbar.thumb_height)
        .max(1);
    let next = cursor_y
        .saturating_sub(scrollbar.y)
        .saturating_sub(scrollbar.thumb_height / 2)
        .clamp(0, travel)
        .saturating_mul(max_scroll)
        .checked_div(travel)
        .unwrap_or(0);
    input.set_dialog_multiline_text_scroll_amount(layout.index, next, max_scroll);
    input.begin_dialog_multiline_scroll_drag(layout.index, cursor_y)
}

pub(super) fn cycle_dialog_single_option(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    index: usize,
    reverse: bool,
) -> bool {
    let (selected_index, option_count) = {
        let Some(dialog) = input.current_dialog() else {
            return false;
        };
        let Some(controls) = dialog
            .data
            .as_ref()
            .and_then(|data| dialog_controls(data, world.server_links()))
        else {
            return false;
        };
        let Some(definition) = controls.common.input_data.get(index) else {
            return false;
        };
        let DialogInputControlState::SingleOption { options, .. } = &definition.control else {
            return false;
        };
        let Some(Some(DialogInputValueState::SingleOption { selected_index })) =
            input.current_dialog_input_values().get(index)
        else {
            return false;
        };
        (*selected_index, options.len())
    };
    if option_count == 0 {
        return false;
    }
    let selected_index = if reverse {
        selected_index
            .checked_sub(1)
            .unwrap_or(option_count.saturating_sub(1))
    } else {
        selected_index.saturating_add(1) % option_count
    };
    input.set_dialog_single_option_index(index, selected_index)
}

pub(crate) fn handle_dialog_mouse_wheel(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    delta: MouseScrollDelta,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    if input.dialog_link_confirmation().is_some() || input.dialog_warning_is_open() {
        return true;
    }
    if input.dialog_waiting_ticks().is_some() {
        return true;
    }
    let Some(dialog) = input.current_dialog() else {
        return false;
    };
    let Some(controls) = dialog
        .data
        .as_ref()
        .and_then(|data| dialog_controls(data, world.server_links()))
    else {
        return true;
    };
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let Some(content) = dialog_content_layout_with(&controls, DialogTextContext::approximate(None))
    else {
        return true;
    };
    let Some(scroll_layout) = dialog_body_scroll_layout_with_content(
        &controls,
        &content,
        surface_size,
        input.dialog_body_scroll_amount(),
    ) else {
        return true;
    };
    if !dialog_body_viewport_contains(
        scroll_layout,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    ) {
        return true;
    }
    let scroll_pixels = match delta {
        MouseScrollDelta::LineDelta(_, y) => f64::from(y) * 10.0,
        MouseScrollDelta::PixelDelta(position) => position.y,
    };
    input.set_dialog_body_scroll_amount(
        input.dialog_body_scroll_amount() - scroll_pixels,
        f64::from(scroll_layout.max_scroll),
    );
    true
}

#[cfg(test)]
pub(crate) fn handle_dialog_cursor_moved(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> bool {
    handle_dialog_cursor_moved_with_measurer(
        input,
        world,
        cursor_position,
        surface_size,
        &ApproximateDialogTextMeasurer,
    )
}

pub(crate) fn handle_dialog_cursor_moved_with_measurer(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> bool {
    if input.dialog_link_confirmation().is_some() || input.dialog_warning_is_open() {
        input.end_dialog_number_range_drag();
        input.end_dialog_text_input_drag();
        input.end_dialog_multiline_scroll_drag();
        input.end_dialog_body_scroll_drag();
        return true;
    }
    if input.dialog_waiting_ticks().is_some() {
        input.end_dialog_number_range_drag();
        input.end_dialog_text_input_drag();
        input.end_dialog_multiline_scroll_drag();
        input.end_dialog_body_scroll_drag();
        return true;
    }
    let text_ctx = DialogTextContext::from_measurer(None, text_measurer);
    if let Some(index) = input.dialog_dragging_multiline_scroll_input() {
        let state = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let content = dialog_content_layout_with(&controls, text_ctx)?;
                let scroll_layout = dialog_body_scroll_layout_with_content(
                    &controls,
                    &content,
                    surface_size,
                    input.dialog_body_scroll_amount(),
                )?;
                let layout = dialog_input_layouts_with_content(
                    &controls,
                    &content,
                    surface_size,
                    scroll_layout.scroll_amount,
                )?
                .into_iter()
                .find(|layout| layout.index == index)?;
                let label_visible = match &controls.common.input_data.get(index)?.control {
                    DialogInputControlState::Text {
                        label_visible,
                        multiline: Some(_),
                        ..
                    } => *label_visible,
                    _ => return None,
                };
                let layout = dialog_multiline_text_box_layout(layout, label_visible);
                let value = match input.current_dialog_input_values().get(index) {
                    Some(Some(DialogInputValueState::Text { value })) => value,
                    _ => return None,
                };
                let lines = dialog_multiline_lines(
                    value,
                    u32::try_from(
                        layout
                            .width
                            .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                            .max(1),
                    )
                    .unwrap_or(1),
                    text_measurer,
                );
                let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
                let current = input
                    .dialog_text_edit_state()
                    .filter(|edit| edit.input_index == index)?
                    .scroll_amount
                    .clamp(0, max_scroll);
                let scrollbar = dialog_multiline_scrollbar_layout(layout, lines.len(), current)?;
                let cursor_y = dialog_cursor_screen_y(
                    cursor_position,
                    surface_size,
                    surface_size.height.max(1),
                )?;
                Some((scrollbar, max_scroll, current, cursor_y))
            });
        let Some((scrollbar, max_scroll, current, cursor_y)) = state else {
            input.end_dialog_multiline_scroll_drag();
            return input.current_dialog().is_some();
        };
        let delta_y = input
            .update_dialog_multiline_scroll_drag_y(cursor_y)
            .unwrap_or_default();
        let travel = scrollbar
            .height
            .saturating_sub(scrollbar.thumb_height)
            .max(1);
        let next = if cursor_y < scrollbar.y {
            0
        } else if cursor_y > scrollbar.y.saturating_add(scrollbar.height) {
            max_scroll
        } else {
            current.saturating_add(
                delta_y
                    .saturating_mul(max_scroll.max(1))
                    .checked_div(travel)
                    .unwrap_or(0),
            )
        };
        input.set_dialog_multiline_text_scroll_amount(index, next, max_scroll);
        return true;
    }
    if input.dialog_body_scroll_drag_y().is_some() {
        let screen_height = surface_size.height.max(1);
        let Some((layout, scrollbar, cursor_y)) = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let content = dialog_content_layout_with(&controls, text_ctx)?;
                let layout = dialog_body_scroll_layout_with_content(
                    &controls,
                    &content,
                    surface_size,
                    input.dialog_body_scroll_amount(),
                )?;
                let scrollbar = dialog_scrollbar_layout(layout)?;
                let cursor_y =
                    dialog_cursor_screen_y(cursor_position, surface_size, screen_height)?;
                Some((layout, scrollbar, cursor_y))
            })
        else {
            input.end_dialog_body_scroll_drag();
            return input.current_dialog().is_some();
        };
        let delta_y = input
            .update_dialog_body_scroll_drag_y(cursor_y)
            .unwrap_or_default();
        let next = if cursor_y < layout.rectangle.y {
            0.0
        } else if cursor_y > layout.rectangle.y.saturating_add(layout.rectangle.height) {
            f64::from(layout.max_scroll)
        } else {
            let travel = layout
                .rectangle
                .height
                .saturating_sub(scrollbar.thumb_height);
            let scale = (f64::from(layout.max_scroll.max(1)) / f64::from(travel.max(1))).max(1.0);
            input.dialog_body_scroll_amount() + f64::from(delta_y) * scale
        };
        input.set_dialog_body_scroll_amount(next, f64::from(layout.max_scroll));
        return true;
    }
    if let Some(index) = input.dialog_dragging_text_input() {
        let layout = input
            .current_dialog()
            .and_then(|dialog| dialog.data.as_ref())
            .and_then(|data| dialog_controls(data, world.server_links()))
            .and_then(|controls| {
                let content = dialog_content_layout_with(&controls, text_ctx)?;
                let scroll_layout = dialog_body_scroll_layout_with_content(
                    &controls,
                    &content,
                    surface_size,
                    input.dialog_body_scroll_amount(),
                )?;
                let layouts = dialog_input_layouts_with_content(
                    &controls,
                    &content,
                    surface_size,
                    scroll_layout.scroll_amount,
                )?;
                let layout = layouts.into_iter().find(|layout| layout.index == index)?;
                match &controls.common.input_data.get(index)?.control {
                    DialogInputControlState::Text {
                        label_visible,
                        multiline: None,
                        ..
                    } => Some(dialog_single_line_text_box_layout(layout, *label_visible)),
                    DialogInputControlState::Text {
                        label_visible,
                        multiline: Some(_),
                        ..
                    } => Some(dialog_multiline_text_box_layout(layout, *label_visible)),
                    _ => None,
                }
            });
        let Some(layout) = layout else {
            input.end_dialog_text_input_drag();
            return input.current_dialog().is_some();
        };
        focus_dialog_text_from_cursor(
            input,
            layout,
            cursor_position,
            surface_size,
            true,
            text_measurer,
        );
        return true;
    }
    let Some(index) = input.dialog_dragging_number_range_input() else {
        return input.current_dialog().is_some();
    };
    let layout = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
        .and_then(|data| dialog_controls(data, world.server_links()))
        .and_then(|controls| {
            let content = dialog_content_layout_with(&controls, text_ctx)?;
            let scroll_layout = dialog_body_scroll_layout_with_content(
                &controls,
                &content,
                surface_size,
                input.dialog_body_scroll_amount(),
            )?;
            dialog_input_layouts_with_content(
                &controls,
                &content,
                surface_size,
                scroll_layout.scroll_amount,
            )
        })
        .and_then(|layouts| layouts.into_iter().find(|layout| layout.index == index));
    let Some(layout) = layout else {
        input.end_dialog_number_range_drag();
        return input.current_dialog().is_some();
    };
    set_dialog_number_range_from_cursor(input, layout, cursor_position, surface_size);
    true
}
