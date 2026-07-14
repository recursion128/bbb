use bbb_protocol::{
    canonical_component_json_utf16_len_with_owner, canonical_json_string_utf16_len,
    CanonicalComponentOwnerValue, NbtSummaryValue,
};

use super::{
    action::decode_action_dialog_holder, body, default_notice_action, DialogDataState,
    DialogDecodeContext, DialogState, DialogTextState,
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
    if common.external_title.is_some()
        || !common.can_close_with_escape
        || !common.pause
        || common.after_action != "close"
        || !common.body_data.is_empty()
        || !common.input_data.is_empty()
        || action != &default_notice_action()
    {
        return Ok(None);
    }
    let Some(title_len) = component_len(&common.title, context)? else {
        return Ok(None);
    };
    Ok(Some(object_len(
        [
            ("type", string_len("minecraft:notice")),
            ("title", title_len),
        ]
        .into_iter(),
    )))
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
            value_entry("pause", NbtSummaryValue::Byte(0)),
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
