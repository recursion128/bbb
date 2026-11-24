use super::*;
use crate::input::dialog::DialogBodyFocusPartState;
use crate::input::dialog::DialogFocusState;

impl DialogBodyBlock {
    pub(super) fn width(&self) -> i32 {
        match self {
            Self::Text(text) => i32::try_from(text.width).unwrap_or(i32::MAX),
            Self::Item(item) => {
                dialog_item_dimensions(
                    item.widget_width,
                    item.widget_height,
                    item.description.as_ref(),
                )
                .0
            }
        }
    }

    pub(super) fn height(&self) -> i32 {
        match self {
            Self::Text(text) => text.height,
            Self::Item(item) => {
                dialog_item_dimensions(
                    item.widget_width,
                    item.widget_height,
                    item.description.as_ref(),
                )
                .1
            }
        }
    }
}

pub(super) fn dialog_item_dimensions(
    widget_width: i32,
    widget_height: i32,
    description: Option<&DialogTextBlock>,
) -> (i32, i32) {
    description.map_or((widget_width, widget_height), |description| {
        (
            widget_width
                .saturating_add(ITEM_DESCRIPTION_SPACING)
                .saturating_add(i32::try_from(description.width).unwrap_or(i32::MAX)),
            widget_height.max(description.height),
        )
    })
}

impl DialogFooterLayout {
    pub(super) fn new(
        action_widths: &[i32],
        footer_height: i32,
        surface_size: PhysicalSize<u32>,
    ) -> Option<Self> {
        let screen_width = surface_size.width.max(1);
        let screen_height = surface_size.height.max(1);
        let screen_width_i32 = i32::try_from(screen_width).ok()?;
        let screen_height_i32 = i32::try_from(screen_height).ok()?;
        let button_widths = action_widths
            .iter()
            .map(|width| u32::try_from(*width).ok())
            .collect::<Option<Vec<_>>>()?;
        let spacing_width = SIMPLE_BUTTON_SPACING.saturating_mul(
            i32::try_from(button_widths.len().saturating_sub(1)).unwrap_or(i32::MAX),
        );
        let total_width = button_widths.iter().fold(spacing_width, |total, width| {
            total.saturating_add(i32::try_from(*width).unwrap_or(i32::MAX))
        });
        let button_y = screen_height_i32 - footer_height
            + (footer_height - i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX)) / 2;
        // Notice and Confirmation have at most two codec-bounded buttons, so this
        // subtraction is bounded while preserving vanilla's negative centered x
        // when the declared button row is wider than the screen.
        let mut button_x = (screen_width_i32 - total_width) / 2;
        let buttons = button_widths
            .into_iter()
            .map(|width| {
                let button = DialogButtonLayout {
                    x: button_x,
                    y: button_y,
                    width,
                };
                button_x = button_x
                    .saturating_add(i32::try_from(width).unwrap_or(i32::MAX))
                    .saturating_add(SIMPLE_BUTTON_SPACING);
                button
            })
            .collect();
        Some(Self {
            screen_width,
            screen_height,
            buttons,
        })
    }

    pub(super) fn button_at(
        &self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> Option<usize> {
        self.buttons.iter().position(|button| {
            screen_button_contains(
                self.screen_width,
                self.screen_height,
                button.x,
                button.y,
                button.width,
                cursor_position,
                surface_size,
            )
        })
    }
}

impl DialogWarningButtonLayout {
    #[cfg(test)]
    pub(super) fn new(title: &DialogTextState, surface_size: PhysicalSize<u32>) -> Option<Self> {
        Self::new_with(title, surface_size, DialogTextContext::approximate(None))
    }

    pub(super) fn new_with(
        title: &DialogTextState,
        surface_size: PhysicalSize<u32>,
        text_ctx: DialogTextContext<'_>,
    ) -> Option<Self> {
        let screen_width = i32::try_from(surface_size.width.max(1)).ok()?;
        let screen_height = i32::try_from(surface_size.height.max(1)).ok()?;
        let title_runs = dialog_text_runs_with(title, text_ctx.item_runtime);
        let title_width = i32::try_from(runs_width(&title_runs, text_ctx.glyphs)).ok()?;
        let row_width = title_width
            .saturating_add(WARNING_BUTTON_SPACING)
            .saturating_add(WARNING_BUTTON_SIZE);
        let row_x = (screen_width - row_width) / 2;
        let row_y = (HEADER_HEIGHT - WARNING_BUTTON_SIZE)
            .saturating_add(1)
            .div_euclid(2);
        let mut x = row_x
            .saturating_add(title_width)
            .saturating_add(WARNING_BUTTON_SPACING);
        let mut y = row_y;
        if x < 0
            || y < 0
            || x > screen_width.saturating_sub(WARNING_BUTTON_SIZE)
            || y > screen_height.saturating_sub(WARNING_BUTTON_SIZE)
        {
            x = 0.max(screen_width.saturating_sub(40));
            y = 5.min(screen_height);
        }
        Some(Self {
            title_x: row_x,
            title_y: row_y.saturating_add(
                (WARNING_BUTTON_SIZE - TEXT_LINE_HEIGHT)
                    .saturating_add(1)
                    .div_euclid(2),
            ),
            x,
            y,
        })
    }

    pub(super) fn contains(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let screen_width = surface_size.width.max(1);
        let screen_height = surface_size.height.max(1);
        dialog_widget_contains(
            self.x,
            self.y,
            WARNING_BUTTON_SIZE,
            WARNING_BUTTON_SIZE,
            cursor_position,
            surface_size,
            screen_width,
            screen_height,
        )
    }
}

impl DialogWarningScreenLayout {
    pub(super) fn new(message_line_count: usize, surface_size: PhysicalSize<u32>) -> Self {
        let screen_width = surface_size.width.max(1);
        let screen_height = surface_size.height.max(1);
        let screen_width_i32 = i32::try_from(screen_width).unwrap_or(i32::MAX);
        let screen_height_i32 = i32::try_from(screen_height).unwrap_or(i32::MAX);
        let message_height = i32::try_from(message_line_count)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT);
        let layout_height = TEXT_LINE_HEIGHT
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP)
            .saturating_add(i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX));
        let layout_y = (screen_height_i32 - layout_height) / 2;
        let button_row_width = i32::try_from(CONFIRM_BUTTON_WIDTH)
            .unwrap_or(i32::MAX)
            .saturating_mul(2)
            .saturating_add(CONFIRM_BUTTON_SPACING);
        let yes_x = (screen_width_i32 - button_row_width) / 2;
        let button_y = layout_y
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP);
        Self {
            screen_width,
            screen_height,
            title_y: layout_y,
            message_y: layout_y
                .saturating_add(TEXT_LINE_HEIGHT)
                .saturating_add(CONFIRM_ELEMENT_SPACING),
            yes: DialogButtonLayout {
                x: yes_x,
                y: button_y,
                width: CONFIRM_BUTTON_WIDTH,
            },
            no: DialogButtonLayout {
                x: yes_x
                    .saturating_add(i32::try_from(CONFIRM_BUTTON_WIDTH).unwrap_or(i32::MAX))
                    .saturating_add(CONFIRM_BUTTON_SPACING),
                y: button_y,
                width: CONFIRM_BUTTON_WIDTH,
            },
        }
    }

    pub(super) fn button_at(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> Option<bool> {
        [(&self.yes, true), (&self.no, false)]
            .into_iter()
            .find_map(|(button, result)| {
                dialog_widget_contains(
                    button.x,
                    button.y,
                    i32::try_from(button.width).unwrap_or(i32::MAX),
                    i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                    cursor_position,
                    surface_size,
                    self.screen_width,
                    self.screen_height,
                )
                .then_some(result)
            })
    }
}

