use super::*;
use crate::input::dialog::DialogBodyFocusPartState;
use crate::input::dialog::DialogFocusState;
use crate::input::dialog::DialogInputValueState;
use crate::input::dialog::DialogLinkConfirmationState;

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(in crate::runtime) fn hud_dialog_screen(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
    dialog: &DialogState,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_state: &DialogHudLocalState,
    surface_size: PhysicalSize<u32>,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    hud_dialog_screen_with_measurer(
        input,
        world,
        dialog,
        item_runtime,
        terrain_textures,
        local_state,
        surface_size,
        &ApproximateDialogTextMeasurer,
        partial_tick,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::runtime) fn hud_dialog_screen_with_measurer(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
    dialog: &DialogState,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    local_state: &DialogHudLocalState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
    partial_tick: f32,
) -> Option<HudInventoryScreen> {
    let text_ctx = DialogTextContext::from_measurer(item_runtime, text_measurer);
    let controls = dialog_controls(dialog.data.as_ref()?, world.server_links())?;
    let layout = DialogFooterLayout::new(
        &controls
            .footer_actions()
            .iter()
            .map(|action| action.width())
            .collect::<Vec<_>>(),
        controls.footer_height,
        surface_size,
    )?;
    let screen_width_i32 = i32::try_from(layout.screen_width).ok()?;
    let screen_height_i32 = i32::try_from(layout.screen_height).ok()?;
    let content = dialog_content_layout_with(&controls, text_ctx)?;
    let scroll_layout = dialog_body_scroll_layout_with_content(
        &controls,
        &content,
        surface_size,
        input.dialog_body_scroll_amount(),
    )?;
    let cursor_screen_position = local_state.cursor_position;
    let cursor_position = local_state
        .cursor_position
        .map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted_footer_button = layout
        .button_at(cursor_position, surface_size)
        .map(|index| controls.body_action_count.saturating_add(index));
    let warning_button =
        DialogWarningButtonLayout::new_with(&controls.common.title, surface_size, text_ctx)?;
    let warning_button_hovered = controls.show_server_dialog_warning()
        && warning_button.contains(cursor_position, surface_size);
    let warning_button_focused = controls.show_server_dialog_warning()
        && input.dialog_focus() == Some(DialogFocusState::WarningButton);
    let focused_action = match input.dialog_focus() {
        Some(DialogFocusState::Action { index }) => Some(index),
        _ => None,
    };

    let mut text_labels = Vec::new();
    let title_x = if controls.show_server_dialog_warning() {
        warning_button.title_x
    } else {
        let title_runs = dialog_text_runs_with(&controls.common.title, text_ctx.item_runtime);
        (screen_width_i32 - i32::try_from(runs_width(&title_runs, text_ctx.glyphs)).ok()?) / 2
    };
    push_text_label(
        &mut text_labels,
        &controls.common.title,
        text_ctx,
        title_x,
        warning_button.title_y,
    );
    let scroll_text_start = text_labels.len();
    let item_models = controls
        .common
        .body_data
        .iter()
        .any(|body| matches!(body, DialogBodyState::Item { .. }))
        .then(|| HudItemModelContext::new(world, item_runtime));

    let body_blocks = controls
        .common
        .body_data
        .iter()
        .enumerate()
        .filter_map(|(body_index, body)| match body {
            DialogBodyState::PlainMessage { message } => dialog_text_block_with(message, text_ctx)
                .map(DialogBodyBlock::Text)
                .map(|block| (body_index, block)),
            DialogBodyState::Item {
                item,
                description,
                show_decorations,
                show_tooltip,
                width,
                height,
            } => dialog_item_block(
                world,
                item_models.as_ref()?,
                text_ctx,
                terrain_textures,
                local_state,
                item,
                description.as_ref(),
                *show_decorations,
                *show_tooltip,
                *width,
                *height,
                partial_tick,
            )
            .map(DialogBodyBlock::Item)
            .map(|block| (body_index, block)),
        })
        .collect::<Vec<_>>();
    let content_y = dialog_content_y(screen_height_i32, content.height, controls.footer_height);
    let mut body_y = content_y.saturating_sub(scroll_layout.scroll_amount);
    let mut floating_items = Vec::new();
    let mut scroll_fill_layers = Vec::new();
    let mut item_tooltip = None;
    for (render_index, (body_index, block)) in body_blocks.into_iter().enumerate() {
        if render_index > 0 {
            body_y = body_y.saturating_add(BODY_SPACING);
        }
        let block_x = screen_width_i32 / 2 - block.width() / 2;
        let block_height = block.height();
        match block {
            DialogBodyBlock::Text(text) => {
                if input.dialog_focus()
                    == Some(DialogFocusState::Body {
                        index: body_index,
                        part: DialogBodyFocusPartState::Primary,
                    })
                {
                    push_dialog_outline_layers(
                        &mut scroll_fill_layers,
                        block_x,
                        body_y,
                        i32::try_from(text.width).unwrap_or(i32::MAX),
                        text.height,
                    );
                }
                push_dialog_text_block(&mut text_labels, text, block_x, body_y);
            }
            DialogBodyBlock::Item(mut item) => {
                let item_y =
                    body_y.saturating_add(vertical_center_offset(block_height, item.widget_height));
                item.item.x = block_x;
                item.item.y = item_y;
                item.item.scissor = Some(HudInventoryItemScissor {
                    x: scroll_layout.rectangle.x,
                    y: scroll_layout.rectangle.y,
                    width: u32::try_from(scroll_layout.rectangle.width).unwrap_or_default(),
                    height: u32::try_from(scroll_layout.rectangle.height).unwrap_or_default(),
                });
                if input.dialog_focus()
                    == Some(DialogFocusState::Body {
                        index: body_index,
                        part: DialogBodyFocusPartState::Primary,
                    })
                {
                    push_dialog_outline_layers(
                        &mut scroll_fill_layers,
                        block_x,
                        item_y,
                        item.widget_width,
                        item.widget_height,
                    );
                }
                if item.show_tooltip
                    && dialog_body_viewport_contains(
                        scroll_layout,
                        cursor_position,
                        surface_size,
                        layout.screen_width,
                        layout.screen_height,
                    )
                    && dialog_widget_contains(
                        block_x,
                        item_y,
                        item.widget_width,
                        item.widget_height,
                        cursor_position,
                        surface_size,
                        layout.screen_width,
                        layout.screen_height,
                    )
                {
                    item_tooltip = dialog_item_tooltip(
                        world,
                        item_runtime,
                        &item.stack,
                        local_state.advanced_item_tooltips,
                        cursor_screen_position,
                        surface_size,
                        layout.screen_width,
                        layout.screen_height,
                        body_index,
                    );
                }
                floating_items.push(item.item);
                if let Some(description) = item.description {
                    let description_y = body_y
                        .saturating_add(vertical_center_offset(block_height, description.height));
                    let description_x = block_x
                        .saturating_add(item.widget_width)
                        .saturating_add(ITEM_DESCRIPTION_SPACING);
                    if input.dialog_focus()
                        == Some(DialogFocusState::Body {
                            index: body_index,
                            part: DialogBodyFocusPartState::Description,
                        })
                    {
                        push_dialog_outline_layers(
                            &mut scroll_fill_layers,
                            description_x,
                            description_y,
                            i32::try_from(description.width).unwrap_or(i32::MAX),
                            description.height,
                        );
                    }
                    push_dialog_text_block(
                        &mut text_labels,
                        description,
                        description_x,
                        description_y,
                    );
                }
            }
        }
        body_y = body_y.saturating_add(block_height);
    }

    let input_layouts = dialog_input_layouts_with_content(
        &controls,
        &content,
        surface_size,
        scroll_layout.scroll_amount,
    )?;
    let mut input_layers = Vec::new();
    let mut text_areas = Vec::new();
    for &input_layout in &input_layouts {
        let Some(definition) = controls.common.input_data.get(input_layout.index) else {
            continue;
        };
        let hovered = dialog_body_viewport_contains(
            scroll_layout,
            cursor_position,
            surface_size,
            layout.screen_width,
            layout.screen_height,
        ) && dialog_widget_contains(
            input_layout.x,
            input_layout.y,
            input_layout.width,
            input_layout.height,
            cursor_position,
            surface_size,
            layout.screen_width,
            layout.screen_height,
        );
        let focused = input.dialog_focused_input() == Some(input_layout.index);
        match (
            &definition.control,
            input.current_dialog_input_values().get(input_layout.index),
        ) {
            (
                DialogInputControlState::Boolean { label, .. },
                Some(Some(DialogInputValueState::Boolean { selected })),
            ) => {
                input_layers.push(HudInventoryBackgroundLayer {
                    texture: match (*selected, hovered || focused) {
                        (false, false) => HudInventoryBackgroundTexture::Sprite("widget/checkbox"),
                        (false, true) => {
                            HudInventoryBackgroundTexture::Sprite("widget/checkbox_highlighted")
                        }
                        (true, false) => {
                            HudInventoryBackgroundTexture::Sprite("widget/checkbox_selected")
                        }
                        (true, true) => HudInventoryBackgroundTexture::Sprite(
                            "widget/checkbox_selected_highlighted",
                        ),
                    },
                    x: input_layout.x,
                    y: input_layout.y,
                    width: CHECKBOX_SIZE,
                    height: CHECKBOX_SIZE,
                    uv: HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                });
                push_text_label(
                    &mut text_labels,
                    label,
                    text_ctx,
                    input_layout
                        .x
                        .saturating_add(i32::try_from(CHECKBOX_SIZE).unwrap_or(i32::MAX))
                        .saturating_add(CHECKBOX_LABEL_SPACING),
                    input_layout.y.saturating_add(
                        (i32::try_from(CHECKBOX_SIZE).unwrap_or(i32::MAX) - TEXT_LINE_HEIGHT) / 2,
                    ),
                );
            }
            (
                DialogInputControlState::NumberRange {
                    label,
                    label_format,
                    range,
                    ..
                },
                Some(Some(DialogInputValueState::NumberRange { slider_value_bits })),
            ) => {
                let keyboard_can_change =
                    input.dialog_number_range_keyboard_can_change(input_layout.index);
                push_slider_track_layers(
                    &mut input_layers,
                    input_layout,
                    focused && !keyboard_can_change,
                );
                let slider_value = f64::from_bits(*slider_value_bits);
                let handle_offset =
                    (slider_value
                        * f64::from(input_layout.width.saturating_sub(
                            i32::try_from(SLIDER_HANDLE_WIDTH).unwrap_or(i32::MAX),
                        ))) as i32;
                input_layers.push(HudInventoryBackgroundLayer {
                    texture: if hovered
                        || keyboard_can_change
                        || input.dialog_dragging_number_range_input() == Some(input_layout.index)
                    {
                        HudInventoryBackgroundTexture::Sprite("widget/slider_handle_highlighted")
                    } else {
                        HudInventoryBackgroundTexture::Sprite("widget/slider_handle")
                    },
                    x: input_layout.x.saturating_add(handle_offset),
                    y: input_layout.y,
                    width: SLIDER_HANDLE_WIDTH,
                    height: SLIDER_HEIGHT,
                    uv: HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                });
                let value = dialog_number_range_scaled_value(range, slider_value);
                let value = dialog_number_range_value_string(value);
                let label_format = label_format.to_display_string();
                let template = match item_runtime {
                    Some(runtime) => runtime.localized_text(&label_format),
                    None if label_format == "options.generic_value" => "%s: %s",
                    None => &label_format,
                };
                let runs = dialog_number_range_label_runs_with(
                    template,
                    label,
                    &value,
                    text_ctx.item_runtime,
                );
                push_scrolling_widget_text_label(
                    &mut text_labels,
                    runs,
                    input_layout,
                    text_ctx.glyphs,
                );
            }
            (
                DialogInputControlState::SingleOption {
                    options,
                    label,
                    label_visible,
                    ..
                },
                Some(Some(DialogInputValueState::SingleOption { selected_index })),
            ) => {
                let Some(selected) = options.get(*selected_index) else {
                    continue;
                };
                push_horizontal_nine_slice_layers(
                    &mut input_layers,
                    input_layout,
                    if hovered || focused {
                        HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
                    } else {
                        HudInventoryBackgroundTexture::Sprite("widget/button")
                    },
                    WIDGET_SPRITE_WIDTH,
                    3,
                );
                let value_runs = selected.display.as_ref().map_or_else(
                    || vec![HudStyledTextRun::plain(selected.id.to_display_string())],
                    |display| dialog_text_runs_with(display, text_ctx.item_runtime),
                );
                let runs = if *label_visible {
                    let template = match item_runtime {
                        Some(runtime) => runtime.localized_text("options.generic_value"),
                        None => "%s: %s",
                    };
                    dialog_translated_two_arg_runs(
                        template,
                        &dialog_text_runs_with(label, text_ctx.item_runtime),
                        &value_runs,
                    )
                } else {
                    value_runs
                };
                push_scrolling_widget_text_label(
                    &mut text_labels,
                    runs,
                    input_layout,
                    text_ctx.glyphs,
                );
            }
            (
                DialogInputControlState::Text {
                    label,
                    label_visible,
                    max_length,
                    multiline: None,
                    ..
                },
                Some(Some(DialogInputValueState::Text { value })),
            ) => {
                if *label_visible {
                    push_text_label(
                        &mut text_labels,
                        label,
                        text_ctx,
                        input_layout.x,
                        input_layout.y,
                    );
                }
                let box_layout = dialog_single_line_text_box_layout(input_layout, *label_visible);
                let edit = input
                    .dialog_text_edit_state()
                    .filter(|edit| edit.input_index == input_layout.index);
                push_horizontal_nine_slice_layers(
                    &mut input_layers,
                    box_layout,
                    if edit.is_some() {
                        HudInventoryBackgroundTexture::Sprite("widget/text_field_highlighted")
                    } else {
                        HudInventoryBackgroundTexture::Sprite("widget/text_field")
                    },
                    WIDGET_SPRITE_WIDTH,
                    1,
                );
                let inner_width = u32::try_from(
                    box_layout
                        .width
                        .saturating_sub(TEXT_INPUT_INSET.saturating_mul(2))
                        .max(1),
                )
                .unwrap_or(1);
                let projection = DialogTextRaw::new(value);
                let text_len = value.units().len();
                let cursor = edit.map_or(text_len, |edit| edit.cursor);
                let selection = edit.map_or(text_len, |edit| edit.selection);
                text_labels.push(HudInventoryTextLabel {
                    component_runs: Vec::new(),
                    x: box_layout.x.saturating_add(TEXT_INPUT_INSET),
                    y: box_layout.y.saturating_add(TEXT_INPUT_Y_OFFSET),
                    width: inner_width,
                    text: projection.display.clone(),
                    tint: TEXT_INPUT_COLOR,
                    background: None,
                    input: Some(HudInventoryTextInputDecoration {
                        cursor,
                        selection,
                        scroll_to: if selection != cursor {
                            selection
                        } else {
                            cursor
                        },
                        max_length: usize::try_from(*max_length).unwrap_or(usize::MAX),
                        cursor_visible: edit
                            .is_some_and(|edit| caret_visible(edit.focused_at, clock_millis())),
                        cursor_tint: TEXT_INPUT_COLOR,
                    }),
                    scrolling: None,
                    shadow: true,
                    font: None,
                    runs: Vec::new(),
                });
            }
            (
                DialogInputControlState::Text {
                    label,
                    label_visible,
                    max_length,
                    multiline: Some(_),
                    ..
                },
                Some(Some(DialogInputValueState::Text { value })),
            ) => {
                if *label_visible {
                    push_text_label(
                        &mut text_labels,
                        label,
                        text_ctx,
                        input_layout.x,
                        input_layout.y,
                    );
                }
                let box_layout = dialog_multiline_text_box_layout(input_layout, *label_visible);
                let edit = input.dialog_text_edit_state_for_input(input_layout.index);
                let focused = input.dialog_focused_input() == Some(input_layout.index);
                push_full_nine_slice_layers(
                    &mut input_layers,
                    box_layout,
                    if focused {
                        HudInventoryBackgroundTexture::Sprite("widget/text_field_highlighted")
                    } else {
                        HudInventoryBackgroundTexture::Sprite("widget/text_field")
                    },
                    WIDGET_SPRITE_WIDTH,
                    20,
                    1,
                );
                let projection = DialogTextRaw::new(value);
                let text_len = value.units().len();
                text_areas.push(HudInventoryTextArea {
                    needs_cursor_position: focused
                        && local_state
                            .dialog_ime_preedit
                            .as_ref()
                            .is_some_and(|preedit| {
                                preedit.screen_revision == input.dialog_screen_revision()
                                    && !preedit.text.is_empty()
                            }),
                    x: box_layout.x,
                    y: box_layout.y,
                    width: u32::try_from(box_layout.width).unwrap_or_default(),
                    height: u32::try_from(box_layout.height).unwrap_or_default(),
                    text: projection.display.clone(),
                    cursor: edit.map_or(text_len, |edit| edit.cursor),
                    selection: edit.map_or(text_len, |edit| edit.selection),
                    scroll_amount: edit.map_or(i32::MAX, |edit| edit.scroll_amount),
                    max_length: usize::try_from(*max_length).unwrap_or(usize::MAX),
                    cursor_visible: focused
                        && edit.is_some_and(|edit| caret_visible(edit.focused_at, clock_millis())),
                    text_tint: TEXT_INPUT_COLOR,
                    cursor_tint: TEXT_INPUT_COLOR,
                    counter_tint: INACTIVE_TEXT_COLOR,
                    shadow: true,
                });
            }
            _ => {}
        }
    }

    let mut body_action_buttons = Vec::new();
    let mut highlighted_body_action = None;
    if let Some(action_grid) = content.action_grid {
        let grid_x = screen_width_i32 / 2 - action_grid.width / 2;
        let grid_y = content_y
            .saturating_sub(scroll_layout.scroll_amount)
            .saturating_add(content.height.saturating_sub(action_grid.height));
        for (index, (action, button)) in controls
            .body_actions()
            .iter()
            .zip(action_grid.buttons)
            .enumerate()
        {
            let button = DialogButtonLayout {
                x: grid_x.saturating_add(button.x),
                y: grid_y.saturating_add(button.y),
                width: button.width,
            };
            let hovered = dialog_body_viewport_contains(
                scroll_layout,
                cursor_position,
                surface_size,
                layout.screen_width,
                layout.screen_height,
            ) && dialog_widget_contains(
                button.x,
                button.y,
                i32::try_from(button.width).unwrap_or(i32::MAX),
                i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                cursor_position,
                surface_size,
                layout.screen_width,
                layout.screen_height,
            );
            if hovered {
                highlighted_body_action = Some(index);
            }
            push_horizontal_nine_slice_layers(
                &mut input_layers,
                DialogInputLayout {
                    index,
                    x: button.x,
                    y: button.y,
                    width: i32::try_from(button.width).unwrap_or(i32::MAX),
                    height: i32::try_from(BUTTON_HEIGHT).unwrap_or(i32::MAX),
                },
                if hovered || focused_action == Some(index) {
                    HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
                } else {
                    HudInventoryBackgroundTexture::Sprite("widget/button")
                },
                WIDGET_SPRITE_WIDTH,
                3,
            );
            push_centered_dialog_control_label(
                &mut text_labels,
                *action,
                text_ctx,
                button.x + i32::try_from(button.width).unwrap_or(i32::MAX) / 2,
                button.y + BUTTON_TEXT_Y_OFFSET,
            );
            body_action_buttons.push(button);
        }
    }

    let scroll_text_labels = text_labels.split_off(scroll_text_start);

    for (action, button) in controls.footer_actions().iter().zip(&layout.buttons) {
        push_centered_dialog_control_label(
            &mut text_labels,
            *action,
            text_ctx,
            button.x + i32::try_from(button.width).unwrap_or(i32::MAX) / 2,
            button.y + BUTTON_TEXT_Y_OFFSET,
        );
    }

    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    if controls.show_server_dialog_warning() {
        background_layers.push(HudInventoryBackgroundLayer {
            texture: if warning_button_hovered || warning_button_focused {
                HudInventoryBackgroundTexture::Sprite("dialog/warning_button_highlighted")
            } else {
                HudInventoryBackgroundTexture::Sprite("dialog/warning_button")
            },
            x: warning_button.x,
            y: warning_button.y,
            width: u32::try_from(WARNING_BUTTON_SIZE).unwrap_or_default(),
            height: u32::try_from(WARNING_BUTTON_SIZE).unwrap_or_default(),
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        });
    }
    background_layers.extend(layout.buttons.iter().enumerate().map(|(index, button)| {
        let action_index = controls.body_action_count.saturating_add(index);
        HudInventoryBackgroundLayer {
            texture: if highlighted_footer_button == Some(action_index)
                || focused_action == Some(action_index)
            {
                HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
            } else {
                HudInventoryBackgroundTexture::Sprite("widget/button")
            },
            x: button.x,
            y: button.y,
            width: button.width,
            height: BUTTON_HEIGHT,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        }
    }));
    let highlighted_action = highlighted_body_action.or(highlighted_footer_button);
    let action_tooltip = highlighted_action.or(focused_action).and_then(|index| {
        let tooltip = controls.actions.get(index)?.tooltip()?;
        let button = if index < controls.body_action_count {
            body_action_buttons.get(index)?
        } else {
            layout
                .buttons
                .get(index.saturating_sub(controls.body_action_count))?
        };
        let (cursor_x, cursor_y) = cursor_screen_position.unwrap_or_else(|| {
            let (origin_x, origin_y) =
                centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
            (
                i32::try_from(origin_x + i64::from(button.x)).unwrap_or(i32::MAX),
                i32::try_from(origin_y + i64::from(button.y) + i64::from(BUTTON_HEIGHT))
                    .unwrap_or(i32::MAX),
            )
        });
        let (origin_x, origin_y) =
            centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
        Some(HudInventoryTooltip {
            slot_id: u16::try_from(index).unwrap_or(u16::MAX),
            x: i32::try_from(i64::from(cursor_x) - origin_x).unwrap_or_else(|_| {
                if i64::from(cursor_x) < origin_x {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }),
            y: i32::try_from(i64::from(cursor_y) - origin_y).unwrap_or_else(|_| {
                if i64::from(cursor_y) < origin_y {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }),
            style: None,
            lines: wrapped_text_lines_with(tooltip, 170, text_ctx)
                .into_iter()
                .map(|line| HudInventoryTooltipLine {
                    text: line.text,
                    tint: TEXT_COLOR,
                    runs: line
                        .runs
                        .into_iter()
                        .map(HudComponentTextRun::from)
                        .collect(),
                })
                .collect(),
        })
    });
    let warning_tooltip = (warning_button_hovered || warning_button_focused).then(|| {
        let (cursor_x, cursor_y) = cursor_screen_position.unwrap_or_else(|| {
            let (origin_x, origin_y) =
                centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
            (
                i32::try_from(origin_x + i64::from(warning_button.x)).unwrap_or(i32::MAX),
                i32::try_from(
                    origin_y + i64::from(warning_button.y) + i64::from(WARNING_BUTTON_SIZE),
                )
                .unwrap_or(i32::MAX),
            )
        });
        let (origin_x, origin_y) =
            centered_screen_origin(surface_size, layout.screen_width, layout.screen_height);
        HudInventoryTooltip {
            slot_id: u16::MAX,
            x: i32::try_from(i64::from(cursor_x) - origin_x).unwrap_or_else(|_| {
                if i64::from(cursor_x) < origin_x {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }),
            y: i32::try_from(i64::from(cursor_y) - origin_y).unwrap_or_else(|_| {
                if i64::from(cursor_y) < origin_y {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }),
            style: None,
            lines: plain_dialog_text_lines_with(DIALOG_WARNING_TOOLTIP, 170, text_ctx.glyphs)
                .into_iter()
                .map(|line| HudInventoryTooltipLine {
                    text: line.text,
                    tint: TEXT_COLOR,
                    runs: line
                        .runs
                        .into_iter()
                        .map(HudComponentTextRun::from)
                        .collect(),
                })
                .collect(),
        }
    });
    let preedit_overlay = local_state.dialog_ime_preedit.as_ref().and_then(|preedit| {
        dialog_preedit_projection_with_inputs(
            input,
            world,
            &controls,
            &input_layouts,
            preedit,
            surface_size,
            text_measurer,
        )
        .map(|projection| projection.overlay)
    });

    Some(HudInventoryScreen {
        width: layout.screen_width,
        height: layout.screen_height,
        background_layers,
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items,
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels,
        scroll_area: Some(HudInventoryScrollArea {
            scissor: HudInventoryItemScissor {
                x: scroll_layout.rectangle.x,
                y: scroll_layout.rectangle.y,
                width: u32::try_from(scroll_layout.rectangle.width).unwrap_or_default(),
                height: u32::try_from(scroll_layout.rectangle.height).unwrap_or_default(),
            },
            background_layers: input_layers,
            fill_layers: scroll_fill_layers,
            text_labels: scroll_text_labels,
            text_areas,
            scrollbar: dialog_scrollbar_layout(scroll_layout).map(|scrollbar| {
                HudInventoryScrollbar {
                    x: scrollbar.x,
                    y: scrollbar.y,
                    width: u32::try_from(scrollbar.width).unwrap_or_default(),
                    height: u32::try_from(scrollbar.height).unwrap_or_default(),
                    thumb_y: scrollbar.thumb_y,
                    thumb_height: u32::try_from(scrollbar.thumb_height).unwrap_or_default(),
                }
            }),
        }),
        preedit_overlay,
        hovered_slot_id: None,
        tooltip: warning_tooltip.or(action_tooltip).or(item_tooltip),
    })
}

pub(super) struct DialogPreeditProjection {
    pub(super) overlay: HudInventoryPreeditOverlay,
    pub(super) ime_position: PhysicalPosition<i32>,
    pub(super) ime_size: PhysicalSize<u32>,
}

pub(crate) fn dialog_ime_cursor_area(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
    preedit: Option<&DialogImePreeditHudState>,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
    let projection =
        dialog_preedit_projection(input, world, preedit?, surface_size, text_measurer)?;
    Some((projection.ime_position, projection.ime_size))
}

pub(super) fn dialog_preedit_projection(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
    preedit: &DialogImePreeditHudState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> Option<DialogPreeditProjection> {
    if preedit.screen_revision != input.dialog_screen_revision()
        || input.dialog_link_confirmation().is_some()
        || input.dialog_warning_is_open()
        || preedit.text.is_empty()
    {
        return None;
    }
    let dialog = input.current_dialog()?;
    let controls = dialog_controls(dialog.data.as_ref()?, world.server_links())?;
    let text_ctx = DialogTextContext::from_measurer(None, text_measurer);
    let content = dialog_content_layout_with(&controls, text_ctx)?;
    let scroll_layout = dialog_body_scroll_layout_with_content(
        &controls,
        &content,
        surface_size,
        input.dialog_body_scroll_amount(),
    )?;
    let input_layouts = dialog_input_layouts_with_content(
        &controls,
        &content,
        surface_size,
        scroll_layout.scroll_amount,
    )?;
    dialog_preedit_projection_with_inputs(
        input,
        world,
        &controls,
        &input_layouts,
        preedit,
        surface_size,
        text_measurer,
    )
}

pub(super) fn dialog_preedit_projection_with_inputs(
    input: &crate::input::ClientInputState,
    world: &WorldStore,
    controls: &DialogControls<'_>,
    input_layouts: &[DialogInputLayout],
    preedit: &DialogImePreeditHudState,
    surface_size: PhysicalSize<u32>,
    text_measurer: &dyn DialogTextMeasurer,
) -> Option<DialogPreeditProjection> {
    if preedit.screen_revision != input.dialog_screen_revision()
        || input.dialog_link_confirmation().is_some()
        || input.dialog_warning_is_open()
        || preedit.text.is_empty()
    {
        return None;
    }
    let snapshot = focused_dialog_text_snapshot(input, world)?;
    let input_layout = input_layouts
        .iter()
        .copied()
        .find(|layout| layout.index == snapshot.index)?;
    let definition = controls.common.input_data.get(snapshot.index)?;
    let (input_left, input_top) = match &definition.control {
        DialogInputControlState::Text {
            label_visible,
            multiline: None,
            ..
        } => {
            let layout = dialog_single_line_text_box_layout(input_layout, *label_visible);
            let inner_width = i32::try_from(snapshot.inner_width()).unwrap_or(i32::MAX);
            let scroll_to = if snapshot.selection != snapshot.cursor {
                snapshot.selection
            } else {
                snapshot.cursor
            };
            let units = snapshot.value.units();
            let display_start = text_measurer
                .glyphs()
                .plain_utf16_scroll_to(units, 0, scroll_to, inner_width)
                .unwrap_or(0);
            let display_end = display_start
                + text_measurer
                    .glyphs()
                    .plain_utf16_head_len(&units[display_start..], inner_width)
                    .unwrap_or(0);
            let prefix = dialog_char_slice(
                &snapshot.value,
                display_start,
                snapshot.cursor.min(display_end),
            );
            let insert = snapshot.cursor < units.len() || units.len() >= snapshot.max_length;
            let cursor_x = if snapshot.cursor < display_start {
                0
            } else if snapshot.cursor > display_end {
                inner_width
            } else {
                text_measurer.width(&prefix)
                    + match (display_end > display_start, insert) {
                        (true, false) => 1,
                        (false, true) => -1,
                        _ => 0,
                    }
            };
            (
                layout
                    .x
                    .saturating_add(TEXT_INPUT_INSET)
                    .saturating_add(cursor_x),
                layout.y.saturating_add(TEXT_INPUT_Y_OFFSET),
            )
        }
        DialogInputControlState::Text {
            label_visible,
            multiline: Some(_),
            ..
        } => {
            let layout = dialog_multiline_text_box_layout(input_layout, *label_visible);
            let lines =
                dialog_multiline_lines(&snapshot.value, snapshot.inner_width(), text_measurer);
            let max_scroll = dialog_multiline_max_scroll(lines.len(), layout.height);
            let scroll_amount = snapshot.scroll_amount.clamp(0, max_scroll);
            let cursor_line = lines
                .iter()
                .position(|line| snapshot.cursor <= line.end)
                .unwrap_or_else(|| lines.len().saturating_sub(1));
            let line = lines[cursor_line];
            let prefix =
                dialog_char_slice(&snapshot.value, line.begin, snapshot.cursor.min(line.end));
            (
                layout
                    .x
                    .saturating_add(TEXT_INPUT_INSET)
                    .saturating_add(text_measurer.width(&prefix)),
                layout
                    .y
                    .saturating_add(TEXT_INPUT_INSET)
                    .saturating_sub(scroll_amount)
                    .saturating_add(
                        i32::try_from(cursor_line)
                            .unwrap_or(i32::MAX)
                            .saturating_mul(TEXT_LINE_HEIGHT),
                    ),
            )
        }
        _ => return None,
    };

    let screen_width = surface_size.width.max(1);
    let screen_height = surface_size.height.max(1);
    let (origin_x, origin_y) = centered_screen_origin(surface_size, screen_width, screen_height);
    let input_left = i32::try_from(origin_x + i64::from(input_left)).unwrap_or_else(|_| {
        if origin_x + i64::from(input_left) < 0 {
            i32::MIN
        } else {
            i32::MAX
        }
    });
    let input_top = i32::try_from(origin_y + i64::from(input_top)).unwrap_or_else(|_| {
        if origin_y + i64::from(input_top) < 0 {
            i32::MIN
        } else {
            i32::MAX
        }
    });
    let preedit_width = text_measurer.width(&preedit.text);
    let surface_width = i32::try_from(surface_size.width).unwrap_or(i32::MAX);
    let surface_height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    let mut preedit_left = input_left;
    let mut preedit_right = preedit_left.saturating_add(preedit_width);
    if preedit_right > surface_width {
        preedit_left = surface_width.saturating_sub(preedit_width);
        preedit_right = preedit_left.saturating_add(preedit_width);
    }
    let input_bottom = input_top.saturating_add(PREEDIT_INPUT_HEIGHT);
    let mut preedit_bottom = input_bottom
        .saturating_add(PREEDIT_INPUT_SEPARATION)
        .saturating_add(PREEDIT_TEXT_HEIGHT);
    if preedit_bottom > surface_height {
        preedit_bottom = input_top
            .saturating_sub(PREEDIT_INPUT_SEPARATION)
            .saturating_sub(PREEDIT_TEXT_HEIGHT);
    }
    let preedit_top = preedit_bottom.saturating_sub(PREEDIT_TEXT_HEIGHT);
    let hot_left = preedit_left
        .min(input_left)
        .saturating_sub(PREEDIT_HOT_AREA_MARGIN);
    let hot_top = preedit_top
        .min(input_top)
        .saturating_sub(PREEDIT_HOT_AREA_MARGIN);
    let hot_right = preedit_right.saturating_add(PREEDIT_HOT_AREA_MARGIN);
    let hot_bottom = preedit_bottom
        .max(input_bottom)
        .saturating_add(PREEDIT_HOT_AREA_MARGIN);

    Some(DialogPreeditProjection {
        overlay: HudInventoryPreeditOverlay {
            text: preedit.text.clone(),
            focused_start: preedit.focused_start,
            focused_end: preedit.focused_end,
            caret: preedit.caret,
            caret_visible: preedit.caret_visible,
            text_x: preedit_left,
            text_y: preedit_top,
            background_x: preedit_left.saturating_sub(PREEDIT_BACKGROUND_OFFSET),
            background_y: preedit_top.saturating_sub(PREEDIT_BACKGROUND_OFFSET),
            background_width: u32::try_from(
                preedit_width.saturating_add(PREEDIT_BACKGROUND_OFFSET.saturating_mul(2)),
            )
            .unwrap_or(u32::MAX),
            background_height: u32::try_from(
                PREEDIT_TEXT_HEIGHT.saturating_add(PREEDIT_BACKGROUND_OFFSET.saturating_mul(2)),
            )
            .unwrap_or(u32::MAX),
        },
        ime_position: PhysicalPosition::new(hot_left, hot_top),
        ime_size: PhysicalSize::new(
            u32::try_from(hot_right.saturating_sub(hot_left).max(1)).unwrap_or(u32::MAX),
            u32::try_from(hot_bottom.saturating_sub(hot_top).max(1)).unwrap_or(u32::MAX),
        ),
    })
}

pub(in crate::runtime) fn hud_dialog_confirm_link_screen(
    confirmation: &DialogLinkConfirmationState,
    focused_button: Option<usize>,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> HudInventoryScreen {
    let glyphs = ApproximateDialogTextMeasurer.glyphs();
    let message_width = surface_size
        .width
        .max(1)
        .saturating_sub(CONFIRM_MESSAGE_HORIZONTAL_PADDING)
        .max(1);
    // The screen draws the link, so an unpaired unit reaches the font as
    // `U+FFFD` here while the value the platform receives stays raw.
    let message_lines =
        plain_dialog_text_lines_with(&confirmation.url.to_display_string(), message_width, glyphs)
            .into_iter()
            .take(CONFIRM_MESSAGE_MAX_ROWS)
            .collect::<Vec<_>>();
    let layout = DialogConfirmLinkLayout::new(message_lines.len(), surface_size);
    let cursor = cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted = layout.button_at(cursor, surface_size);
    let screen_center_x = i32::try_from(layout.screen_width).unwrap_or(i32::MAX) / 2;
    let mut text_labels = Vec::new();
    push_centered_plain_text_label(
        &mut text_labels,
        CONFIRM_LINK_TITLE,
        screen_center_x,
        layout.title_y,
        TEXT_COLOR,
        glyphs,
    );
    let mut message_y = layout.message_y;
    for line in message_lines {
        push_centered_runs_text_label(
            &mut text_labels,
            line.runs,
            screen_center_x,
            message_y,
            None,
            glyphs,
        );
        message_y = message_y.saturating_add(TEXT_LINE_HEIGHT);
    }
    push_centered_plain_text_label(
        &mut text_labels,
        CONFIRM_LINK_WARNING,
        screen_center_x,
        layout.warning_y,
        CONFIRM_LINK_WARNING_COLOR,
        glyphs,
    );
    for (button, label) in
        layout
            .buttons
            .into_iter()
            .zip([CONFIRM_LINK_YES, CONFIRM_LINK_COPY, CONFIRM_LINK_NO])
    {
        push_centered_plain_text_label(
            &mut text_labels,
            label,
            button
                .x
                .saturating_add(i32::try_from(button.width).unwrap_or(i32::MAX) / 2),
            button.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
            TEXT_COLOR,
            glyphs,
        );
    }
    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    background_layers.extend(
        layout
            .buttons
            .into_iter()
            .enumerate()
            .map(|(index, button)| HudInventoryBackgroundLayer {
                texture: if highlighted == Some(index) || focused_button == Some(index) {
                    HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
                } else {
                    HudInventoryBackgroundTexture::Sprite("widget/button")
                },
                x: button.x,
                y: button.y,
                width: button.width,
                height: BUTTON_HEIGHT,
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
            }),
    );
    HudInventoryScreen {
        width: layout.screen_width,
        height: layout.screen_height,
        background_layers,
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels,
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}

pub(in crate::runtime) fn hud_dialog_warning_screen(
    focused_button: Option<bool>,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> HudInventoryScreen {
    let glyphs = ApproximateDialogTextMeasurer.glyphs();
    let message_lines = dialog_warning_message_lines_with(surface_size, glyphs);
    let layout = DialogWarningScreenLayout::new(message_lines.len(), surface_size);
    let cursor = cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted = layout.button_at(cursor, surface_size);
    let screen_center_x = i32::try_from(layout.screen_width).unwrap_or(i32::MAX) / 2;
    let mut text_labels = Vec::new();
    push_centered_plain_text_label(
        &mut text_labels,
        DIALOG_WARNING_TITLE,
        screen_center_x,
        layout.title_y,
        TEXT_COLOR,
        glyphs,
    );
    let mut message_y = layout.message_y;
    for line in message_lines {
        push_centered_runs_text_label(
            &mut text_labels,
            line.runs,
            screen_center_x,
            message_y,
            None,
            glyphs,
        );
        message_y = message_y.saturating_add(TEXT_LINE_HEIGHT);
    }
    for (button, label) in [
        (layout.yes, DIALOG_WARNING_DISCONNECT),
        (layout.no, DIALOG_WARNING_BACK),
    ] {
        push_centered_plain_text_label(
            &mut text_labels,
            label,
            button
                .x
                .saturating_add(i32::try_from(button.width).unwrap_or(i32::MAX) / 2),
            button.y.saturating_add(BUTTON_TEXT_Y_OFFSET),
            TEXT_COLOR,
            glyphs,
        );
    }
    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    background_layers.extend([(layout.yes, true), (layout.no, false)].into_iter().map(
        |(button, result)| HudInventoryBackgroundLayer {
            texture: if highlighted == Some(result) || focused_button == Some(result) {
                HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
            } else {
                HudInventoryBackgroundTexture::Sprite("widget/button")
            },
            x: button.x,
            y: button.y,
            width: button.width,
            height: BUTTON_HEIGHT,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        },
    ));
    HudInventoryScreen {
        width: layout.screen_width,
        height: layout.screen_height,
        background_layers,
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels,
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}

pub(in crate::runtime) fn hud_waiting_response_screen(
    waiting_ticks: u32,
    cursor_position: Option<(i32, i32)>,
    surface_size: PhysicalSize<u32>,
) -> HudInventoryScreen {
    let layout = WaitingLayout::new(surface_size);
    let screen_width_i32 = i32::try_from(layout.screen_width).unwrap_or(i32::MAX);
    let visible = waiting_ticks >= WAITING_BUTTON_VISIBLE_TICKS;
    let active = waiting_ticks >= WAITING_BUTTON_ACTIVE_TICKS;
    let cursor_position =
        cursor_position.map(|(x, y)| PhysicalPosition::new(f64::from(x), f64::from(y)));
    let highlighted = active && layout.button_contains(cursor_position, surface_size);
    let glyphs = ApproximateDialogTextMeasurer.glyphs();
    let mut text_labels = Vec::new();
    push_centered_plain_text_label(
        &mut text_labels,
        WAITING_TITLE,
        screen_width_i32 / 2,
        (HEADER_HEIGHT - TEXT_LINE_HEIGHT) / 2,
        TEXT_COLOR,
        glyphs,
    );
    let mut background_layers = vec![inworld_menu_background_layer(
        surface_size,
        layout.screen_width,
        layout.screen_height,
    )];
    if visible {
        let seconds_visible = waiting_ticks.saturating_sub(1) / 20;
        let button_text = if active {
            "Back".to_string()
        } else {
            format!("Back ({}s)", 5u32.saturating_sub(seconds_visible))
        };
        background_layers.push(HudInventoryBackgroundLayer {
            texture: if highlighted {
                HudInventoryBackgroundTexture::Sprite("widget/button_highlighted")
            } else if active {
                HudInventoryBackgroundTexture::Sprite("widget/button")
            } else {
                HudInventoryBackgroundTexture::Sprite("widget/button_disabled")
            },
            x: layout.button_x,
            y: layout.button_y,
            width: WAITING_BUTTON_WIDTH,
            height: BUTTON_HEIGHT,
            uv: HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
        });
        push_centered_plain_text_label(
            &mut text_labels,
            &button_text,
            layout.button_x + WAITING_BUTTON_WIDTH as i32 / 2,
            layout.button_y + BUTTON_TEXT_Y_OFFSET,
            if active {
                TEXT_COLOR
            } else {
                INACTIVE_TEXT_COLOR
            },
            glyphs,
        );
    }

    HudInventoryScreen {
        width: layout.screen_width,
        height: layout.screen_height,
        background_layers,
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: Vec::new(),
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels,
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    }
}
