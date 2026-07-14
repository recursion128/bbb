use bbb_protocol::{
    canonical_component_json_utf16_len_with_owner, canonical_f32_json_utf16_len,
    canonical_json_string_utf16_len, canonical_nbt_json_utf16_len, CanonicalComponentOwnerValue,
    NbtSummaryValue,
};

use super::{
    action::decode_action_dialog_holder, body, default_notice_action, CommonDialogDataState,
    DialogActionState, DialogBodyState, DialogButtonState, DialogDataState, DialogDecodeContext,
    DialogHolderSetState, DialogInputControlState, DialogInputState, DialogPlainMessageState,
    DialogSingleOptionEntryState, DialogState, DialogTextState,
};

pub(in crate::client::dialog) fn canonical_component_json_utf16_len_with_context(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    canonical_component_json_utf16_len_with_owner(value, |kind, value| match kind {
        CanonicalComponentOwnerValue::PlayerProfile => {
            body::canonical_component_player_profile_json_utf16_len(value)
        }
        CanonicalComponentOwnerValue::ItemComponentPatch => {
            let patch = body::decode_data_component_patch(value, context)?;
            body::canonical_data_component_patch_json_utf16_len(&patch)
        }
        CanonicalComponentOwnerValue::Dialog => canonical_dialog_holder_json_utf16_len(
            value,
            DialogDecodeContext::new(context.registries, context.depth + 1),
        ),
    })
}

fn canonical_dialog_holder_json_utf16_len(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let state = decode_action_dialog_holder(value, context.registries, context.depth)?;
    dialog_holder_len(&state, context)
}

fn dialog_holder_len(
    state: &DialogState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    match state.holder_kind.as_str() {
        "reference" => Ok(state.resource_id.as_deref().map(string_len)),
        "direct" | "context_free" => {
            let Some(data) = state.data.as_ref() else {
                return Ok(None);
            };
            dialog_data_len(data, context)
        }
        _ => Ok(None),
    }
}

fn dialog_data_len(
    data: &DialogDataState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let common = match data {
        DialogDataState::Notice { common, .. }
        | DialogDataState::Confirmation { common, .. }
        | DialogDataState::MultiAction { common, .. }
        | DialogDataState::ServerLinks { common, .. }
        | DialogDataState::DialogList { common, .. } => common,
    };
    let Some(mut fields) = common_fields(common, context)? else {
        return Ok(None);
    };
    match data {
        DialogDataState::Notice { action, .. } => {
            fields.insert(0, ("type", string_len("minecraft:notice")));
            if action != &default_notice_action() {
                let Some(action_len) = button_len(action, context)? else {
                    return Ok(None);
                };
                fields.push(("action", action_len));
            }
        }
        DialogDataState::Confirmation { yes, no, .. } => {
            fields.insert(0, ("type", string_len("minecraft:confirmation")));
            let Some(yes_len) = button_len(yes, context)? else {
                return Ok(None);
            };
            fields.push(("yes", yes_len));
            let Some(no_len) = button_len(no, context)? else {
                return Ok(None);
            };
            fields.push(("no", no_len));
        }
        DialogDataState::MultiAction {
            actions,
            exit_action,
            columns,
            ..
        } => {
            fields.insert(0, ("type", string_len("minecraft:multi_action")));
            let Some(actions_len) = button_list_len(actions, context)? else {
                return Ok(None);
            };
            fields.push(("actions", actions_len));
            if let Some(exit_action) = exit_action {
                let Some(exit_action_len) = button_len(exit_action, context)? else {
                    return Ok(None);
                };
                fields.push(("exit_action", exit_action_len));
            }
            if *columns != 2 {
                fields.push(("columns", columns.to_string().len()));
            }
        }
        DialogDataState::ServerLinks {
            exit_action,
            columns,
            button_width,
            ..
        } => {
            fields.insert(0, ("type", string_len("minecraft:server_links")));
            if let Some(exit_action) = exit_action {
                let Some(exit_action_len) = button_len(exit_action, context)? else {
                    return Ok(None);
                };
                fields.push(("exit_action", exit_action_len));
            }
            if *columns != 2 {
                fields.push(("columns", columns.to_string().len()));
            }
            if *button_width != 150 {
                fields.push(("button_width", button_width.to_string().len()));
            }
        }
        DialogDataState::DialogList {
            dialogs,
            exit_action,
            columns,
            button_width,
            ..
        } => {
            fields.insert(0, ("type", string_len("minecraft:dialog_list")));
            let Some(dialogs_len) = dialog_holder_set_len(dialogs, context)? else {
                return Ok(None);
            };
            fields.push(("dialogs", dialogs_len));
            if let Some(exit_action) = exit_action {
                let Some(exit_action_len) = button_len(exit_action, context)? else {
                    return Ok(None);
                };
                fields.push(("exit_action", exit_action_len));
            }
            if *columns != 2 {
                fields.push(("columns", columns.to_string().len()));
            }
            if *button_width != 150 {
                fields.push(("button_width", button_width.to_string().len()));
            }
        }
    }
    Ok(Some(object_len(fields.into_iter())))
}