impl DialogConfirmLinkLayout {
    pub(super) fn new(message_line_count: usize, surface_size: PhysicalSize<u32>) -> Self {
        let screen_width = surface_size.width.max(1);
        let screen_height = surface_size.height.max(1);
        let screen_width_i32 = i32::try_from(screen_width).unwrap_or(i32::MAX);
        let screen_height_i32 = i32::try_from(screen_height).unwrap_or(i32::MAX);
        let message_height = i32::try_from(message_line_count)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT);
        let layout_height = TEXT_LINE_HEIGHT
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP)
            .saturating_add(i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX));
        let layout_y = (screen_height_i32 - layout_height) / 2;
        let warning_y = layout_y
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(message_height)
            .saturating_add(CONFIRM_ELEMENT_SPACING);
        let button_y = warning_y
            .saturating_add(TEXT_LINE_HEIGHT)
            .saturating_add(CONFIRM_ELEMENT_SPACING)
            .saturating_add(CONFIRM_BUTTON_PADDING_TOP);
        let button_width = i32::try_from(CONFIRM_LINK_BUTTON_WIDTH).unwrap_or(i32::MAX);
        let button_row_width = button_width
            .saturating_mul(3)
            .saturating_add(CONFIRM_BUTTON_SPACING.saturating_mul(2));
        let button_x = (screen_width_i32 - button_row_width) / 2;
        Self {
            screen_width,
            screen_height,
            title_y: layout_y,
            message_y: layout_y
                .saturating_add(TEXT_LINE_HEIGHT)
                .saturating_add(CONFIRM_ELEMENT_SPACING),
            warning_y,
            buttons: [
                DialogButtonLayout {
                    x: button_x,
                    y: button_y,
                    width: CONFIRM_LINK_BUTTON_WIDTH,
                },
                DialogButtonLayout {
                    x: button_x
                        .saturating_add(button_width)
                        .saturating_add(CONFIRM_BUTTON_SPACING),
                    y: button_y,
                    width: CONFIRM_LINK_BUTTON_WIDTH,
                },
                DialogButtonLayout {
                    x: button_x
                        .saturating_add(button_width.saturating_mul(2))
                        .saturating_add(CONFIRM_BUTTON_SPACING.saturating_mul(2)),
                    y: button_y,
                    width: CONFIRM_LINK_BUTTON_WIDTH,
                },
            ],
        }
    }

    pub(super) fn button_at(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> Option<usize> {
        self.buttons.iter().position(|button| {
            screen_button_contains(
                self.screen_width,
                self.screen_height,
                button.x,
                button.y,
                button.width,
                cursor_position,
                surface_size,
            )
        })
    }
}

#[derive(Clone, Copy)]
pub(super) enum DialogControlAction<'a> {
    Button(&'a DialogButtonState),
    DialogListEntry {
        index: usize,
        label: &'a DialogTextState,
        width: i32,
    },
    ServerLink {
        label: &'a str,
        url: &'a str,
        width: i32,
    },
}

impl<'a> DialogControlAction<'a> {
    pub(super) fn width(self) -> i32 {
        match self {
            Self::Button(button) => button.width,
            Self::DialogListEntry { width, .. } | Self::ServerLink { width, .. } => width,
        }
    }

    pub(super) fn styled_label(self) -> Option<&'a DialogTextState> {
        match self {
            Self::Button(button) => Some(&button.label),
            Self::DialogListEntry { label, .. } => Some(label),
            Self::ServerLink { .. } => None,
        }
    }

    pub(super) fn label_text(self) -> &'a str {
        match self {
            Self::Button(button) => &button.label.text,
            Self::DialogListEntry { label, .. } => &label.text,
            Self::ServerLink { label, .. } => label,
        }
    }

    pub(super) fn tooltip(self) -> Option<&'a DialogTextState> {
        match self {
            Self::Button(button) => button.tooltip.as_ref(),
            Self::DialogListEntry { .. } | Self::ServerLink { .. } => None,
        }
    }

    pub(super) fn activation(self) -> DialogControlActivation {
        match self {
            Self::Button(button) => DialogControlActivation::Action(button.action_data.clone()),
            Self::DialogListEntry { index, .. } => DialogControlActivation::DialogListEntry(index),
            Self::ServerLink { url, .. } => {
                DialogControlActivation::Action(Some(DialogActionState::OpenUrl {
                    url: url.to_string().into(),
                }))
            }
        }
    }
}

pub(super) enum DialogControlActivation {
    Action(Option<DialogActionState>),
    DialogListEntry(usize),
}

pub(super) struct DialogControls<'a> {
    pub(super) common: &'a CommonDialogDataState,
    pub(super) actions: Vec<DialogControlAction<'a>>,
    pub(super) body_action_count: usize,
    pub(super) body_action_columns: i32,
    pub(super) cancel: Option<&'a DialogButtonState>,
    pub(super) footer_height: i32,
}

impl DialogControls<'_> {
    pub(super) fn show_server_dialog_warning(&self) -> bool {
        !matches!(
            self.cancel.and_then(|button| button.action_data.as_ref()),
            Some(DialogActionState::ResourcePackConsent { .. })
        )
    }

    pub(super) fn body_actions(&self) -> &[DialogControlAction<'_>] {
        &self.actions[..self.body_action_count]
    }

    pub(super) fn footer_actions(&self) -> &[DialogControlAction<'_>] {
        &self.actions[self.body_action_count..]
    }
}

pub(super) fn dialog_controls<'a>(
    data: &'a DialogDataState,
    server_links: &'a [ServerLinkState],
) -> Option<DialogControls<'a>> {
    match data {
        DialogDataState::Notice { common, action } => Some(DialogControls {
            common,
            actions: vec![DialogControlAction::Button(action)],
            body_action_count: 0,
            body_action_columns: 1,
            cancel: Some(action),
            footer_height: FOOTER_HEIGHT,
        }),
        DialogDataState::Confirmation { common, yes, no } => Some(DialogControls {
            common,
            actions: vec![
                DialogControlAction::Button(yes),
                DialogControlAction::Button(no),
            ],
            body_action_count: 0,
            body_action_columns: 1,
            cancel: Some(no),
            footer_height: FOOTER_HEIGHT,
        }),
        DialogDataState::MultiAction {
            common,
            actions,
            exit_action,
            columns,
        } => {
            let mut all_actions = actions
                .iter()
                .map(DialogControlAction::Button)
                .collect::<Vec<_>>();
            all_actions.extend(exit_action.iter().map(DialogControlAction::Button));
            Some(DialogControls {
                common,
                actions: all_actions,
                body_action_count: actions.len(),
                body_action_columns: *columns,
                cancel: exit_action.as_ref(),
                footer_height: if exit_action.is_some() {
                    FOOTER_HEIGHT
                } else {
                    5
                },
            })
        }
        DialogDataState::DialogList {
            common,
            dialogs,
            exit_action,
            columns,
            button_width,
        } => {
            let mut all_actions = dialogs
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| DialogControlAction::DialogListEntry {
                    index,
                    label: &entry.external_title,
                    width: *button_width,
                })
                .collect::<Vec<_>>();
            all_actions.extend(exit_action.iter().map(DialogControlAction::Button));
            Some(DialogControls {
                common,
                actions: all_actions,
                body_action_count: dialogs.entries.len(),
                body_action_columns: *columns,
                cancel: exit_action.as_ref(),
                footer_height: if exit_action.is_some() {
                    FOOTER_HEIGHT
                } else {
                    5
                },
            })
        }
        DialogDataState::ServerLinks {
            common,
            exit_action,
            columns,
            button_width,
        } => {
            let mut all_actions = server_links
                .iter()
                .map(|link| DialogControlAction::ServerLink {
                    label: server_link_display_label(link),
                    url: &link.url,
                    width: *button_width,
                })
                .collect::<Vec<_>>();
            all_actions.extend(exit_action.iter().map(DialogControlAction::Button));
            Some(DialogControls {
                common,
                actions: all_actions,
                body_action_count: server_links.len(),
                body_action_columns: *columns,
                cancel: exit_action.as_ref(),
                footer_height: if exit_action.is_some() {
                    FOOTER_HEIGHT
                } else {
                    5
                },
            })
        }
    }
}

