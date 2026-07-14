use bbb_protocol::{
    canonical_component_json_utf16_len_with_owner, canonical_json_string_utf16_len,
    canonical_nbt_json_utf16_len, CanonicalComponentOwnerValue, NbtSummaryValue,
};

use super::{
    action::decode_action_dialog_holder, body, default_notice_action, CommonDialogDataState,
    DialogActionState, DialogButtonState, DialogDataState, DialogDecodeContext, DialogState,
    DialogTextState,
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
    let DialogDataState::Notice { common, action } = data else {
        return Ok(None);
    };
    let Some(mut fields) = common_fields(common, context)? else {
        return Ok(None);
    };
    fields.insert(0, ("type", string_len("minecraft:notice")));
    if action != &default_notice_action() {
        let Some(action_len) = button_len(action, context)? else {
            return Ok(None);
        };
        fields.push(("action", action_len));
    }
    Ok(Some(object_len(fields.into_iter())))
}

fn common_fields(
    common: &CommonDialogDataState,
    context: DialogDecodeContext<'_>,
) -> Result<Option<Vec<(&'static str, usize)>>, String> {
    if !common.body_data.is_empty() || !common.input_data.is_empty() {
        return Ok(None);
    }
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
    Ok(Some(fields))
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

fn object_len<'a>(fields: impl Iterator<Item = (&'a str, usize)>) -> usize {
    collection_len(
        fields
            .map(|(name, value_len)| string_len(name).saturating_add(1).saturating_add(value_len)),
    )
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
    fn leaves_non_default_or_unresolved_dialogs_unknown() {
        let non_default = component_with_show_dialog(compound(vec![
            string_entry("type", "notice"),
            string_entry("title", "Hello"),
            value_entry(
                "body",
                compound(vec![
                    string_entry("type", "plain_message"),
                    string_entry("contents", "Body"),
                ]),
            ),
        ]));
        assert_eq!(
            canonical_component_json_utf16_len_with_context(
                &non_default,
                DialogDecodeContext::new(None, 0),
            )
            .unwrap(),
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
