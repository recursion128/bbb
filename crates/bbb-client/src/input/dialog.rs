use super::{
    screen::Screen, ClientInputState, DialogImePreeditHudState, DialogImePreeditState,
    ImePreeditState, DIALOG_DOUBLE_CLICK_THRESHOLD,
};
use bbb_client_clock::Instant;
use bbb_client_world::{
    CommonDialogDataState, DialogBodyState, DialogDataState, DialogInputControlState, DialogState,
    WorldStore,
};
use serde::Serialize;
use winit::event::{ElementState, MouseButton};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DialogScreenState {
    pub(super) dialog_last_click_button: Option<MouseButton>,
    pub(super) dialog_last_click_at: Option<Instant>,
    pub(super) dialog_last_click_screen_revision: Option<u64>,
    pub(super) dialog_ime_preedit: Option<DialogImePreeditState>,

    dialog: DialogState,
    dialog_input_values: Vec<Option<DialogInputValueState>>,
    dialog_dragging_number_range_input: Option<usize>,
    dialog_focus: Option<DialogFocusState>,
    dialog_number_range_keyboard_input: Option<usize>,
    dialog_text_edit: Option<DialogTextEditState>,
    dialog_dragging_text_input: Option<usize>,
    dialog_dragging_multiline_scroll_input: Option<usize>,
    dialog_dragging_multiline_scroll_y: Option<i32>,
    dialog_body_scroll_amount_bits: u64,
    dialog_body_scroll_drag_y: Option<i32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DialogLinkConfirmationState {
    /// `ClickEvent.OpenUrl`'s own units. The confirmation draws a projection of
    /// them; only the platform hand-off encodes them.
    pub url: bbb_protocol::JavaString,
    pub after_action: String,
    pub focused_button: Option<usize>,
}

pub(super) enum DialogUrlAction {
    Open(bbb_protocol::JavaString),
    Confirm(DialogLinkConfirmationState),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum DialogInputValueState {
    Boolean { selected: bool },
    NumberRange { slider_value_bits: u64 },
    SingleOption { selected_index: usize },
    Text { value: bbb_protocol::JavaString },
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DialogTextEditState {
    pub input_index: usize,
    pub cursor: usize,
    pub selection: usize,
    pub scroll_amount: i32,
    /// `EditBox.focusedTime` / `MultiLineEditBox.focusedTime` in client-clock
    /// milliseconds; it only phases the caret blink.
    pub focused_at: u64,
}

impl PartialEq for DialogTextEditState {
    fn eq(&self, other: &Self) -> bool {
        (
            self.input_index,
            self.cursor,
            self.selection,
            self.scroll_amount,
        ) == (
            other.input_index,
            other.cursor,
            other.selection,
            other.scroll_amount,
        )
    }
}

impl Eq for DialogTextEditState {}

fn dialog_focus_millis() -> u64 {
    bbb_client_clock::system_now()
        .duration_since(bbb_client_clock::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialogBodyFocusPartState {
    Primary,
    Description,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialogFocusState {
    WarningButton,
    Body {
        index: usize,
        part: DialogBodyFocusPartState,
    },
    Input {
        index: usize,
    },
    Action {
        index: usize,
    },
}

fn initial_dialog_input_values(dialog: &DialogState) -> Vec<Option<DialogInputValueState>> {
    let Some(data) = dialog.data.as_ref() else {
        return Vec::new();
    };
    let common = common_dialog_data(data);
    common
        .input_data
        .iter()
        .map(|input| match &input.control {
            DialogInputControlState::Boolean { initial, .. } => {
                Some(DialogInputValueState::Boolean { selected: *initial })
            }
            DialogInputControlState::NumberRange { range, .. } => {
                let start = range.start();
                let end = range.end();
                let initial = range.initial().unwrap_or((start + end) / 2.0);
                let slider_value = if start == end {
                    0.5
                } else {
                    (initial - start) / (end - start)
                };
                Some(DialogInputValueState::NumberRange {
                    slider_value_bits: f64::from(slider_value).to_bits(),
                })
            }
            DialogInputControlState::SingleOption { options, .. } => {
                Some(DialogInputValueState::SingleOption {
                    selected_index: options
                        .iter()
                        .position(|option| option.initial)
                        .unwrap_or(0),
                })
            }
            DialogInputControlState::Text { initial, .. } => Some(DialogInputValueState::Text {
                value: initial.clone(),
            }),
        })
        .collect()
}

fn common_dialog_data(data: &DialogDataState) -> &CommonDialogDataState {
    match data {
        DialogDataState::Notice { common, .. }
        | DialogDataState::Confirmation { common, .. }
        | DialogDataState::MultiAction { common, .. }
        | DialogDataState::ServerLinks { common, .. }
        | DialogDataState::DialogList { common, .. } => common,
    }
}

impl DialogScreenState {
    fn new(dialog: DialogState) -> Self {
        Self {
            dialog_last_click_button: None,
            dialog_last_click_at: None,
            dialog_last_click_screen_revision: None,
            dialog_ime_preedit: None,
            dialog_input_values: initial_dialog_input_values(&dialog),
            dialog,
            dialog_dragging_number_range_input: None,
            dialog_focus: None,
            dialog_number_range_keyboard_input: None,
            dialog_text_edit: None,
            dialog_dragging_text_input: None,
            dialog_dragging_multiline_scroll_input: None,
            dialog_dragging_multiline_scroll_y: None,
            dialog_body_scroll_amount_bits: 0,
            dialog_body_scroll_drag_y: None,
        }
    }
}
impl ClientInputState {
    pub(super) fn dialog_state(&self) -> Option<&DialogScreenState> {
        self.screen
            .iter()
            .rev()
            .take_while(|screen| {
                matches!(
                    screen,
                    Screen::Dialog(_)
                        | Screen::DialogWarning(_)
                        | Screen::DialogLink(_)
                        | Screen::CommandConfirmation(_)
                )
            })
            .find_map(|screen| match screen {
                Screen::Dialog(state) => Some(state),
                _ => None,
            })
    }
    pub(super) fn dialog_state_mut(&mut self) -> Option<&mut DialogScreenState> {
        self.screen
            .iter_mut()
            .rev()
            .take_while(|screen| {
                matches!(
                    screen,
                    Screen::Dialog(_)
                        | Screen::DialogWarning(_)
                        | Screen::DialogLink(_)
                        | Screen::CommandConfirmation(_)
                )
            })
            .find_map(|screen| match screen {
                Screen::Dialog(state) => Some(state),
                _ => None,
            })
    }
    pub(crate) fn current_dialog(&self) -> Option<&DialogState> {
        self.dialog_state().map(|state| &state.dialog)
    }
    pub(crate) fn current_dialog_input_values(&self) -> &[Option<DialogInputValueState>] {
        self.dialog_state()
            .map_or(&[], |state| state.dialog_input_values.as_slice())
    }
    pub(crate) fn toggle_dialog_boolean_input(&mut self, index: usize) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        let Some(Some(DialogInputValueState::Boolean { selected })) =
            state.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        *selected = !*selected;
        true
    }

    pub(crate) fn set_dialog_number_range_slider_value(
        &mut self,
        index: usize,
        value: f64,
    ) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        let Some(Some(DialogInputValueState::NumberRange { slider_value_bits })) =
            state.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        let value = value.clamp(0.0, 1.0);
        *slider_value_bits = value.to_bits();
        true
    }

    pub(crate) fn begin_dialog_number_range_drag(&mut self, index: usize) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        if !matches!(
            state.dialog_input_values.get(index),
            Some(Some(DialogInputValueState::NumberRange { .. }))
        ) {
            return false;
        }
        state.dialog_dragging_number_range_input = Some(index);
        true
    }

    pub(crate) fn end_dialog_number_range_drag(&mut self) -> Option<usize> {
        let state = self.dialog_state_mut()?;
        state.dialog_dragging_number_range_input.take()
    }

    pub(crate) fn dialog_dragging_number_range_input(&self) -> Option<usize> {
        let state = self.dialog_state()?;
        state.dialog_dragging_number_range_input
    }

    pub(crate) fn set_dialog_single_option_index(
        &mut self,
        index: usize,
        selected_index: usize,
    ) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        if !state
            .dialog
            .data
            .as_ref()
            .map(common_dialog_data)
            .and_then(|common| common.input_data.get(index))
            .is_some_and(|input| {
                matches!(
                    &input.control,
                    DialogInputControlState::SingleOption { options, .. }
                        if selected_index < options.len()
                )
            })
        {
            return false;
        }
        let Some(Some(DialogInputValueState::SingleOption {
            selected_index: current,
        })) = state.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        *current = selected_index;
        true
    }

    pub(crate) fn dialog_text_edit_state(&self) -> Option<DialogTextEditState> {
        let state = self.dialog_state()?;
        state.dialog_text_edit.filter(|edit| {
            state.dialog_focus
                == Some(DialogFocusState::Input {
                    index: edit.input_index,
                })
        })
    }

    pub(crate) fn dialog_text_edit_state_for_input(
        &self,
        index: usize,
    ) -> Option<DialogTextEditState> {
        let state = self.dialog_state()?;
        state
            .dialog_text_edit
            .filter(|edit| edit.input_index == index)
    }

    pub(crate) fn focus_dialog_text_input(
        &mut self,
        index: usize,
        cursor: usize,
        extend_selection: bool,
    ) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        let Some(Some(DialogInputValueState::Text { value })) =
            state.dialog_input_values.get(index)
        else {
            return false;
        };
        if state.dialog_text_max_length(index).is_none() {
            return false;
        }
        let cursor = cursor.min(value.units().len());
        let previous = state
            .dialog_text_edit
            .filter(|edit| edit.input_index == index);
        let selection = previous
            .filter(|_| extend_selection)
            .map_or(cursor, |edit| edit.selection);
        let scroll_amount = previous.map_or_else(
            || {
                if state.dialog_multiline_text_settings(index).is_some() {
                    i32::MAX
                } else {
                    0
                }
            },
            |edit| edit.scroll_amount,
        );
        state.dialog_text_edit = Some(DialogTextEditState {
            input_index: index,
            cursor,
            selection,
            scroll_amount,
            focused_at: previous
                .filter(|_| state.dialog_focus == Some(DialogFocusState::Input { index }))
                .map_or_else(dialog_focus_millis, |edit| edit.focused_at),
        });
        state.dialog_focus = Some(DialogFocusState::Input { index });
        state.dialog_number_range_keyboard_input = None;
        state.dialog_dragging_multiline_scroll_input = None;
        state.dialog_dragging_multiline_scroll_y = None;
        true
    }

    pub(crate) fn dialog_focused_input(&self) -> Option<usize> {
        let state = self.dialog_state()?;
        match state.dialog_focus {
            Some(DialogFocusState::Input { index }) => Some(index),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn dialog_focused_body(&self) -> Option<usize> {
        let state = self.dialog_state()?;
        match state.dialog_focus {
            Some(DialogFocusState::Body { index, .. }) => Some(index),
            _ => None,
        }
    }

    pub(crate) fn dialog_focus(&self) -> Option<DialogFocusState> {
        let state = self.dialog_state()?;
        state.dialog_focus
    }

    pub(crate) fn dialog_body_scroll_amount(&self) -> f64 {
        let Some(state) = self.dialog_state() else {
            return 0.0;
        };
        let amount = f64::from_bits(state.dialog_body_scroll_amount_bits);
        if amount.is_finite() {
            amount.max(0.0)
        } else {
            0.0
        }
    }

    pub(crate) fn set_dialog_body_scroll_amount(&mut self, amount: f64, max_scroll: f64) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        let max_scroll = if max_scroll.is_finite() {
            max_scroll.max(0.0)
        } else {
            0.0
        };
        let amount = if amount.is_finite() {
            amount.clamp(0.0, max_scroll)
        } else {
            0.0
        };
        let bits = amount.to_bits();
        if state.dialog_body_scroll_amount_bits == bits {
            return false;
        }
        state.dialog_body_scroll_amount_bits = bits;
        true
    }

    pub(crate) fn begin_dialog_body_scroll_drag(&mut self, cursor_y: i32) {
        let Some(state) = self.dialog_state_mut() else {
            return;
        };
        state.dialog_body_scroll_drag_y = Some(cursor_y);
    }

    pub(crate) fn dialog_body_scroll_drag_y(&self) -> Option<i32> {
        let state = self.dialog_state()?;
        state.dialog_body_scroll_drag_y
    }

    pub(crate) fn update_dialog_body_scroll_drag_y(&mut self, cursor_y: i32) -> Option<i32> {
        let state = self.dialog_state_mut()?;
        let previous = state.dialog_body_scroll_drag_y.replace(cursor_y)?;
        Some(cursor_y.saturating_sub(previous))
    }

    pub(crate) fn end_dialog_body_scroll_drag(&mut self) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        state.dialog_body_scroll_drag_y.take().is_some()
    }

    #[cfg(test)]
    pub(crate) fn focus_dialog_body_item(&mut self, index: usize) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        if !state
            .dialog
            .data
            .as_ref()
            .map(common_dialog_data)
            .and_then(|common| common.body_data.get(index))
            .is_some_and(|body| matches!(body, DialogBodyState::Item { .. }))
        {
            return false;
        }
        state.dialog_focus = Some(DialogFocusState::Body {
            index,
            part: DialogBodyFocusPartState::Primary,
        });
        state.dialog_number_range_keyboard_input = None;
        state.dialog_dragging_text_input = None;
        state.dialog_dragging_multiline_scroll_input = None;
        state.dialog_dragging_multiline_scroll_y = None;
        true
    }

    pub(crate) fn focus_dialog_navigation_target(
        &mut self,
        world: &WorldStore,
        target: DialogFocusState,
        number_range_can_change: bool,
    ) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        let server_link_count = world.server_links().len();
        let Some((valid, is_number_range, is_text, is_multiline_text)) =
            state.dialog.data.as_ref().map(|data| {
                let common = common_dialog_data(data);
                let valid = match target {
                    DialogFocusState::WarningButton => true,
                    DialogFocusState::Body { index, part } => {
                        common.body_data.get(index).is_some_and(|body| {
                            matches!(part, DialogBodyFocusPartState::Primary)
                                || matches!(
                                    (body, part),
                                    (
                                        DialogBodyState::Item {
                                            description: Some(_),
                                            ..
                                        },
                                        DialogBodyFocusPartState::Description,
                                    )
                                )
                        })
                    }
                    DialogFocusState::Input { index } => common.input_data.get(index).is_some(),
                    DialogFocusState::Action { index } => match data {
                        DialogDataState::Notice { .. } => index == 0,
                        DialogDataState::Confirmation { .. } => index < 2,
                        DialogDataState::MultiAction {
                            actions,
                            exit_action,
                            ..
                        } => {
                            index
                                < actions
                                    .len()
                                    .saturating_add(usize::from(exit_action.is_some()))
                        }
                        DialogDataState::DialogList {
                            dialogs,
                            exit_action,
                            ..
                        } => {
                            index
                                < dialogs
                                    .entries
                                    .len()
                                    .saturating_add(usize::from(exit_action.is_some()))
                        }
                        DialogDataState::ServerLinks { exit_action, .. } => {
                            index
                                < server_link_count
                                    .saturating_add(usize::from(exit_action.is_some()))
                        }
                    },
                };
                let control = match target {
                    DialogFocusState::Input { index } => {
                        common.input_data.get(index).map(|input| &input.control)
                    }
                    _ => None,
                };
                (
                    valid,
                    matches!(control, Some(DialogInputControlState::NumberRange { .. })),
                    matches!(control, Some(DialogInputControlState::Text { .. })),
                    matches!(
                        control,
                        Some(DialogInputControlState::Text {
                            multiline: Some(_),
                            ..
                        })
                    ),
                )
            })
        else {
            return false;
        };
        if !valid {
            return false;
        }

        state.dialog_focus = Some(target);
        state.dialog_dragging_text_input = None;
        state.dialog_dragging_multiline_scroll_input = None;
        state.dialog_dragging_multiline_scroll_y = None;
        state.dialog_number_range_keyboard_input = match target {
            DialogFocusState::Input { index } if number_range_can_change && is_number_range => {
                Some(index)
            }
            _ => None,
        };
        if let DialogFocusState::Input { index } = target {
            if is_text
                && !state
                    .dialog_text_edit
                    .is_some_and(|edit| edit.input_index == index)
            {
                let cursor = match state.dialog_input_values.get(index) {
                    Some(Some(DialogInputValueState::Text { value })) => value.units().len(),
                    _ => 0,
                };
                state.dialog_text_edit = Some(DialogTextEditState {
                    input_index: index,
                    cursor,
                    selection: cursor,
                    scroll_amount: if is_multiline_text { i32::MAX } else { 0 },
                    focused_at: dialog_focus_millis(),
                });
            }
        }
        true
    }

    pub(crate) fn focus_dialog_mouse_input(&mut self, index: usize) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        let control = state
            .dialog
            .data
            .as_ref()
            .map(common_dialog_data)
            .and_then(|common| common.input_data.get(index))
            .map(|input| &input.control);
        let number_range = matches!(control, Some(DialogInputControlState::NumberRange { .. }));
        if !matches!(
            control,
            Some(
                DialogInputControlState::Boolean { .. }
                    | DialogInputControlState::NumberRange { .. }
                    | DialogInputControlState::SingleOption { .. }
            )
        ) {
            return false;
        }
        state.dialog_focus = Some(DialogFocusState::Input { index });
        state.dialog_number_range_keyboard_input = number_range.then_some(index);
        state.dialog_dragging_text_input = None;
        state.dialog_dragging_multiline_scroll_input = None;
        state.dialog_dragging_multiline_scroll_y = None;
        true
    }

    pub(crate) fn dialog_number_range_keyboard_can_change(&self, index: usize) -> bool {
        let Some(state) = self.dialog_state() else {
            return false;
        };
        (match state.dialog_focus {
            Some(DialogFocusState::Input { index }) => Some(index),
            _ => None,
        }) == Some(index)
            && state.dialog_number_range_keyboard_input == Some(index)
    }

    pub(crate) fn toggle_dialog_number_range_keyboard_change(&mut self, index: usize) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        if match state.dialog_focus {
            Some(DialogFocusState::Input { index }) => Some(index),
            _ => None,
        } != Some(index)
            || !matches!(
                state.dialog_input_values.get(index),
                Some(Some(DialogInputValueState::NumberRange { .. }))
            )
        {
            return false;
        }
        if state.dialog_number_range_keyboard_input == Some(index) {
            state.dialog_number_range_keyboard_input = None;
        } else {
            state.dialog_number_range_keyboard_input = Some(index);
        }
        true
    }

    pub(crate) fn begin_dialog_text_input_drag(&mut self, index: usize) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        if !state
            .dialog_text_edit
            .is_some_and(|edit| edit.input_index == index)
        {
            return false;
        }
        state.dialog_dragging_text_input = Some(index);
        true
    }

    pub(crate) fn end_dialog_text_input_drag(&mut self) -> Option<usize> {
        let state = self.dialog_state_mut()?;
        state.dialog_dragging_text_input.take()
    }

    pub(crate) fn dialog_dragging_text_input(&self) -> Option<usize> {
        let state = self.dialog_state()?;
        state.dialog_dragging_text_input
    }

    #[cfg(test)]
    pub(crate) fn set_dialog_text_input_value_and_selection(
        &mut self,
        index: usize,
        value: String,
        cursor: usize,
        selection: usize,
    ) -> bool {
        self.set_dialog_text_input_raw_value_and_selection(index, value.into(), cursor, selection)
    }

    pub(crate) fn set_dialog_text_input_raw_value_and_selection(
        &mut self,
        index: usize,
        value: bbb_protocol::JavaString,
        cursor: usize,
        selection: usize,
    ) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        let Some(max_length) = state.dialog_text_max_length(index) else {
            return false;
        };
        let text_len = value.units().len();
        if text_len > max_length
            || cursor > text_len
            || selection > text_len
            || !state
                .dialog_text_edit
                .is_some_and(|edit| edit.input_index == index)
            || match state.dialog_focus {
                Some(DialogFocusState::Input { index }) => Some(index),
                _ => None,
            } != Some(index)
        {
            return false;
        }
        let Some(Some(DialogInputValueState::Text { value: current })) =
            state.dialog_input_values.get_mut(index)
        else {
            return false;
        };
        *current = value;
        let previous = state.dialog_text_edit;
        state.dialog_text_edit = Some(DialogTextEditState {
            input_index: index,
            cursor,
            selection,
            scroll_amount: previous.map_or(0, |edit| edit.scroll_amount),
            focused_at: previous.map_or_else(dialog_focus_millis, |edit| edit.focused_at),
        });
        true
    }

    pub(crate) fn dialog_multiline_text_settings(
        &self,
        index: usize,
    ) -> Option<(Option<i32>, Option<i32>)> {
        self.dialog_state()?.dialog_multiline_text_settings(index)
    }

    pub(crate) fn set_dialog_multiline_text_scroll_amount(
        &mut self,
        index: usize,
        amount: i32,
        max_scroll: i32,
    ) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        if state.dialog_multiline_text_settings(index).is_none()
            || match state.dialog_focus {
                Some(DialogFocusState::Input { index }) => Some(index),
                _ => None,
            } != Some(index)
        {
            return false;
        }
        let Some(edit) = state.dialog_text_edit.as_mut() else {
            return false;
        };
        if edit.input_index != index {
            return false;
        }
        let amount = amount.clamp(0, max_scroll.max(0));
        if edit.scroll_amount == amount {
            return false;
        }
        edit.scroll_amount = amount;
        true
    }

    pub(crate) fn begin_dialog_multiline_scroll_drag(
        &mut self,
        index: usize,
        cursor_y: i32,
    ) -> bool {
        let Some(state) = self.dialog_state_mut() else {
            return false;
        };
        if state.dialog_multiline_text_settings(index).is_none()
            || !state
                .dialog_text_edit
                .is_some_and(|edit| edit.input_index == index)
        {
            return false;
        }
        state.dialog_dragging_text_input = None;
        state.dialog_dragging_multiline_scroll_input = Some(index);
        state.dialog_dragging_multiline_scroll_y = Some(cursor_y);
        true
    }

    pub(crate) fn dialog_dragging_multiline_scroll_input(&self) -> Option<usize> {
        let state = self.dialog_state()?;
        state.dialog_dragging_multiline_scroll_input
    }

    pub(crate) fn update_dialog_multiline_scroll_drag_y(&mut self, cursor_y: i32) -> Option<i32> {
        let state = self.dialog_state_mut()?;
        let previous = state.dialog_dragging_multiline_scroll_y.replace(cursor_y)?;
        Some(cursor_y.saturating_sub(previous))
    }

    pub(crate) fn end_dialog_multiline_scroll_drag(&mut self) -> Option<usize> {
        let state = self.dialog_state_mut()?;
        state.dialog_dragging_multiline_scroll_y = None;
        state.dialog_dragging_multiline_scroll_input.take()
    }
}
impl ClientInputState {
    pub(crate) fn refresh_dialog_parent_suggestions(
        &mut self,
        world: &mut WorldStore,
        counters: &mut crate::control::NetCounters,
        commands: &Option<tokio::sync::mpsc::Sender<bbb_client_net::NetCommand>>,
    ) {
        if self.command_block_editor_hud_state().is_some() {
            super::maybe_queue_command_suggestion_request(self, counters, world, commands);
        }
    }
    pub(crate) fn show_dialog(&mut self, dialog: DialogState) {
        let warning = if self.dialog_warning_is_open() {
            self.screen
                .remove_if(|screen| matches!(screen, Screen::DialogWarning(_)))
                .pop()
        } else {
            None
        };
        if matches!(
            self.screen.current(),
            Some(Screen::Dialog(_) | Screen::DialogWaiting(_))
        ) {
            self.screen.close();
        }
        self.screen
            .open_child(Screen::Dialog(DialogScreenState::new(dialog)));
        if let Some(warning) = warning {
            self.screen.open_child(warning);
        }
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
    }
    pub(crate) fn close_dialog(&mut self) -> bool {
        if matches!(
            self.screen.current(),
            Some(Screen::DialogLink(_) | Screen::DialogWaiting(_) | Screen::CommandConfirmation(_))
        ) {
            return false;
        }
        let warning = self
            .screen
            .close_if(|screen| matches!(screen, Screen::DialogWarning(_)));
        let closed = self
            .screen
            .close_if(|screen| matches!(screen, Screen::Dialog(_)))
            .is_some();
        if let Some(warning) = warning {
            let link = self.screen.close_if(|screen| {
                matches!(
                    screen,
                    Screen::DialogLink(_) | Screen::CommandConfirmation(_)
                )
            });
            self.screen.open_child(warning);
            if let Some(link) = link {
                self.screen.open_child(link);
            }
        }
        if closed {
            self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
            self.restore_command_confirmation_presentation();
        }
        closed
    }
    pub(crate) fn wait_for_dialog_response(&mut self) -> bool {
        if self.screen.dialog().is_none() {
            return false;
        }
        self.screen.close();
        self.screen.open_child(Screen::DialogWaiting(0));
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
        true
    }
    pub(crate) fn close_dialog_waiting_response(&mut self) -> bool {
        if !self.dialog_waiting_can_close() {
            return false;
        }
        self.screen.close();
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
        self.restore_command_confirmation_presentation();
        true
    }
    pub(crate) fn advance_dialog_waiting_ticks(&mut self, ticks: u32) {
        if let Some(Screen::DialogWaiting(waiting)) = self.screen.current_mut() {
            *waiting = waiting.saturating_add(ticks);
        }
    }
    pub(crate) fn dialog_waiting_ticks(&self) -> Option<u32> {
        match self.screen.current() {
            Some(Screen::DialogWaiting(ticks)) => Some(*ticks),
            _ => None,
        }
    }
    pub(crate) fn dialog_waiting_can_close(&self) -> bool {
        self.dialog_waiting_ticks()
            .is_some_and(|ticks| ticks >= 101)
    }
    pub(crate) fn dialog_warning_is_open(&self) -> bool {
        self.screen
            .iter()
            .rev()
            .take_while(|screen| {
                matches!(
                    screen,
                    Screen::DialogWarning(_)
                        | Screen::DialogLink(_)
                        | Screen::CommandConfirmation(_)
                )
            })
            .any(|screen| matches!(screen, Screen::DialogWarning(_)))
    }
    pub(crate) fn dialog_screen_revision(&self) -> u64 {
        self.dialog_screen_revision
    }
    pub(crate) fn open_dialog_warning(&mut self) -> bool {
        if self.screen.dialog().is_none() {
            return false;
        }
        self.screen.open_child(Screen::DialogWarning(None));
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
        true
    }
    pub(crate) fn close_dialog_warning(&mut self) -> bool {
        let closed = self
            .screen
            .close_if(|screen| matches!(screen, Screen::DialogWarning(_)))
            .is_some();
        if closed {
            self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
        }
        closed
    }
    pub(crate) fn dialog_warning_focused_button(&self) -> Option<bool> {
        match self.screen.current() {
            Some(Screen::DialogWarning(focused)) => *focused,
            _ => None,
        }
    }
    pub(crate) fn focus_dialog_warning_button(&mut self, disconnect: bool) -> bool {
        let Some(Screen::DialogWarning(focused)) = self.screen.current_mut() else {
            return false;
        };
        *focused = Some(disconnect);
        true
    }
    pub(crate) fn dialog_screen_is_open(&self) -> bool {
        matches!(
            self.screen.current(),
            Some(
                Screen::Dialog(_)
                    | Screen::DialogWarning(_)
                    | Screen::DialogLink(_)
                    | Screen::DialogWaiting(_)
            )
        )
    }
    pub(crate) fn dialog_close_returns_screen(&self) -> bool {
        self.screen.parent().is_some()
    }
    pub(crate) fn set_dialog_link_options(&mut self, enabled: bool, prompt: bool) {
        self.dialog_chat_links_disabled = !enabled;
        self.dialog_chat_links_prompt_disabled = !prompt;
    }
    pub(crate) fn open_dialog_url(
        &mut self,
        url: bbb_protocol::JavaString,
        after_action: String,
    ) -> bool {
        if self.current_dialog().is_none() {
            return false;
        }
        self.open_url_with_after_action(url, after_action)
    }
    pub(crate) fn open_chat_url(&mut self, url: bbb_protocol::JavaString) -> bool {
        self.open_url_with_after_action(url, "none".into())
    }
    pub(super) fn dialog_url_action(
        &self,
        url: bbb_protocol::JavaString,
        after_action: String,
    ) -> Option<DialogUrlAction> {
        if self.dialog_chat_links_disabled {
            return None;
        }
        Some(if self.dialog_chat_links_prompt_disabled {
            DialogUrlAction::Open(url)
        } else {
            DialogUrlAction::Confirm(DialogLinkConfirmationState {
                url,
                after_action,
                focused_button: None,
            })
        })
    }
    fn open_url_with_after_action(
        &mut self,
        url: bbb_protocol::JavaString,
        after_action: String,
    ) -> bool {
        match self.dialog_url_action(url, after_action) {
            None => return false,
            Some(DialogUrlAction::Open(url)) => self.dialog_open_url_requests.push(url),
            Some(DialogUrlAction::Confirm(confirmation)) => {
                self.open_dialog_link_confirmation(confirmation);
            }
        }
        true
    }
    pub(super) fn open_dialog_link_confirmation(&mut self, link: DialogLinkConfirmationState) {
        self.screen.open_child(Screen::DialogLink(link));
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
    }
    pub(crate) fn dialog_link_confirmation(&self) -> Option<&DialogLinkConfirmationState> {
        self.screen.dialog_link()
    }
    pub(crate) fn dialog_link_focused_button(&self) -> Option<usize> {
        self.dialog_link_confirmation()
            .and_then(|link| link.focused_button)
    }
    pub(crate) fn focus_dialog_link_button(&mut self, index: usize) -> bool {
        let Some(link) = self.screen.dialog_link_mut().filter(|_| index < 3) else {
            return false;
        };
        link.focused_button = Some(index);
        true
    }
    pub(crate) fn resolve_dialog_link_confirmation(&mut self, open: bool) -> bool {
        let Some(Screen::DialogLink(link)) = self
            .screen
            .close_if(|screen| matches!(screen, Screen::DialogLink(_)))
        else {
            return false;
        };
        if open {
            self.dialog_open_url_requests.push(link.url);
        }
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
        match link.after_action.as_str() {
            "close" => self.close_dialog(),
            "wait_for_response" => self.wait_for_dialog_response(),
            "none" => true,
            _ => false,
        }
    }
    pub(crate) fn take_dialog_open_url_requests(&mut self) -> Vec<bbb_protocol::JavaString> {
        std::mem::take(&mut self.dialog_open_url_requests)
    }
    pub(crate) fn show_dialog_list_entry(&mut self, world: &WorldStore, index: usize) -> bool {
        let Some(target) = self
            .current_dialog()
            .and_then(|dialog| world.resolve_dialog_list_entry(dialog, index))
        else {
            return false;
        };
        self.show_dialog(target);
        true
    }
    pub(crate) fn show_dialog_from_component_click(
        &mut self,
        world: &WorldStore,
        value: &bbb_protocol::NbtSummaryValue,
    ) -> bool {
        let context_free = self
            .current_dialog()
            .is_some_and(|dialog| dialog.holder_kind == "context_free");
        let Some(target) = world.resolve_component_click_dialog(value, context_free) else {
            return false;
        };
        self.show_dialog(target);
        true
    }
    pub(crate) fn dialog_control_projection(&self, mut ui: serde_json::Value) -> serde_json::Value {
        let object = ui.as_object_mut().unwrap();
        object.insert(
            "current_dialog".into(),
            bbb_protocol::to_display_json_value(&self.current_dialog()).unwrap(),
        );
        let values = self.current_dialog_input_values();
        if !values.is_empty() {
            object.insert(
                "dialog_input_values".into(),
                bbb_protocol::to_display_json_value(&values).unwrap(),
            );
        }
        let link = self
            .dialog_link_confirmation()
            .or_else(|| match self.screen.current() {
                Some(Screen::Death(super::death_screen::DeathScreen {
                    runtime: super::death_screen::DeathScreenRuntime::LinkConfirmation(state),
                    ..
                })) => Some(&state.confirmation),
                _ => None,
            });
        if let Some(link) = link {
            object.insert(
                "dialog_link_confirmation".into(),
                serde_json::json!({
                    "url": bbb_protocol::to_display_json_value(&link.url).unwrap(),
                    "after_action": link.after_action,
                }),
            );
        }
        if let Some(ticks) = self.dialog_waiting_ticks() {
            object.insert("dialog_waiting_ticks".into(), ticks.into());
        }
        ui
    }
}