pub(super) fn server_link_display_label(link: &ServerLinkState) -> &str {
    match link.known_type.as_deref() {
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
        _ => &link.label,
    }
}

pub(super) fn dialog_body_action_grid_layout(
    controls: &DialogControls<'_>,
) -> Option<DialogBodyActionGridLayout> {
    let actions = controls.body_actions();
    if actions.is_empty() {
        return None;
    }
    let count = actions.len();
    let columns = usize::try_from(controls.body_action_columns).ok()?.max(1);
    let widths = actions
        .iter()
        .map(|action| i32::try_from(u32::try_from(action.width()).ok()?).ok())
        .collect::<Option<Vec<_>>>()?;
    if columns > count {
        let row_width = widths.iter().copied().fold(
            i32::try_from(count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
            i32::saturating_add,
        );
        let spacing_width = i32::try_from(columns.saturating_sub(1))
            .unwrap_or(i32::MAX)
            .saturating_mul(BUTTON_LIST_SPACING);
        // GridLayout lets the partial LinearLayout span the declared column
        // count. Its Divisor can contribute only non-negative column widths;
        // preserve that result without allocating an attacker-sized column
        // array when the codec's positive column count exceeds the actions.
        let grid_width =
            spacing_width.saturating_add(row_width.saturating_sub(spacing_width).max(0));
        let mut x = (grid_width.saturating_sub(row_width)) / 2;
        let mut buttons = Vec::with_capacity(count);
        for width in widths {
            buttons.push(DialogButtonLayout {
                x,
                y: 0,
                width: u32::try_from(width).ok()?,
            });
            x = x.saturating_add(width).saturating_add(BUTTON_LIST_SPACING);
        }
        return Some(DialogBodyActionGridLayout {
            width: grid_width,
            height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
            buttons,
        });
    }
    let complete_rows = count / columns;
    let remainder = count % columns;
    let complete_count = complete_rows.saturating_mul(columns);
    let mut column_widths = vec![0i32; columns];
    for (index, width) in widths.iter().copied().take(complete_count).enumerate() {
        column_widths[index % columns] = column_widths[index % columns].max(width);
    }
    if remainder > 0 {
        let last_row_width = widths[complete_count..].iter().copied().fold(
            i32::try_from(remainder.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
            i32::saturating_add,
        );
        let numerator = last_row_width.saturating_sub(
            i32::try_from(columns.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
        );
        let denominator = i32::try_from(columns).ok()?;
        let quotient = numerator / denominator;
        let modulus = numerator % denominator;
        let mut carry = 0i32;
        for column_width in &mut column_widths {
            let mut part = quotient;
            carry = carry.saturating_add(modulus);
            if carry >= denominator {
                carry = carry.saturating_sub(denominator);
                part = part.saturating_add(1);
            }
            *column_width = (*column_width).max(part);
        }
    }
    let grid_width = column_widths.iter().copied().fold(
        i32::try_from(columns.saturating_sub(1))
            .unwrap_or(i32::MAX)
            .saturating_mul(BUTTON_LIST_SPACING),
        i32::saturating_add,
    );
    let rows = complete_rows.saturating_add(usize::from(remainder > 0));
    let grid_height = i32::try_from(rows)
        .unwrap_or(i32::MAX)
        .saturating_mul(i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX))
        .saturating_add(
            i32::try_from(rows.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
        );
    let mut column_offsets = Vec::with_capacity(columns);
    let mut next_x = 0i32;
    for width in &column_widths {
        column_offsets.push(next_x);
        next_x = next_x
            .saturating_add(*width)
            .saturating_add(BUTTON_LIST_SPACING);
    }
    let mut buttons = Vec::with_capacity(count);
    for (index, width) in widths.iter().copied().take(complete_count).enumerate() {
        let row = index / columns;
        let column = index % columns;
        buttons.push(DialogButtonLayout {
            x: column_offsets[column]
                .saturating_add((column_widths[column].saturating_sub(width)) / 2),
            y: i32::try_from(row).unwrap_or(i32::MAX).saturating_mul(
                i32::try_from(BUTTON_HEIGHT)
                    .unwrap_or(i32::MAX)
                    .saturating_add(BUTTON_LIST_SPACING),
            ),
            width: u32::try_from(width).ok()?,
        });
    }
    if remainder > 0 {
        let last_widths = &widths[complete_count..];
        let last_row_width = last_widths.iter().copied().fold(
            i32::try_from(remainder.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BUTTON_LIST_SPACING),
            i32::saturating_add,
        );
        let mut x = (grid_width.saturating_sub(last_row_width)) / 2;
        let y = i32::try_from(complete_rows)
            .unwrap_or(i32::MAX)
            .saturating_mul(
                i32::try_from(BUTTON_HEIGHT)
                    .unwrap_or(i32::MAX)
                    .saturating_add(BUTTON_LIST_SPACING),
            );
        for width in last_widths {
            buttons.push(DialogButtonLayout {
                x,
                y,
                width: u32::try_from(*width).ok()?,
            });
            x = x.saturating_add(*width).saturating_add(BUTTON_LIST_SPACING);
        }
    }
    Some(DialogBodyActionGridLayout {
        width: grid_width,
        height: grid_height,
        buttons,
    })
}

pub(super) fn dialog_content_layout_with(
    controls: &DialogControls<'_>,
    text_ctx: DialogTextContext<'_>,
) -> Option<DialogContentLayout> {
    let body_dimensions = controls
        .common
        .body_data
        .iter()
        .map(|body| dialog_body_declared_dimensions_with(body, text_ctx))
        .collect::<Option<Vec<_>>>()?;
    let input_dimensions = controls
        .common
        .input_data
        .iter()
        .map(|input| dialog_input_dimensions_with(&input.control, text_ctx))
        .collect::<Option<Vec<_>>>()?;
    let action_grid = dialog_body_action_grid_layout(controls);
    let element_count = body_dimensions
        .len()
        .saturating_add(input_dimensions.len())
        .saturating_add(usize::from(action_grid.is_some()));
    let width = body_dimensions
        .iter()
        .chain(&input_dimensions)
        .map(|(width, _)| *width)
        .max()
        .unwrap_or(0)
        .max(action_grid.as_ref().map_or(0, |grid| grid.width));
    let height = body_dimensions
        .iter()
        .chain(&input_dimensions)
        .map(|(_, height)| *height)
        .fold(0i32, i32::saturating_add)
        .saturating_add(action_grid.as_ref().map_or(0, |grid| grid.height))
        .saturating_add(
            i32::try_from(element_count.saturating_sub(1))
                .unwrap_or(i32::MAX)
                .saturating_mul(BODY_SPACING),
        );
    Some(DialogContentLayout {
        body_dimensions,
        input_dimensions,
        action_grid,
        width,
        height,
    })
}

pub(super) fn dialog_body_declared_dimensions_with(
    body: &DialogBodyState,
    text_ctx: DialogTextContext<'_>,
) -> Option<(i32, i32)> {
    match body {
        DialogBodyState::PlainMessage { message } => {
            let text = dialog_text_block_with(message, text_ctx)?;
            Some((i32::try_from(text.width).ok()?, text.height))
        }
        DialogBodyState::Item {
            description,
            width,
            height,
            ..
        } => {
            let description = description
                .as_ref()
                .and_then(|desc| dialog_text_block_with(desc, text_ctx));
            Some(dialog_item_dimensions(
                *width,
                *height,
                description.as_ref(),
            ))
        }
    }
}

pub(super) fn dialog_content_y(screen_height: i32, content_height: i32, footer_height: i32) -> i32 {
    let container_height = content_height.min(
        screen_height
            .saturating_sub(HEADER_HEIGHT)
            .saturating_sub(footer_height),
    );
    (HEADER_HEIGHT + CONTENT_MARGIN_TOP).min(
        screen_height
            .saturating_sub(footer_height)
            .saturating_sub(container_height),
    )
}

pub(super) fn dialog_input_dimensions_with(
    control: &DialogInputControlState,
    text_ctx: DialogTextContext<'_>,
) -> Option<(i32, i32)> {
    match control {
        DialogInputControlState::Boolean { label, .. } => {
            let label_runs = dialog_text_runs_with(label, text_ctx.item_runtime);
            let label_width = i32::try_from(runs_width(&label_runs, text_ctx.glyphs)).ok()?;
            Some((
                i32::try_from(CHECKBOX_SIZE)
                    .ok()?
                    .saturating_add(CHECKBOX_LABEL_SPACING)
                    .saturating_add(label_width),
                i32::try_from(CHECKBOX_SIZE).ok()?,
            ))
        }
        DialogInputControlState::NumberRange { width, .. } => {
            Some((*width, i32::try_from(SLIDER_HEIGHT).ok()?))
        }
        DialogInputControlState::SingleOption { width, .. } => {
            Some((*width, i32::try_from(BUTTON_HEIGHT).ok()?))
        }
        DialogInputControlState::Text {
            width,
            label_visible,
            multiline,
            ..
        } => {
            let control_height = multiline.as_ref().map_or(20, |multiline| {
                multiline.height.unwrap_or_else(|| {
                    multiline
                        .max_lines
                        .unwrap_or(4)
                        .saturating_mul(TEXT_LINE_HEIGHT)
                        .saturating_add(8)
                        .min(512)
                })
            });
            Some((
                *width,
                control_height.saturating_add(if *label_visible {
                    TEXT_LINE_HEIGHT + 4
                } else {
                    0
                }),
            ))
        }
    }
}

#[cfg(test)]
pub(super) fn dialog_input_layouts(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
) -> Option<Vec<DialogInputLayout>> {
    let content = dialog_content_layout_with(controls, DialogTextContext::approximate(None))?;
    dialog_input_layouts_with_content(controls, &content, surface_size, 0)
}

pub(super) fn dialog_input_layouts_with_content(
    controls: &DialogControls<'_>,
    content: &DialogContentLayout,
    surface_size: PhysicalSize<u32>,
    scroll_amount: i32,
) -> Option<Vec<DialogInputLayout>> {
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let screen_width_i32 = i32::try_from(screen_width).ok()?;
    let screen_height_i32 = i32::try_from(screen_height).ok()?;
    let mut y = dialog_content_y(screen_height_i32, content.height, controls.footer_height)
        .saturating_sub(scroll_amount);
    let mut ordinal = 0usize;
    for &(_, height) in &content.body_dimensions {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        ordinal = ordinal.saturating_add(1);
        y = y.saturating_add(height);
    }
    content
        .input_dimensions
        .iter()
        .copied()
        .enumerate()
        .map(|(index, (width, height))| {
            if ordinal > 0 {
                y = y.saturating_add(BODY_SPACING);
            }
            ordinal = ordinal.saturating_add(1);
            let layout = DialogInputLayout {
                index,
                x: screen_width_i32 / 2 - width / 2,
                y,
                width,
                height,
            };
            y = y.saturating_add(height);
            Some(layout)
        })
        .collect()
}

#[cfg(test)]
pub(super) fn dialog_focus_graph(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
) -> Option<DialogFocusGraph> {
    let text_ctx = DialogTextContext::approximate(None);
    let content = dialog_content_layout_with(controls, text_ctx)?;
    dialog_focus_graph_with_content(controls, &content, surface_size, 0, text_ctx)
}

pub(super) fn dialog_focus_graph_with_content(
    controls: &DialogControls<'_>,
    content: &DialogContentLayout,
    surface_size: PhysicalSize<u32>,
    scroll_amount: i32,
    text_ctx: DialogTextContext<'_>,
) -> Option<DialogFocusGraph> {
    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let screen_width_i32 = i32::try_from(screen_width).ok()?;
    let screen_height_i32 = i32::try_from(screen_height).ok()?;
    let content_y = dialog_content_y(screen_height_i32, content.height, controls.footer_height);
    let mut y = content_y.saturating_sub(scroll_amount);
    let mut body = Vec::new();
    let mut ordinal = 0usize;
    for (index, (body_state, (block_width, block_height))) in controls
        .common
        .body_data
        .iter()
        .zip(content.body_dimensions.iter().copied())
        .enumerate()
    {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        ordinal = ordinal.saturating_add(1);
        let block_x = screen_width_i32 / 2 - block_width / 2;
        match body_state {
            DialogBodyState::PlainMessage { .. } => body.push(DialogFocusNode {
                target: DialogFocusState::Body {
                    index,
                    part: DialogBodyFocusPartState::Primary,
                },
                rectangle: DialogFocusRectangle {
                    x: block_x,
                    y,
                    width: block_width,
                    height: block_height,
                },
            }),
            DialogBodyState::Item {
                description,
                width,
                height,
                ..
            } => {
                let item_y = y.saturating_add(vertical_center_offset(block_height, *height));
                body.push(DialogFocusNode {
                    target: DialogFocusState::Body {
                        index,
                        part: DialogBodyFocusPartState::Primary,
                    },
                    rectangle: DialogFocusRectangle {
                        x: block_x,
                        y: item_y,
                        width: *width,
                        height: *height,
                    },
                });
                if let Some(description) = description
                    .as_ref()
                    .and_then(|desc| dialog_text_block_with(desc, text_ctx))
                {
                    body.push(DialogFocusNode {
                        target: DialogFocusState::Body {
                            index,
                            part: DialogBodyFocusPartState::Description,
                        },
                        rectangle: DialogFocusRectangle {
                            x: block_x
                                .saturating_add(*width)
                                .saturating_add(ITEM_DESCRIPTION_SPACING),
                            y: y.saturating_add(vertical_center_offset(
                                block_height,
                                description.height,
                            )),
                            width: i32::try_from(description.width).unwrap_or(i32::MAX),
                            height: description.height,
                        },
                    });
                }
            }
        }
        y = y.saturating_add(block_height);
    }
    for (index, (input, (width, height))) in controls
        .common
        .input_data
        .iter()
        .zip(content.input_dimensions.iter().copied())
        .enumerate()
    {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        ordinal = ordinal.saturating_add(1);
        let layout = DialogInputLayout {
            index,
            x: screen_width_i32 / 2 - width / 2,
            y,
            width,
            height,
        };
        let focus_layout = match &input.control {
            DialogInputControlState::Boolean { .. }
            | DialogInputControlState::NumberRange { .. }
            | DialogInputControlState::SingleOption { .. } => Some(layout),
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
        };
        if let Some(layout) = focus_layout {
            body.push(DialogFocusNode {
                target: DialogFocusState::Input { index },
                rectangle: DialogFocusRectangle {
                    x: layout.x,
                    y: layout.y,
                    width: layout.width,
                    height: layout.height,
                },
            });
        }
        y = y.saturating_add(height);
    }
    if let Some(action_grid) = content.action_grid.as_ref() {
        if ordinal > 0 {
            y = y.saturating_add(BODY_SPACING);
        }
        let grid_x = screen_width_i32 / 2 - action_grid.width / 2;
        body.extend(
            action_grid
                .buttons
                .iter()
                .enumerate()
                .map(|(index, button)| DialogFocusNode {
                    target: DialogFocusState::Action { index },
                    rectangle: DialogFocusRectangle {
                        x: grid_x.saturating_add(button.x),
                        y: y.saturating_add(button.y),
                        width: i32::try_from(button.width).unwrap_or(i32::MAX),
                        height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                    },
                }),
        );
    }

    let container_width = content.width.saturating_add(BODY_SCROLLBAR_RESERVE * 2);
    let container_height = content.height.min(
        screen_height_i32
            .saturating_sub(HEADER_HEIGHT)
            .saturating_sub(controls.footer_height),
    );
    let container = DialogBodyFocusContainer {
        rectangle: DialogFocusRectangle {
            x: screen_width_i32 / 2 - container_width / 2,
            y: content_y,
            width: container_width,
            height: container_height,
        },
        content_rectangle: DialogFocusRectangle {
            x: screen_width_i32 / 2 - container_width / 2,
            y: content_y.saturating_sub(scroll_amount),
            width: container_width,
            height: content.height,
        },
    };
    let warning =
        DialogWarningButtonLayout::new_with(&controls.common.title, surface_size, text_ctx)?;
    let warning = DialogFocusNode {
        target: DialogFocusState::WarningButton,
        rectangle: DialogFocusRectangle {
            x: warning.x,
            y: warning.y,
            width: WARNING_BUTTON_SIZE,
            height: WARNING_BUTTON_SIZE,
        },
    };
    let action_layout = DialogFooterLayout::new(
        &controls
            .footer_actions()
            .iter()
            .map(|action| action.width())
            .collect::<Vec<_>>(),
        controls.footer_height,
        surface_size,
    )?;
    let actions = action_layout
        .buttons
        .iter()
        .enumerate()
        .map(|(index, button)| DialogFocusNode {
            target: DialogFocusState::Action {
                index: controls.body_action_count.saturating_add(index),
            },
            rectangle: DialogFocusRectangle {
                x: button.x,
                y: button.y,
                width: i32::try_from(button.width).unwrap_or(i32::MAX),
                height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
            },
        })
        .collect::<Vec<_>>();
    let mut top = Vec::with_capacity(actions.len().saturating_add(2));
    top.push(DialogTopFocusNode::Body(container));
    top.extend(actions.iter().copied().map(DialogTopFocusNode::Target));
    if controls.show_server_dialog_warning() {
        top.push(DialogTopFocusNode::Target(warning));
    }
    let mut tab = Vec::with_capacity(body.len().saturating_add(actions.len()).saturating_add(1));
    if controls.show_server_dialog_warning() {
        tab.push(warning.target);
    }
    tab.extend(body.iter().map(|node| node.target));
    tab.extend(actions.iter().map(|node| node.target));
    Some(DialogFocusGraph {
        screen: DialogFocusRectangle {
            x: 0,
            y: 0,
            width: screen_width_i32,
            height: screen_height_i32,
        },
        body,
        top,
        tab,
    })
}

#[cfg(test)]
pub(super) fn dialog_body_scroll_layout(
    controls: &DialogControls<'_>,
    surface_size: PhysicalSize<u32>,
    scroll_amount: f64,
) -> Option<DialogBodyScrollLayout> {
    let content = dialog_content_layout_with(controls, DialogTextContext::approximate(None))?;
    dialog_body_scroll_layout_with_content(controls, &content, surface_size, scroll_amount)
}

pub(super) fn dialog_body_scroll_layout_with_content(
    controls: &DialogControls<'_>,
    content: &DialogContentLayout,
    surface_size: PhysicalSize<u32>,
    scroll_amount: f64,
) -> Option<DialogBodyScrollLayout> {
    let screen_width = i32::try_from(surface_size.width.max(1)).ok()?;
    let screen_height = i32::try_from(surface_size.height.max(1)).ok()?;
    let height = content.height.min(
        screen_height
            .saturating_sub(HEADER_HEIGHT)
            .saturating_sub(controls.footer_height),
    );
    let max_scroll = content.height.saturating_sub(height).max(0);
    let scroll_amount = if scroll_amount.is_finite() {
        scroll_amount.clamp(0.0, f64::from(max_scroll)) as i32
    } else {
        0
    };
    let width = content.width.saturating_add(BODY_SCROLLBAR_RESERVE * 2);
    Some(DialogBodyScrollLayout {
        rectangle: DialogFocusRectangle {
            x: screen_width / 2 - width / 2,
            y: dialog_content_y(screen_height, content.height, controls.footer_height),
            width,
            height,
        },
        content_height: content.height,
        max_scroll,
        scroll_amount,
    })
}

pub(super) fn dialog_scrollbar_layout(
    layout: DialogBodyScrollLayout,
) -> Option<DialogScrollbarLayout> {
    if layout.max_scroll <= 0 || layout.rectangle.height <= 8 || layout.content_height <= 0 {
        return None;
    }
    let max_thumb = layout.rectangle.height.saturating_sub(8).max(1);
    let min_thumb = 32.min(max_thumb);
    let thumb_height = layout
        .rectangle
        .height
        .saturating_mul(layout.rectangle.height)
        .div_euclid(layout.content_height)
        .clamp(min_thumb, max_thumb);
    let thumb_y = layout.rectangle.y.max(
        layout
            .scroll_amount
            .saturating_mul(layout.rectangle.height.saturating_sub(thumb_height))
            .div_euclid(layout.max_scroll)
            .saturating_add(layout.rectangle.y),
    );
    Some(DialogScrollbarLayout {
        x: layout
            .rectangle
            .x
            .saturating_add(layout.rectangle.width)
            .saturating_sub(6),
        y: layout.rectangle.y,
        width: 6,
        height: layout.rectangle.height,
        thumb_y,
        thumb_height,
    })
}

pub(super) fn dialog_body_viewport_contains(
    layout: DialogBodyScrollLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> bool {
    dialog_widget_contains(
        layout.rectangle.x,
        layout.rectangle.y,
        layout.rectangle.width,
        layout.rectangle.height,
        cursor_position,
        surface_size,
        screen_width,
        screen_height,
    )
}

pub(super) fn dialog_cursor_screen_y(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_height: u32,
) -> Option<i32> {
    let cursor_y = cursor_position?.y.floor();
    if !cursor_y.is_finite() {
        return None;
    }
    let (_, origin_y) =
        centered_screen_origin(surface_size, surface_size.width.max(1), screen_height);
    i32::try_from((cursor_y as i64).saturating_sub(origin_y)).ok()
}

pub(super) fn dialog_scrollbar_contains(
    scrollbar: DialogScrollbarLayout,
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
    screen_width: u32,
    screen_height: u32,
) -> bool {
    let Some(cursor) = cursor_position else {
        return false;
    };
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let left = origin_x + i64::from(scrollbar.x);
    let top = origin_y + i64::from(scrollbar.y);
    cursor.x >= left as f64
        && cursor.x <= (left + i64::from(scrollbar.width)) as f64
        && cursor.y >= top as f64
        && cursor.y < (top + i64::from(scrollbar.height)) as f64
}

impl DialogFocusRectangle {
    pub(super) fn axis_bound(self, horizontal: bool, positive: bool) -> i32 {
        let (start, length) = if horizontal {
            (self.x, self.width)
        } else {
            (self.y, self.height)
        };
        if positive {
            start.saturating_add(length).saturating_sub(1)
        } else {
            start
        }
    }

    pub(super) fn center(self, horizontal: bool) -> i32 {
        let low = self.axis_bound(horizontal, false);
        let high = self.axis_bound(horizontal, true);
        low.saturating_add(high) / 2
    }

    pub(super) fn overlaps_axis(self, other: Self, horizontal: bool) -> bool {
        self.axis_bound(horizontal, false)
            .max(other.axis_bound(horizontal, false))
            <= self
                .axis_bound(horizontal, true)
                .min(other.axis_bound(horizontal, true))
    }

    pub(super) fn border(self, direction: DialogFocusDirection) -> Self {
        match direction {
            DialogFocusDirection::Up => Self {
                x: self.x,
                y: self.y.saturating_sub(1),
                width: self.width,
                height: 1,
            },
            DialogFocusDirection::Down => Self {
                x: self.x,
                y: self.y.saturating_add(self.height),
                width: self.width,
                height: 1,
            },
            DialogFocusDirection::Left => Self {
                x: self.x.saturating_sub(1),
                y: self.y,
                width: 1,
                height: self.height,
            },
            DialogFocusDirection::Right => Self {
                x: self.x.saturating_add(self.width),
                y: self.y,
                width: 1,
                height: self.height,
            },
        }
    }
}

impl DialogFocusDirection {
    pub(super) fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    pub(super) fn positive(self) -> bool {
        matches!(self, Self::Down | Self::Right)
    }

    pub(super) fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub(super) fn is_after(self, value: i32, reference: i32) -> bool {
        if self.positive() {
            value > reference
        } else {
            value < reference
        }
    }
}

pub(super) fn ordered_dialog_focus_candidates<T: Copy>(
    focused: DialogFocusRectangle,
    direction: DialogFocusDirection,
    candidates: impl IntoIterator<Item = (T, DialogFocusRectangle)>,
) -> Vec<T> {
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    let horizontal = direction.horizontal();
    let other_horizontal = !horizontal;
    let focused_first = focused.axis_bound(horizontal, !direction.positive());
    let mut exact = candidates
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, (_, rectangle))| {
            if !rectangle.overlaps_axis(focused, other_horizontal) {
                return false;
            }
            let first = rectangle.axis_bound(horizontal, !direction.positive());
            direction.is_after(first, focused_first)
                || (first == focused_first
                    && direction.is_after(
                        rectangle.axis_bound(horizontal, direction.positive()),
                        focused.axis_bound(horizontal, direction.positive()),
                    ))
        })
        .collect::<Vec<_>>();
    exact.sort_by(|(left_index, (_, left)), (right_index, (_, right))| {
        let left_primary = left.axis_bound(horizontal, !direction.positive());
        let right_primary = right.axis_bound(horizontal, !direction.positive());
        let primary = if direction.positive() {
            left_primary.cmp(&right_primary)
        } else {
            right_primary.cmp(&left_primary)
        };
        primary
            .then_with(|| {
                left.axis_bound(other_horizontal, false)
                    .cmp(&right.axis_bound(other_horizontal, false))
            })
            .then_with(|| left_index.cmp(right_index))
    });
    let focused_axis = focused.axis_bound(horizontal, direction.positive());
    let focused_other = focused.center(other_horizontal);
    let mut vague = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, (target, rectangle))| {
            let candidate_axis = rectangle.axis_bound(horizontal, !direction.positive());
            if !direction.is_after(candidate_axis, focused_axis) {
                return None;
            }
            let candidate_other = rectangle.center(other_horizontal);
            let axis_delta = i64::from(candidate_axis) - i64::from(focused_axis);
            let other_delta = i64::from(candidate_other) - i64::from(focused_other);
            Some((
                index,
                target,
                axis_delta
                    .saturating_mul(axis_delta)
                    .saturating_add(other_delta.saturating_mul(other_delta)),
            ))
        })
        .collect::<Vec<_>>();
    vague.sort_by_key(|(index, _, distance)| (*distance, *index));
    exact
        .into_iter()
        .map(|(_, (target, _))| target)
        .chain(vague.into_iter().map(|(_, target, _)| target))
        .collect()
}