fn button_list_len(
    buttons: &[DialogButtonState],
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let mut lengths = Vec::with_capacity(buttons.len());
    for button in buttons {
        let Some(length) = button_len(button, context)? else {
            return Ok(None);
        };
        lengths.push(length);
    }
    Ok(Some(array_len(lengths.into_iter())))
}

fn dialog_holder_set_len(
    holder_set: &DialogHolderSetState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    match holder_set.holder_set_kind.as_str() {
        "named" => Ok(holder_set
            .tag
            .as_ref()
            .map(|tag| string_len(&format!("#{tag}")))),
        "direct" => {
            let nested_context =
                DialogDecodeContext::new(context.registries, context.depth.saturating_add(1));
            let mut lengths = Vec::with_capacity(holder_set.entries.len());
            for entry in &holder_set.entries {
                let length = match entry.holder_kind.as_str() {
                    "reference" => {
                        let Some(resource_id) = entry.resource_id.as_deref() else {
                            return Ok(None);
                        };
                        string_len(resource_id)
                    }
                    "direct" => {
                        let Some(dialog) = entry.dialog.as_ref() else {
                            return Ok(None);
                        };
                        let state = decode_action_dialog_holder(
                            dialog,
                            nested_context.registries,
                            nested_context.depth,
                        )?;
                        let Some(length) = dialog_holder_len(&state, nested_context)? else {
                            return Ok(None);
                        };
                        length
                    }
                    _ => return Ok(None),
                };
                lengths.push(length);
            }
            Ok(Some(match lengths.as_slice() {
                [single] => *single,
                _ => array_len(lengths.into_iter()),
            }))
        }
        _ => Ok(None),
    }
}