impl DialogScreenState {
    pub(crate) fn dialog_multiline_text_settings(
        &self,
        index: usize,
    ) -> Option<(Option<i32>, Option<i32>)> {
        let input = self
            .dialog
            .data
            .as_ref()
            .map(common_dialog_data)?
            .input_data
            .get(index)?;
        match &input.control {
            DialogInputControlState::Text {
                multiline: Some(multiline),
                ..
            } => Some((multiline.max_lines, multiline.height)),
            _ => None,
        }
    }
    fn dialog_text_max_length(&self, index: usize) -> Option<usize> {
        let input = self
            .dialog
            .data
            .as_ref()
            .map(common_dialog_data)?
            .input_data
            .get(index)?;
        match &input.control {
            DialogInputControlState::Text { max_length, .. } => usize::try_from(*max_length).ok(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;

impl ClientInputState {
    pub(crate) fn dialog_mouse_click_is_double(
        &mut self,
        button: MouseButton,
        state: ElementState,
        screen_revision: u64,
    ) -> bool {
        self.dialog_mouse_click_is_double_at(
            button,
            state,
            screen_revision,
            bbb_client_clock::now(),
        )
    }

    pub(super) fn dialog_mouse_click_is_double_at(
        &mut self,
        button: MouseButton,
        state: ElementState,
        screen_revision: u64,
        now: Instant,
    ) -> bool {
        let Some(dialog) = self.dialog_state_mut() else {
            return false;
        };
        if state != ElementState::Pressed {
            return false;
        }
        let double_click = dialog.dialog_last_click_button == Some(button)
            && dialog.dialog_last_click_screen_revision == Some(screen_revision)
            && dialog
                .dialog_last_click_at
                .and_then(|last| now.checked_duration_since(last))
                .is_some_and(|elapsed| elapsed < DIALOG_DOUBLE_CLICK_THRESHOLD);
        dialog.dialog_last_click_button = Some(button);
        dialog.dialog_last_click_at = Some(now);
        dialog.dialog_last_click_screen_revision = Some(screen_revision);
        double_click
    }

    pub(crate) fn handle_dialog_ime_preedit(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        screen_revision: u64,
    ) -> bool {
        self.handle_dialog_ime_preedit_at(
            text,
            cursor_range,
            screen_revision,
            bbb_client_clock::now(),
        )
    }

    pub(super) fn handle_dialog_ime_preedit_at(
        &mut self,
        text: &str,
        cursor_range: Option<(usize, usize)>,
        screen_revision: u64,
        now: Instant,
    ) -> bool {
        let Some(dialog) = self.dialog_state_mut() else {
            return true;
        };
        dialog.dialog_ime_preedit =
            ImePreeditState::from_event(text, cursor_range, now).map(|preedit| {
                DialogImePreeditState {
                    screen_revision,
                    preedit,
                }
            });
        true
    }

    pub(crate) fn clear_dialog_ime_preedit(&mut self) {
        let Some(dialog) = self.dialog_state_mut() else {
            return;
        };
        dialog.dialog_ime_preedit = None;
    }

    pub(crate) fn dialog_ime_preedit_hud_state(&self) -> Option<DialogImePreeditHudState> {
        self.dialog_ime_preedit_hud_state_at(bbb_client_clock::now())
    }

    pub(super) fn dialog_ime_preedit_hud_state_at(
        &self,
        now: Instant,
    ) -> Option<DialogImePreeditHudState> {
        let dialog = self.dialog_state()?;
        let state = dialog.dialog_ime_preedit.as_ref()?;
        let preedit = state.preedit.hud_state(now);
        Some(DialogImePreeditHudState {
            text: preedit.text,
            caret: preedit.caret,
            focused_start: preedit.focused_start,
            focused_end: preedit.focused_end,
            screen_revision: state.screen_revision,
            caret_visible: preedit.caret_visible,
        })
    }
}