pub(super) fn dialog_focus_node(
    nodes: &[DialogFocusNode],
    target: DialogFocusState,
) -> Option<DialogFocusNode> {
    nodes.iter().copied().find(|node| node.target == target)
}

pub(super) fn next_body_focus_from_rectangle(
    graph: &DialogFocusGraph,
    focused: DialogFocusRectangle,
    direction: DialogFocusDirection,
    excluded: Option<DialogFocusState>,
) -> Option<DialogFocusState> {
    ordered_dialog_focus_candidates(
        focused,
        direction,
        graph.body.iter().filter_map(|node| {
            (Some(node.target) != excluded).then_some((node.target, node.rectangle))
        }),
    )
    .into_iter()
    .next()
}

pub(super) fn resolve_top_focus_candidates(
    graph: &DialogFocusGraph,
    focused: DialogFocusRectangle,
    direction: DialogFocusDirection,
    excluded: Option<DialogFocusState>,
    exclude_body: bool,
    body_previous_focus: Option<DialogFocusRectangle>,
) -> Option<DialogFocusState> {
    let candidates = graph
        .top
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            DialogTopFocusNode::Body(container) if !exclude_body && !graph.body.is_empty() => {
                Some((index, container.rectangle))
            }
            DialogTopFocusNode::Target(node) if Some(node.target) != excluded => {
                Some((index, node.rectangle))
            }
            _ => None,
        });
    for index in ordered_dialog_focus_candidates(focused, direction, candidates) {
        match graph.top[index] {
            DialogTopFocusNode::Target(node) => return Some(node.target),
            DialogTopFocusNode::Body(container) => {
                let body_focused = body_previous_focus
                    .unwrap_or_else(|| container.content_rectangle.border(direction.opposite()));
                if let Some(target) =
                    next_body_focus_from_rectangle(graph, body_focused, direction, None)
                {
                    return Some(target);
                }
            }
        }
    }
    None
}