fn common_fields(
    common: &CommonDialogDataState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<Vec<(&'static str, usize)>>, String> {
    let Some(title_len) = component_len(&common.title, context)? else {
        return Ok(None);
    };
    let mut fields = vec![("title", title_len)];
    if let Some(external_title) = &common.external_title {
        let Some(external_title_len) = component_len(external_title, context)? else {
            return Ok(None);
        };
        fields.push(("external_title", external_title_len));
    }
    if !common.can_close_with_escape {
        fields.push(("can_close_with_escape", bool_len(false)));
    }
    if !common.pause {
        fields.push(("pause", bool_len(false)));
    }
    if common.after_action != "close" {
        fields.push(("after_action", string_len(&common.after_action)));
    }
    if !common.body_data.is_empty() {
        let Some(body_len) = compact_body_list_len(&common.body_data, context)? else {
            return Ok(None);
        };
        fields.push(("body", body_len));
    }
    if !common.input_data.is_empty() {
        let Some(inputs_len) = inputs_len(&common.input_data, context)? else {
            return Ok(None);
        };
        fields.push(("inputs", inputs_len));
    }
    Ok(Some(fields))
}

fn inputs_len(
    inputs: &[DialogInputState],
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let mut lengths = Vec::with_capacity(inputs.len());
    for input in inputs {
        let Some(length) = input_len(input, context)? else {
            return Ok(None);
        };
        lengths.push(length);
    }
    Ok(Some(array_len(lengths.into_iter())))
}

fn input_len(
    input: &DialogInputState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let mut fields = vec![("key", string_len(&input.key))];
    match &input.control {
        DialogInputControlState::Boolean {
            label,
            initial,
            on_true,
            on_false,
        } => {
            fields.push(("type", string_len("minecraft:boolean")));
            let Some(label_len) = component_len(label, context)? else {
                return Ok(None);
            };
            fields.push(("label", label_len));
            if *initial {
                fields.push(("initial", bool_len(true)));
            }
            if on_true != "true" {
                fields.push(("on_true", string_len(on_true)));
            }
            if on_false != "false" {
                fields.push(("on_false", string_len(on_false)));
            }
        }
        DialogInputControlState::NumberRange {
            width,
            label,
            label_format,
            range,
        } => {
            fields.push(("type", string_len("minecraft:number_range")));
            if *width != 200 {
                fields.push(("width", width.to_string().len()));
            }
            let Some(label_len) = component_len(label, context)? else {
                return Ok(None);
            };
            fields.push(("label", label_len));
            if label_format != "options.generic_value" {
                fields.push(("label_format", string_len(label_format)));
            }
            fields.push(("start", float_len(range.start_bits)?));
            fields.push(("end", float_len(range.end_bits)?));
            if let Some(bits) = range.initial_bits {
                fields.push(("initial", float_len(bits)?));
            }
            if let Some(bits) = range.step_bits {
                fields.push(("step", float_len(bits)?));
            }
        }
        DialogInputControlState::SingleOption {
            width,
            options,
            label,
            label_visible,
        } => {
            fields.push(("type", string_len("minecraft:single_option")));
            if *width != 200 {
                fields.push(("width", width.to_string().len()));
            }
            let Some(options_len) = single_options_len(options, context)? else {
                return Ok(None);
            };
            fields.push(("options", options_len));
            let Some(label_len) = component_len(label, context)? else {
                return Ok(None);
            };
            fields.push(("label", label_len));
            if !*label_visible {
                fields.push(("label_visible", bool_len(false)));
            }
        }
        DialogInputControlState::Text {
            width,
            label,
            label_visible,
            initial,
            max_length,
            multiline,
        } => {
            fields.push(("type", string_len("minecraft:text")));
            if *width != 200 {
                fields.push(("width", width.to_string().len()));
            }
            let Some(label_len) = component_len(label, context)? else {
                return Ok(None);
            };
            fields.push(("label", label_len));
            if !*label_visible {
                fields.push(("label_visible", bool_len(false)));
            }
            if !initial.is_empty() {
                fields.push(("initial", string_len(initial)));
            }
            if *max_length != 32 {
                fields.push(("max_length", max_length.to_string().len()));
            }
            if let Some(multiline) = multiline {
                let mut multiline_fields = Vec::new();
                if let Some(max_lines) = multiline.max_lines {
                    multiline_fields.push(("max_lines", max_lines.to_string().len()));
                }
                if let Some(height) = multiline.height {
                    multiline_fields.push(("height", height.to_string().len()));
                }
                fields.push(("multiline", object_len(multiline_fields.into_iter())));
            }
        }
    }
    Ok(Some(object_len(fields.into_iter())))
}

fn single_options_len(
    options: &[DialogSingleOptionEntryState],
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let mut lengths = Vec::with_capacity(options.len());
    for option in options {
        let mut fields = vec![("id", string_len(&option.id))];
        if let Some(display) = &option.display {
            let Some(display_len) = component_len(display, context)? else {
                return Ok(None);
            };
            fields.push(("display", display_len));
        }
        if option.initial {
            fields.push(("initial", bool_len(true)));
        }
        lengths.push(object_len(fields.into_iter()));
    }
    Ok(Some(array_len(lengths.into_iter())))
}

fn compact_body_list_len(
    bodies: &[DialogBodyState],
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let mut lengths = Vec::with_capacity(bodies.len());
    for body in bodies {
        let Some(length) = body_len(body, context)? else {
            return Ok(None);
        };
        lengths.push(length);
    }
    Ok(Some(match lengths.as_slice() {
        [single] => *single,
        _ => array_len(lengths.into_iter()),
    }))
}

fn body_len(
    body: &DialogBodyState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let mut fields = Vec::new();
    match body {
        DialogBodyState::PlainMessage { message } => {
            fields.push(("type", string_len("minecraft:plain_message")));
            let Some(contents_len) = component_len(&message.contents, context)? else {
                return Ok(None);
            };
            fields.push(("contents", contents_len));
            if message.width != 200 {
                fields.push(("width", message.width.to_string().len()));
            }
        }
        DialogBodyState::Item {
            item,
            description,
            show_decorations,
            show_tooltip,
            width,
            height,
        } => {
            fields.push(("type", string_len("minecraft:item")));
            let Some(item_len) = body::canonical_item_stack_template_json_utf16_len(item)? else {
                return Ok(None);
            };
            fields.push(("item", item_len));
            if let Some(description) = description {
                let Some(description_len) = plain_message_len(description, context)? else {
                    return Ok(None);
                };
                fields.push(("description", description_len));
            }
            if !*show_decorations {
                fields.push(("show_decorations", bool_len(false)));
            }
            if !*show_tooltip {
                fields.push(("show_tooltip", bool_len(false)));
            }
            if *width != 16 {
                fields.push(("width", width.to_string().len()));
            }
            if *height != 16 {
                fields.push(("height", height.to_string().len()));
            }
        }
    }
    Ok(Some(object_len(fields.into_iter())))
}

fn plain_message_len(
    message: &DialogPlainMessageState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let Some(contents_len) = component_len(&message.contents, context)? else {
        return Ok(None);
    };
    let mut fields = vec![("contents", contents_len)];
    if message.width != 200 {
        fields.push(("width", message.width.to_string().len()));
    }
    Ok(Some(object_len(fields.into_iter())))
}

fn button_len(
    button: &DialogButtonState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let Some(label_len) = component_len(&button.label, context)? else {
        return Ok(None);
    };
    let mut fields = vec![("label", label_len)];
    if let Some(tooltip) = &button.tooltip {
        let Some(tooltip_len) = component_len(tooltip, context)? else {
            return Ok(None);
        };
        fields.push(("tooltip", tooltip_len));
    }
    if button.width != 150 {
        fields.push(("width", button.width.to_string().len()));
    }
    match (&button.action, &button.action_data) {
        (None, None) => {}
        (Some(_), Some(action)) => {
            let Some(action_len) = action_len(action, context)? else {
                return Ok(None);
            };
            fields.push(("action", action_len));
        }
        _ => return Ok(None),
    }
    Ok(Some(object_len(fields.into_iter())))
}

fn action_len(
    action: &DialogActionState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    let mut fields = Vec::new();
    match action {
        DialogActionState::OpenUrl { url } => {
            fields.push(("type", string_len("minecraft:open_url")));
            fields.push(("url", string_len(url)));
        }
        DialogActionState::RunCommand { command } => {
            fields.push(("type", string_len("minecraft:run_command")));
            fields.push(("command", string_len(command)));
        }
        DialogActionState::SuggestCommand { command } => {
            fields.push(("type", string_len("minecraft:suggest_command")));
            fields.push(("command", string_len(command)));
        }
        DialogActionState::ShowDialog { dialog } => {
            fields.push(("type", string_len("minecraft:show_dialog")));
            let Some(dialog_len) = dialog_holder_len(dialog, context)? else {
                return Ok(None);
            };
            fields.push(("dialog", dialog_len));
        }
        DialogActionState::ChangePage { page } => {
            fields.push(("type", string_len("minecraft:change_page")));
            fields.push(("page", page.to_string().len()));
        }
        DialogActionState::CopyToClipboard { value } => {
            fields.push(("type", string_len("minecraft:copy_to_clipboard")));
            fields.push(("value", string_len(value)));
        }
        DialogActionState::Custom { id, payload } => {
            fields.push(("type", string_len("minecraft:custom")));
            fields.push(("id", string_len(id)));
            if let Some(payload) = payload {
                fields.push(("payload", canonical_nbt_json_utf16_len(payload)?));
            }
        }
        DialogActionState::DynamicRunCommand { template } => {
            fields.push(("type", string_len("minecraft:dynamic/run_command")));
            fields.push(("template", string_len(&template.raw)));
        }
        DialogActionState::DynamicCustom { id, additions } => {
            fields.push(("type", string_len("minecraft:dynamic/custom")));
            fields.push(("id", string_len(id)));
            if let Some(additions) = additions {
                fields.push(("additions", canonical_nbt_json_utf16_len(additions)?));
            }
        }
    }
    Ok(Some(object_len(fields.into_iter())))
}

fn component_len(
    text: &DialogTextState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<usize>, String> {
    canonical_component_json_utf16_len_with_context(&text.component, context)
}

fn string_len(value: &str) -> usize {
    canonical_json_string_utf16_len(value)
}

fn bool_len(value: bool) -> usize {
    if value {
        4
    } else {
        5
    }
}

fn float_len(bits: u32) -> Result<usize, String> {
    canonical_f32_json_utf16_len(bits)
        .ok_or_else(|| "canonical JsonWriter rejects non-finite Float/Double values".to_string())
}

fn object_len<'a>(fields: impl Iterator<Item = (&'a str, usize)>) -> usize {
    collection_len(
        fields
            .map(|(name, value_len)| string_len(name).saturating_add(1).saturating_add(value_len)),
    )
}