pub(super) fn next_dialog_focus_in_direction(
    graph: &DialogFocusGraph,
    current: Option<DialogFocusState>,
    direction: DialogFocusDirection,
) -> Option<DialogFocusState> {
    if let Some(current) = current {
        if let Some(node) = dialog_focus_node(&graph.body, current) {
            let border = node.rectangle.border(direction);
            if let Some(target) =
                next_body_focus_from_rectangle(graph, border, direction, Some(current))
            {
                return Some(target);
            }
            return resolve_top_focus_candidates(
                graph,
                border,
                direction,
                None,
                true,
                Some(border),
            );
        }
        let top_node = graph.top.iter().find_map(|node| match node {
            DialogTopFocusNode::Target(node) if node.target == current => Some(*node),
            _ => None,
        })?;
        let border = top_node.rectangle.border(direction);
        return resolve_top_focus_candidates(
            graph,
            border,
            direction,
            Some(current),
            false,
            Some(border),
        );
    }

    let border = graph.screen.border(direction.opposite());
    resolve_top_focus_candidates(graph, border, direction, None, false, None)
}

pub(super) fn next_dialog_tab_focus(
    graph: &DialogFocusGraph,
    current: Option<DialogFocusState>,
    forward: bool,
) -> Option<DialogFocusState> {
    if graph.tab.is_empty() {
        return None;
    }
    let Some(index) =
        current.and_then(|current| graph.tab.iter().position(|node| *node == current))
    else {
        return if forward {
            graph.tab.first().copied()
        } else {
            graph.tab.last().copied()
        };
    };
    if forward {
        graph
            .tab
            .get(index.saturating_add(1))
            .copied()
            .or_else(|| graph.tab.first().copied())
    } else {
        index
            .checked_sub(1)
            .and_then(|index| graph.tab.get(index).copied())
            .or_else(|| graph.tab.last().copied())
    }
}

pub(super) fn dialog_single_line_text_box_layout(
    layout: DialogInputLayout,
    label_visible: bool,
) -> DialogInputLayout {
    DialogInputLayout {
        y: layout.y.saturating_add(if label_visible {
            TEXT_LINE_HEIGHT + 4
        } else {
            0
        }),
        height: TEXT_INPUT_HEIGHT,
        ..layout
    }
}

pub(super) fn dialog_multiline_text_box_layout(
    layout: DialogInputLayout,
    label_visible: bool,
) -> DialogInputLayout {
    let label_height = if label_visible {
        TEXT_LINE_HEIGHT + 4
    } else {
        0
    };
    DialogInputLayout {
        y: layout.y.saturating_add(label_height),
        height: layout.height.saturating_sub(label_height),
        ..layout
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DialogMultilineLine {
    pub(super) begin: usize,
    pub(super) end: usize,
}

pub(super) fn dialog_multiline_lines(
    text: impl Into<bbb_protocol::JavaString>,
    width: u32,
    text_measurer: &dyn DialogTextMeasurer,
) -> Vec<DialogMultilineLine> {
    let value = text.into();
    let projection = DialogTextRaw::new(&value);
    bbb_client_render::hud_plain_text_line_ranges(
        &projection.display,
        width,
        text_measurer.glyphs(),
    )
    .into_iter()
    .map(|range| DialogMultilineLine {
        begin: projection.unit_cursor(range.start),
        end: projection.unit_cursor(range.end),
    })
    .collect()
}

pub(super) fn dialog_multiline_content_height(line_count: usize) -> i32 {
    i32::try_from(line_count)
        .unwrap_or(i32::MAX)
        .saturating_mul(TEXT_LINE_HEIGHT)
        .saturating_add(8)
}

pub(super) fn dialog_multiline_max_scroll(line_count: usize, height: i32) -> i32 {
    dialog_multiline_content_height(line_count)
        .saturating_sub(height)
        .max(0)
}

pub(super) fn dialog_multiline_cursor_line(lines: &[DialogMultilineLine], cursor: usize) -> usize {
    lines
        .iter()
        .position(|line| cursor <= line.end)
        .unwrap_or_else(|| lines.len().saturating_sub(1))
}

pub(super) fn dialog_multiline_reveal_cursor(
    scroll_amount: i32,
    height: i32,
    lines: &[DialogMultilineLine],
    cursor: usize,
) -> i32 {
    let max_scroll = dialog_multiline_max_scroll(lines.len(), height);
    let scroll_amount = scroll_amount.clamp(0, max_scroll);
    let cursor_line = dialog_multiline_cursor_line(lines, cursor);
    let first_line = usize::try_from(scroll_amount / TEXT_LINE_HEIGHT).unwrap_or(0);
    if lines
        .get(first_line)
        .is_some_and(|line| cursor <= line.begin)
    {
        return i32::try_from(cursor_line)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT)
            .clamp(0, max_scroll);
    }
    let last_line = usize::try_from(
        scroll_amount
            .saturating_add(height)
            .saturating_div(TEXT_LINE_HEIGHT)
            .saturating_sub(1),
    )
    .unwrap_or(0)
    .min(lines.len().saturating_sub(1));
    if lines.get(last_line).is_some_and(|line| cursor > line.end) {
        return i32::try_from(cursor_line)
            .unwrap_or(i32::MAX)
            .saturating_mul(TEXT_LINE_HEIGHT)
            .saturating_sub(height)
            .saturating_add(TEXT_LINE_HEIGHT + 8)
            .clamp(0, max_scroll);
    }
    scroll_amount
}

pub(super) fn dialog_multiline_scrollbar_layout(
    layout: DialogInputLayout,
    line_count: usize,
    scroll_amount: i32,
) -> Option<DialogScrollbarLayout> {
    let content_height = dialog_multiline_content_height(line_count);
    let max_scroll = content_height.saturating_sub(layout.height).max(0);
    if max_scroll == 0 {
        return None;
    }
    let calculated_thumb_height = layout
        .height
        .saturating_mul(layout.height)
        .checked_div(content_height.max(1))
        .unwrap_or(0);
    let thumb_upper = layout.height.saturating_sub(8);
    let thumb_height = if thumb_upper < 32 {
        thumb_upper.max(1)
    } else {
        calculated_thumb_height.clamp(32, thumb_upper)
    };
    let travel = layout.height.saturating_sub(thumb_height);
    let thumb_y = layout.y.max(
        scroll_amount
            .clamp(0, max_scroll)
            .saturating_mul(travel)
            .checked_div(max_scroll)
            .unwrap_or(0)
            .saturating_add(layout.y),
    );
    Some(DialogScrollbarLayout {
        x: layout.x.saturating_add(layout.width),
        y: layout.y,
        width: 6,
        height: layout.height,
        thumb_y,
        thumb_height,
    })
}

pub(super) fn approximate_dialog_text_display_start(
    text: &str,
    scroll_to: usize,
    width: i32,
) -> usize {
    let scroll_to = scroll_to.min(text.chars().count());
    let prefix_width = text.chars().take(scroll_to).fold(0i32, |total, ch| {
        total.saturating_add(hud_ascii_approx_char_width(ch) as i32)
    });
    if prefix_width <= width {
        return 0;
    }
    let chars = text.chars().take(scroll_to).collect::<Vec<_>>();
    let mut start = chars.len();
    let mut used_width = 0i32;
    while start > 0 {
        let advance = hud_ascii_approx_char_width(chars[start - 1]) as i32;
        if used_width.saturating_add(advance) > width {
            break;
        }
        used_width = used_width.saturating_add(advance);
        start -= 1;
    }
    start
}

pub(super) fn approximate_dialog_text_cursor(
    text: &str,
    display_start: usize,
    width: i32,
) -> usize {
    let display_start = display_start.min(text.chars().count());
    let mut cursor = display_start;
    let mut used_width = 0i32;
    for ch in text.chars().skip(display_start) {
        let advance = hud_ascii_approx_char_width(ch) as i32;
        if used_width.saturating_add(advance) > width {
            break;
        }
        used_width = used_width.saturating_add(advance);
        cursor += 1;
    }
    cursor
}

pub(super) fn push_slider_track_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    layout: DialogInputLayout,
    highlighted: bool,
) {
    push_horizontal_nine_slice_layers(
        layers,
        layout,
        if highlighted {
            HudInventoryBackgroundTexture::Sprite("widget/slider_highlighted")
        } else {
            HudInventoryBackgroundTexture::Sprite("widget/slider")
        },
        WIDGET_SPRITE_WIDTH,
        1,
    );
}