fn array_len(values: impl Iterator<Item = usize>) -> usize {
    collection_len(values)
}

fn collection_len(values: impl Iterator<Item = usize>) -> usize {
    values.enumerate().fold(2usize, |length, (index, value)| {
        length
            .saturating_add(usize::from(index != 0))
            .saturating_add(value)
    })
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::{RegistryPacketEntry, WorldStore};

    use super::super::DialogListEntryState;
    use super::*;

    #[test]
    fn canonicalizes_direct_notice_holder_recursively() {
        let value = component_with_show_dialog(notice("Hello"));
        let expected = r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Hello"}}}"#;
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &value,
                DialogDecodeContext::new(None, 0),
            )
            .unwrap(),
            Some(expected.len())
        );
    }

    #[test]
    fn canonicalizes_named_notice_holder_as_resource_id() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            super::super::DIALOG_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:notice",
                notice_nbt("Referenced"),
            )],
        );
        let value =
            component_with_show_dialog(NbtSummaryValue::String("example:notice".to_string()));
        let expected =
            r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":"example:notice"}}"#;
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &value,
                DialogDecodeContext::new(Some(store.registries()), 0),
            )
            .unwrap(),
            Some(expected.len())
        );
    }

    #[test]
    fn leaves_unknown_or_unresolved_dialog_holders_unprojected() {
        let unknown = DialogState {
            holder_kind: "future".to_string(),
            registry_id: None,
            resource_id: None,
            dialog: None,
            resolution_error: None,
            data: None,
            semantic_error: None,
        };
        assert_eq!(
            dialog_holder_len(&unknown, DialogDecodeContext::new(None, 0)).unwrap(),
            None
        );

        let missing =
            component_with_show_dialog(NbtSummaryValue::String("example:missing".to_string()));
        assert!(canonical_component_json_utf16_len_with_context(
            &missing,
            DialogDecodeContext::new(Some(&crate::RegistrySet::default()), 0),
        )
        .is_err());
    }

    #[test]
    fn canonicalizes_common_fields_and_non_default_button() {
        let value = component_with_show_dialog(compound(vec![
            string_entry("type", "notice"),
            string_entry("title", "Title"),
            string_entry("external_title", "External"),
            value_entry("can_close_with_escape", NbtSummaryValue::Byte(0)),
            value_entry("pause", NbtSummaryValue::Byte(0)),
            string_entry("after_action", "none"),
            value_entry(
                "action",
                compound(vec![
                    string_entry("label", "Go"),
                    string_entry("tooltip", "Tip"),
                    value_entry("width", NbtSummaryValue::Int(200)),
                    value_entry(
                        "action",
                        compound(vec![
                            string_entry("type", "run_command"),
                            string_entry("command", "say hi"),
                        ]),
                    ),
                ]),
            ),
        ]));
        let expected = r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Title","external_title":"External","can_close_with_escape":false,"pause":false,"after_action":"none","action":{"label":"Go","tooltip":"Tip","width":200,"action":{"type":"minecraft:run_command","command":"say hi"}}}}}"#;
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &value,
                DialogDecodeContext::new(None, 0),
            )
            .unwrap(),
            Some(expected.len())
        );
    }

    #[test]
    fn canonicalizes_all_dialog_action_variants() {
        let nested = decode_action_dialog_holder(&notice("Nested"), None, 1).unwrap();
        let cases = vec![
            (
                DialogActionState::OpenUrl {
                    url: "https://example.invalid/".to_string(),
                },
                r#"{"type":"minecraft:open_url","url":"https://example.invalid/"}"#,
            ),
            (
                DialogActionState::RunCommand {
                    command: "say hi".to_string(),
                },
                r#"{"type":"minecraft:run_command","command":"say hi"}"#,
            ),
            (
                DialogActionState::SuggestCommand {
                    command: "/help".to_string(),
                },
                r#"{"type":"minecraft:suggest_command","command":"/help"}"#,
            ),
            (
                DialogActionState::ShowDialog {
                    dialog: Box::new(nested),
                },
                r#"{"type":"minecraft:show_dialog","dialog":{"type":"minecraft:notice","title":"Nested"}}"#,
            ),
            (
                DialogActionState::ChangePage { page: 3 },
                r#"{"type":"minecraft:change_page","page":3}"#,
            ),
            (
                DialogActionState::CopyToClipboard {
                    value: "copy".to_string(),
                },
                r#"{"type":"minecraft:copy_to_clipboard","value":"copy"}"#,
            ),
            (
                DialogActionState::Custom {
                    id: "example:custom".to_string(),
                    payload: Some(NbtSummaryValue::String("payload".to_string())),
                },
                r#"{"type":"minecraft:custom","id":"example:custom","payload":"payload"}"#,
            ),
            (
                DialogActionState::DynamicRunCommand {
                    template: super::super::DialogCommandTemplateState {
                        raw: "say $(value)".to_string(),
                        segments: vec!["say ".to_string()],
                        variables: vec!["value".to_string()],
                    },
                },
                r#"{"type":"minecraft:dynamic/run_command","template":"say $(value)"}"#,
            ),
            (
                DialogActionState::DynamicCustom {
                    id: "example:dynamic".to_string(),
                    additions: Some(compound(vec![value_entry(
                        "fixed",
                        NbtSummaryValue::Int(7),
                    )])),
                },
                r#"{"type":"minecraft:dynamic/custom","id":"example:dynamic","additions":{"fixed":7}}"#,
            ),
        ];
        for (action, expected) in cases {
            assert_eq!(
                action_len(&action, DialogDecodeContext::new(None, 0)).unwrap(),
                Some(expected.len()),
                "{action:?}"
            );
        }
    }

    #[test]
    fn canonicalizes_compact_plain_and_item_bodies() {
        let singleton = component_with_show_dialog(compound(vec![
            string_entry("type", "notice"),
            string_entry("title", "Body"),
            value_entry(
                "body",
                compound(vec![
                    string_entry("type", "plain_message"),
                    string_entry("contents", "Message"),
                ]),
            ),
        ]));
        let singleton_expected = r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Body","body":{"type":"minecraft:plain_message","contents":"Message"}}}}"#;
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &singleton,
                DialogDecodeContext::new(None, 0),
            )
            .unwrap(),
            Some(singleton_expected.len())
        );

        let multiple = component_with_show_dialog(compound(vec![
            string_entry("type", "notice"),
            string_entry("title", "Body"),
            value_entry(
                "body",
                NbtSummaryValue::List(vec![
                    compound(vec![
                        string_entry("type", "plain_message"),
                        string_entry("contents", "Message"),
                        value_entry("width", NbtSummaryValue::Int(240)),
                    ]),
                    compound(vec![
                        string_entry("type", "item"),
                        value_entry(
                            "item",
                            compound(vec![
                                string_entry("id", "stone"),
                                value_entry("count", NbtSummaryValue::Int(2)),
                                value_entry(
                                    "components",
                                    compound(vec![value_entry(
                                        "repair_cost",
                                        NbtSummaryValue::Int(4),
                                    )]),
                                ),
                            ]),
                        ),
                        string_entry("description", "Description"),
                        value_entry("show_decorations", NbtSummaryValue::Byte(0)),
                        value_entry("show_tooltip", NbtSummaryValue::Byte(0)),
                        value_entry("width", NbtSummaryValue::Int(32)),
                        value_entry("height", NbtSummaryValue::Int(24)),
                    ]),
                ]),
            ),
        ]));
        let multiple_expected = r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Body","body":[{"type":"minecraft:plain_message","contents":"Message","width":240},{"type":"minecraft:item","item":{"id":"minecraft:stone","count":2,"components":{"minecraft:repair_cost":4}},"description":{"contents":"Description"},"show_decorations":false,"show_tooltip":false,"width":32,"height":24}]}}}"#;
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &multiple,
                DialogDecodeContext::new(None, 0),
            )
            .unwrap(),
            Some(multiple_expected.len())
        );
    }

    #[test]
    fn canonicalizes_all_dialog_input_controls() {
        let value = component_with_show_dialog(compound(vec![
            string_entry("type", "notice"),
            string_entry("title", "Inputs"),
            value_entry(
                "inputs",
                NbtSummaryValue::List(vec![
                    compound(vec![
                        string_entry("key", "enabled"),
                        string_entry("type", "boolean"),
                        string_entry("label", "Enabled"),
                        value_entry("initial", NbtSummaryValue::Byte(1)),
                        string_entry("on_true", "yes"),
                        string_entry("on_false", "no"),
                    ]),
                    compound(vec![
                        string_entry("key", "amount"),
                        string_entry("type", "number_range"),
                        value_entry("width", NbtSummaryValue::Int(240)),
                        string_entry("label", "Amount"),
                        string_entry("label_format", "example.range"),
                        value_entry("start", NbtSummaryValue::Float((-0.0f32).to_bits())),
                        value_entry("end", NbtSummaryValue::Float(10.0f32.to_bits())),
                        value_entry("initial", NbtSummaryValue::Float(0.5f32.to_bits())),
                        value_entry("step", NbtSummaryValue::Float(0.25f32.to_bits())),
                    ]),
                    compound(vec![
                        string_entry("key", "mode"),
                        string_entry("type", "single_option"),
                        value_entry(
                            "options",
                            NbtSummaryValue::List(vec![
                                NbtSummaryValue::String("compact".to_string()),
                                compound(vec![
                                    string_entry("id", "full"),
                                    string_entry("display", "Full"),
                                    value_entry("initial", NbtSummaryValue::Byte(1)),
                                ]),
                            ]),
                        ),
                        string_entry("label", "Mode"),
                        value_entry("label_visible", NbtSummaryValue::Byte(0)),
                    ]),
                    compound(vec![
                        string_entry("key", "name"),
                        string_entry("type", "text"),
                        value_entry("width", NbtSummaryValue::Int(300)),
                        string_entry("label", "Name"),
                        value_entry("label_visible", NbtSummaryValue::Byte(0)),
                        string_entry("initial", "hello"),
                        value_entry("max_length", NbtSummaryValue::Int(64)),
                        value_entry(
                            "multiline",
                            compound(vec![
                                value_entry("max_lines", NbtSummaryValue::Int(3)),
                                value_entry("height", NbtSummaryValue::Int(100)),
                            ]),
                        ),
                    ]),
                ]),
            ),
        ]));
        let expected = concat!(
            r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:notice","title":"Inputs","inputs":["#,
            r#"{"key":"enabled","type":"minecraft:boolean","label":"Enabled","initial":true,"on_true":"yes","on_false":"no"},"#,
            r#"{"key":"amount","type":"minecraft:number_range","width":240,"label":"Amount","label_format":"example.range","start":-0.0,"end":10.0,"initial":0.5,"step":0.25},"#,
            r#"{"key":"mode","type":"minecraft:single_option","options":[{"id":"compact"},{"id":"full","display":"Full","initial":true}],"label":"Mode","label_visible":false},"#,
            r#"{"key":"name","type":"minecraft:text","width":300,"label":"Name","label_visible":false,"initial":"hello","max_length":64,"multiline":{"max_lines":3,"height":100}}]}}}"#,
        );
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &value,
                DialogDecodeContext::new(None, 0),
            )
            .unwrap(),
            Some(expected.len())
        );
    }

    #[test]
    fn rejects_non_finite_dialog_number_range_values() {
        let value = component_with_show_dialog(compound(vec![
            string_entry("type", "notice"),
            string_entry("title", "Inputs"),
            value_entry(
                "inputs",
                NbtSummaryValue::List(vec![compound(vec![
                    string_entry("key", "amount"),
                    string_entry("type", "number_range"),
                    string_entry("label", "Amount"),
                    value_entry("start", NbtSummaryValue::Float(f32::NAN.to_bits())),
                    value_entry("end", NbtSummaryValue::Float(1.0f32.to_bits())),
                ])]),
            ),
        ]));
        let error = canonical_component_json_utf16_len_with_context(
            &value,
            DialogDecodeContext::new(None, 0),
        )
        .unwrap_err();
        assert!(error.contains("rejects non-finite Float/Double"), "{error}");
    }

    #[test]
    fn canonicalizes_remaining_top_level_dialog_types() {
        let cases = vec![
            (
                compound(vec![
                    string_entry("type", "confirmation"),
                    string_entry("title", "Confirm"),
                    value_entry("yes", compound(vec![string_entry("label", "Yes")])),
                    value_entry("no", compound(vec![string_entry("label", "No")])),
                ]),
                r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:confirmation","title":"Confirm","yes":{"label":"Yes"},"no":{"label":"No"}}}}"#,
            ),
            (
                compound(vec![
                    string_entry("type", "multi_action"),
                    string_entry("title", "Actions"),
                    value_entry(
                        "actions",
                        NbtSummaryValue::List(vec![
                            compound(vec![string_entry("label", "One")]),
                            compound(vec![
                                string_entry("label", "Two"),
                                value_entry("width", NbtSummaryValue::Int(180)),
                            ]),
                        ]),
                    ),
                    value_entry("exit_action", compound(vec![string_entry("label", "Exit")])),
                    value_entry("columns", NbtSummaryValue::Int(3)),
                ]),
                r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:multi_action","title":"Actions","actions":[{"label":"One"},{"label":"Two","width":180}],"exit_action":{"label":"Exit"},"columns":3}}}"#,
            ),
            (
                compound(vec![
                    string_entry("type", "server_links"),
                    string_entry("title", "Links"),
                    value_entry("exit_action", compound(vec![string_entry("label", "Done")])),
                    value_entry("columns", NbtSummaryValue::Int(4)),
                    value_entry("button_width", NbtSummaryValue::Int(200)),
                ]),
                r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:server_links","title":"Links","exit_action":{"label":"Done"},"columns":4,"button_width":200}}}"#,
            ),
            (
                compound(vec![
                    string_entry("type", "dialog_list"),
                    string_entry("title", "Menu"),
                    value_entry(
                        "dialogs",
                        NbtSummaryValue::List(vec![
                            notice("First"),
                            compound(vec![
                                string_entry("type", "confirmation"),
                                string_entry("title", "Second"),
                                value_entry("yes", compound(vec![string_entry("label", "Yes")])),
                                value_entry("no", compound(vec![string_entry("label", "No")])),
                            ]),
                        ]),
                    ),
                    value_entry("exit_action", compound(vec![string_entry("label", "Back")])),
                    value_entry("columns", NbtSummaryValue::Int(3)),
                    value_entry("button_width", NbtSummaryValue::Int(220)),
                ]),
                r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:dialog_list","title":"Menu","dialogs":[{"type":"minecraft:notice","title":"First"},{"type":"minecraft:confirmation","title":"Second","yes":{"label":"Yes"},"no":{"label":"No"}}],"exit_action":{"label":"Back"},"columns":3,"button_width":220}}}"#,
            ),
        ];
        for (dialog, expected) in cases {
            assert_eq!(
                canonical_component_json_utf16_len_with_context(
                    &component_with_show_dialog(dialog),
                    DialogDecodeContext::new(None, 0),
                )
                .unwrap(),
                Some(expected.len()),
                "{expected}"
            );
        }
    }

    #[test]
    fn canonicalizes_compact_and_named_dialog_holder_sets() {
        let singleton = component_with_show_dialog(compound(vec![
            string_entry("type", "dialog_list"),
            string_entry("title", "Menu"),
            value_entry("dialogs", NbtSummaryValue::List(vec![notice("Only")])),
        ]));
        let expected = r#"{"text":"Open","click_event":{"action":"show_dialog","dialog":{"type":"minecraft:dialog_list","title":"Menu","dialogs":{"type":"minecraft:notice","title":"Only"}}}}"#;
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &singleton,
                DialogDecodeContext::new(None, 0),
            )
            .unwrap(),
            Some(expected.len())
        );

        let named = DialogHolderSetState {
            holder_set_kind: "named".to_string(),
            tag: Some("example:menu".to_string()),
            entries: Vec::new(),
        };
        assert_eq!(
            dialog_holder_set_len(&named, DialogDecodeContext::new(None, 0)).unwrap(),
            Some(string_len("#example:menu"))
        );

        let referenced = DialogHolderSetState {
            holder_set_kind: "direct".to_string(),
            tag: None,
            entries: vec![DialogListEntryState {
                holder_kind: "reference".to_string(),
                registry_id: Some(0),
                resource_id: Some("example:notice".to_string()),
                external_title: DialogTextState {
                    text: "Notice".to_string(),
                    runs: Vec::new(),
                    component: NbtSummaryValue::String("Notice".to_string()),
                },
                dialog: None,
            }],
        };
        assert_eq!(
            dialog_holder_set_len(&referenced, DialogDecodeContext::new(None, 0)).unwrap(),
            Some(string_len("example:notice"))
        );

        let empty = DialogHolderSetState {
            holder_set_kind: "direct".to_string(),
            tag: None,
            entries: Vec::new(),
        };
        assert_eq!(
            dialog_holder_set_len(&empty, DialogDecodeContext::new(None, 0)).unwrap(),
            Some(2)
        );
    }

    fn component_with_show_dialog(dialog: NbtSummaryValue) -> NbtSummaryValue {
        compound(vec![
            string_entry("text", "Open"),
            value_entry(
                "click_event",
                compound(vec![
                    string_entry("action", "show_dialog"),
                    value_entry("dialog", dialog),
                ]),
            ),
        ])
    }

    fn notice(title: &str) -> NbtSummaryValue {
        compound(vec![
            string_entry("type", "notice"),
            string_entry("title", title),
        ])
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
        value_entry(name, NbtSummaryValue::String(value.to_string()))
    }

    fn notice_nbt(title: &str) -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_named_string(&mut out, "type", "minecraft:notice");
        write_nbt_named_string(&mut out, "title", title);
        out.push(0);
        out
    }

    fn write_nbt_named_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