pub(super) fn push_horizontal_nine_slice_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    layout: DialogInputLayout,
    texture: HudInventoryBackgroundTexture,
    sprite_width: u32,
    declared_border: u32,
) {
    let width = u32::try_from(layout.width).unwrap_or(0);
    let height = u32::try_from(layout.height).unwrap_or(0);
    if width == 0 || height == 0 || sprite_width == 0 {
        return;
    }
    // These widget sprites keep their declared 20px target height, so vanilla's nine regions
    // collapse to three exact horizontal columns while retaining the clamped left/right borders.
    let border = declared_border.min(width / 2);
    let columns = [
        (0, border, 0, border),
        (
            border,
            width.saturating_sub(border.saturating_mul(2)),
            border,
            sprite_width.saturating_sub(border.saturating_mul(2)),
        ),
        (
            width.saturating_sub(border),
            border,
            sprite_width.saturating_sub(border),
            border,
        ),
    ];
    for (screen_x, screen_width, texture_x, texture_width) in columns {
        if screen_width == 0 || texture_width == 0 {
            continue;
        }
        layers.push(HudInventoryBackgroundLayer {
            texture,
            x: layout
                .x
                .saturating_add(i32::try_from(screen_x).unwrap_or(i32::MAX)),
            y: layout.y,
            width: screen_width,
            height,
            uv: HudUvRect {
                min: [texture_x as f32 / sprite_width as f32, 0.0],
                max: [
                    texture_x.saturating_add(texture_width) as f32 / sprite_width as f32,
                    1.0,
                ],
            },
        });
    }
}

pub(super) fn push_full_nine_slice_layers(
    layers: &mut Vec<HudInventoryBackgroundLayer>,
    layout: DialogInputLayout,
    texture: HudInventoryBackgroundTexture,
    sprite_width: u32,
    sprite_height: u32,
    declared_border: u32,
) {
    let width = u32::try_from(layout.width).unwrap_or(0);
    let height = u32::try_from(layout.height).unwrap_or(0);
    if width == 0 || height == 0 || sprite_width == 0 || sprite_height == 0 {
        return;
    }
    let border_x = declared_border.min(width / 2).min(sprite_width / 2);
    let border_y = declared_border.min(height / 2).min(sprite_height / 2);
    let columns = [
        (0, border_x, 0, border_x),
        (
            border_x,
            width.saturating_sub(border_x.saturating_mul(2)),
            border_x,
            sprite_width.saturating_sub(border_x.saturating_mul(2)),
        ),
        (
            width.saturating_sub(border_x),
            border_x,
            sprite_width.saturating_sub(border_x),
            border_x,
        ),
    ];
    let rows = [
        (0, border_y, 0, border_y),
        (
            border_y,
            height.saturating_sub(border_y.saturating_mul(2)),
            border_y,
            sprite_height.saturating_sub(border_y.saturating_mul(2)),
        ),
        (
            height.saturating_sub(border_y),
            border_y,
            sprite_height.saturating_sub(border_y),
            border_y,
        ),
    ];
    for (screen_y, screen_height, texture_y, texture_height) in rows {
        for (screen_x, screen_width, texture_x, texture_width) in columns {
            if screen_width == 0 || screen_height == 0 || texture_width == 0 || texture_height == 0
            {
                continue;
            }
            layers.push(HudInventoryBackgroundLayer {
                texture,
                x: layout
                    .x
                    .saturating_add(i32::try_from(screen_x).unwrap_or(i32::MAX)),
                y: layout
                    .y
                    .saturating_add(i32::try_from(screen_y).unwrap_or(i32::MAX)),
                width: screen_width,
                height: screen_height,
                uv: HudUvRect {
                    min: [
                        texture_x as f32 / sprite_width as f32,
                        texture_y as f32 / sprite_height as f32,
                    ],
                    max: [
                        texture_x.saturating_add(texture_width) as f32 / sprite_width as f32,
                        texture_y.saturating_add(texture_height) as f32 / sprite_height as f32,
                    ],
                },
            });
        }
    }
}

pub(super) fn dialog_number_range_scaled_value(
    range: &DialogNumberRangeState,
    slider_value: f64,
) -> f32 {
    let start = range.start();
    let end = range.end();
    let value_in_range = start + slider_value as f32 * (end - start);
    let Some(step) = range.step() else {
        return value_in_range;
    };
    let initial = range.initial().unwrap_or((start + end) / 2.0);
    let steps = ((value_in_range - initial) / step + 0.5).floor() as i32;
    let stepped_delta = steps as f32 * step;
    let result = initial + stepped_delta;
    let slider_position = if start == end {
        0.5
    } else {
        (result - start) / (end - start)
    };
    if slider_position < 0.0 || slider_position > 1.0 {
        let one_step_less = steps - steps.signum();
        initial + one_step_less as f32 * step
    } else {
        result
    }
}

pub(super) fn dialog_number_range_value_string(value: f32) -> String {
    let integer = value as i32;
    if integer as f32 == value {
        integer.to_string()
    } else {
        java_float_to_string(value)
    }
}

#[cfg(test)]
pub(super) fn dialog_number_range_label_runs(
    template: &str,
    label: &DialogTextState,
    value: &str,
) -> Vec<HudStyledTextRun> {
    dialog_number_range_label_runs_with(template, label, value, None)
}

pub(super) fn dialog_number_range_label_runs_with(
    template: &str,
    label: &DialogTextState,
    value: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<HudStyledTextRun> {
    dialog_translated_two_arg_runs(
        template,
        &dialog_text_runs_with(label, item_runtime),
        &[HudStyledTextRun::plain(value)],
    )
}

pub(super) fn dialog_translated_two_arg_runs(
    template: &str,
    first: &[HudStyledTextRun],
    second: &[HudStyledTextRun],
) -> Vec<HudStyledTextRun> {
    let mut runs = Vec::new();
    let mut current = 0;
    let mut replacement_index = 0usize;
    while let Some(offset) = template[current..].find('%') {
        let percent = current + offset;
        append_plain_dialog_run(&mut runs, &template[current..percent]);
        let bytes = template.as_bytes();
        let mut next = percent + 1;
        if bytes.get(next) == Some(&b'%') {
            append_plain_dialog_run(&mut runs, "%");
            current = next + 1;
            continue;
        }

        let digit_start = next;
        while bytes.get(next).is_some_and(u8::is_ascii_digit) {
            next += 1;
        }
        let explicit_index = if next > digit_start {
            if bytes.get(next) != Some(&b'$') {
                return vec![HudStyledTextRun::plain(template)];
            }
            let Ok(index) = template[digit_start..next].parse::<usize>() else {
                return vec![HudStyledTextRun::plain(template)];
            };
            next += 1;
            let Some(index) = index.checked_sub(1) else {
                return vec![HudStyledTextRun::plain(template)];
            };
            Some(index)
        } else {
            None
        };
        if bytes.get(next) != Some(&b's') {
            return vec![HudStyledTextRun::plain(template)];
        }
        let index = explicit_index.unwrap_or_else(|| {
            let index = replacement_index;
            replacement_index = replacement_index.saturating_add(1);
            index
        });
        match index {
            0 => append_dialog_runs(&mut runs, first),
            1 => append_dialog_runs(&mut runs, second),
            _ => return vec![HudStyledTextRun::plain(template)],
        }
        current = next + 1;
    }
    append_plain_dialog_run(&mut runs, &template[current..]);
    runs
}

pub(super) fn append_dialog_runs(output: &mut Vec<HudStyledTextRun>, runs: &[HudStyledTextRun]) {
    for run in runs {
        if let Some(previous) = output.last_mut().filter(|previous| {
            previous.style == run.style
                && previous.color == run.color
                && previous.shadow_color == run.shadow_color
        }) {
            previous.text.push_str(&run.text);
        } else {
            output.push(run.clone());
        }
    }
}

pub(super) fn append_plain_dialog_run(output: &mut Vec<HudStyledTextRun>, text: &str) {
    if text.is_empty() {
        return;
    }
    let run = HudStyledTextRun::plain(text);
    append_dialog_runs(output, std::slice::from_ref(&run));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WaitingLayout {
    pub(super) screen_width: u32,
    pub(super) screen_height: u32,
    pub(super) button_x: i32,
    pub(super) button_y: i32,
}

impl WaitingLayout {
    pub(super) fn new(surface_size: PhysicalSize<u32>) -> Self {
        let screen_width = surface_size.width.max(1);
        let screen_height = surface_size.height.max(1);
        let screen_width_i32 = i32::try_from(screen_width).unwrap_or(i32::MAX);
        let screen_height_i32 = i32::try_from(screen_height).unwrap_or(i32::MAX);
        Self {
            screen_width,
            screen_height,
            button_x: (screen_width_i32 - WAITING_BUTTON_WIDTH as i32) / 2,
            button_y: (HEADER_HEIGHT + CONTENT_MARGIN_TOP)
                .min(screen_height_i32 - BUTTON_HEIGHT as i32),
        }
    }

    pub(super) fn button_contains(
        self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        screen_button_contains(
            self.screen_width,
            self.screen_height,
            self.button_x,
            self.button_y,
            WAITING_BUTTON_WIDTH,
            cursor_position,
            surface_size,
        )
    }
}

#[cfg(test)]
mod scrollbar_tests {
    use super::*;

    #[test]
    fn dialog_scrollbar_layout_does_not_panic_for_short_viewport_heights() {
        for height in 9..45 {
            let scrollbar = dialog_scrollbar_layout(DialogBodyScrollLayout {
                rectangle: DialogFocusRectangle {
                    x: 10,
                    y: 20,
                    width: 200,
                    height,
                },
                content_height: 400,
                max_scroll: 350,
                scroll_amount: 100,
            })
            .expect("scrollbar layout exists");
            assert!(scrollbar.thumb_height >= 1);
            assert!(scrollbar.thumb_height <= height - 8);
        }
    }
}
