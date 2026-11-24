use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(test)]
use crate::RegistrySet;
use crate::{registries::normalize_codec_identifier, FrozenRegistryAccess};

use super::{
    compound_entries, decode_dialog_registry_entry, find_entry, optional_i32, required_entry,
    required_string, DialogDecodeContext, DialogState, DIALOG_REGISTRY,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogActionState {
    #[serde(skip_deserializing)]
    ResourcePackConsent {
        accepted: bool,
        latest_sequence: u64,
    },
    OpenUrl {
        url: bbb_protocol::JavaString,
    },
    RunCommand {
        command: bbb_protocol::JavaString,
    },
    SuggestCommand {
        command: bbb_protocol::JavaString,
    },
    ShowDialog {
        dialog: Box<DialogState>,
    },
    ChangePage {
        page: i32,
    },
    CopyToClipboard {
        value: bbb_protocol::JavaString,
    },
    Custom {
        id: String,
        #[serde(default)]
        payload: Option<NbtSummaryValue>,
    },
    #[serde(rename = "dynamic/run_command")]
    DynamicRunCommand {
        template: DialogCommandTemplateState,
    },
    #[serde(rename = "dynamic/custom")]
    DynamicCustom {
        id: String,
        #[serde(default)]
        additions: Option<NbtSummaryValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogCommandTemplateState {
    pub raw: bbb_protocol::JavaString,
    pub segments: Vec<bbb_protocol::JavaString>,
    /// `StringTemplate.isValidVariableName` keeps these alphanumeric or `_`.
    pub variables: Vec<String>,
}

pub(super) fn decode_eager_click_dialog_holder(
    value: &NbtSummaryValue,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogState, String> {
    let context = DialogDecodeContext::with_registry_access(registry_access, 0);
    match value {
        dialog @ NbtSummaryValue::Compound(_) => {
            checked_action_dialog_state(DialogState::resolved_with_context(
                if context.registry_access.is_some() {
                    "direct"
                } else {
                    "context_free"
                },
                None,
                None,
                dialog.clone(),
                context,
            ))
        }
        NbtSummaryValue::String(resource_id) => {
            let registry_access = context.registry_access.ok_or_else(|| {
                "context-free show-dialog action cannot reference a dialog resource".to_string()
            })?;
            let resource_id = resource_id
                .with_utf8(normalize_codec_identifier)
                .flatten()
                .ok_or_else(|| format!("invalid dialog resource id {resource_id}"))?;
            let entry = registry_access
                .require_entry_by_resource_id(DIALOG_REGISTRY, &resource_id)
                .map_err(|error| error.to_string())?;
            let dialog = decode_dialog_registry_entry(entry)?;
            checked_action_dialog_state(DialogState::resolved_with_context(
                "reference",
                Some(entry.holder_id),
                Some(entry.resource_id.to_string()),
                dialog,
                context.for_entry(entry),
            ))
        }
        _ => Err(
            "show-dialog action holder is neither a resource id nor a dialog compound".to_string(),
        ),
    }
}

pub(super) fn decode_action_dialog_holder_with_context(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogState, String> {
    match value {
        dialog @ NbtSummaryValue::Compound(_) => {
            checked_action_dialog_state(DialogState::resolved_with_context(
                if context.registry_access.is_some() {
                    "direct"
                } else {
                    "context_free"
                },
                None,
                None,
                dialog.clone(),
                context,
            ))
        }
        NbtSummaryValue::String(resource_id) => {
            let registry_access = context.registry_access.ok_or_else(|| {
                "context-free show-dialog action cannot reference a dialog resource".to_string()
            })?;
            let resource_id = resource_id
                .with_utf8(normalize_codec_identifier)
                .flatten()
                .ok_or_else(|| format!("invalid dialog resource id {resource_id}"))?;
            let entry = registry_access
                .require_entry_by_resource_id(DIALOG_REGISTRY, &resource_id)
                .map_err(|error| error.to_string())?;
            if context.budget.is_some() {
                // Keep registry dialog references lazy (matching Vanilla
                // `Holder.Reference<Dialog>`) and queue at-most-once iterative
                // validation on the shared decode budget.
                let _ = decode_dialog_registry_entry(entry)?;
                context.queue_registry_validation(entry.holder_id);
                Ok(DialogState::lazy_reference(
                    entry.holder_id,
                    entry.resource_id.to_string(),
                ))
            } else {
                let dialog = decode_dialog_registry_entry(entry)?;
                checked_action_dialog_state(DialogState::resolved_with_context(
                    "reference",
                    Some(entry.holder_id),
                    Some(entry.resource_id.to_string()),
                    dialog,
                    context.for_entry(entry),
                ))
            }
        }
        _ => Err(
            "show-dialog action holder is neither a resource id nor a dialog compound".to_string(),
        ),
    }
}

fn checked_action_dialog_state(state: DialogState) -> Result<DialogState, String> {
    if let Some(error) = &state.semantic_error {
        return Err(format!("invalid show-dialog action dialog: {error}"));
    }
    Ok(state)
}

#[cfg(test)]
pub(super) fn decode_dialog_action(
    value: &NbtSummaryValue,
    registry_access: Option<FrozenRegistryAccess<'_>>,
    depth: usize,
) -> Result<DialogActionState, String> {
    decode_dialog_action_with_context(
        value,
        DialogDecodeContext::with_registry_access(registry_access, depth),
    )
}

pub(super) fn decode_dialog_action_with_context(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogActionState, String> {
    let entries = compound_entries(value, "button action")?;
    match required_string(entries, "type", "button action")?.as_str() {
        "open_url" | "minecraft:open_url" => {
            let url = required_java_string(entries, "url", "open-url action")?;
            validate_untrusted_url(&String::from_utf16_lossy(url.units()))?;
            Ok(DialogActionState::OpenUrl { url })
        }
        "run_command" | "minecraft:run_command" => Ok(DialogActionState::RunCommand {
            command: decode_chat_string(entries, "command", "run-command action")?,
        }),
        "suggest_command" | "minecraft:suggest_command" => {
            let command = bbb_protocol::JavaString::from_nbt(required_entry(
                entries,
                "command",
                "suggest-command action",
            )?)
            .ok_or_else(|| "suggest-command action command is not a string".to_string())?;
            bbb_protocol::validate_chat_string_units(&command, "suggest-command action")?;
            Ok(DialogActionState::SuggestCommand { command })
        }
        "show_dialog" | "minecraft:show_dialog" => Ok(DialogActionState::ShowDialog {
            dialog: Box::new(decode_action_dialog_holder_with_context(
                required_entry(entries, "dialog", "show-dialog action")?,
                context,
            )?),
        }),
        "change_page" | "minecraft:change_page" => {
            required_entry(entries, "page", "change-page action")?;
            let page = optional_i32(entries, "page", 0, "change-page action")?;
            if page <= 0 {
                return Err(format!("change-page action page {page} must be positive"));
            }
            Ok(DialogActionState::ChangePage { page })
        }
        "copy_to_clipboard" | "minecraft:copy_to_clipboard" => {
            Ok(DialogActionState::CopyToClipboard {
                value: bbb_protocol::JavaString::from_nbt(required_entry(
                    entries,
                    "value",
                    "copy-to-clipboard action",
                )?)
                .ok_or_else(|| "copy-to-clipboard action value is not a string".to_string())?,
            })
        }
        "custom" | "minecraft:custom" => Ok(DialogActionState::Custom {
            id: decode_identifier(entries, "id", "custom action")?,
            payload: find_entry(entries, "payload").cloned(),
        }),
        "dynamic/run_command" | "minecraft:dynamic/run_command" => {
            Ok(DialogActionState::DynamicRunCommand {
                template: parse_template(&required_java_string(
                    entries,
                    "template",
                    "dynamic run-command action",
                )?)?,
            })
        }
        "dynamic/custom" | "minecraft:dynamic/custom" => {
            let additions = find_entry(entries, "additions")
                .map(|value| {
                    compound_entries(value, "dynamic custom action additions")?;
                    Ok::<_, String>(value.clone())
                })
                .transpose()?;
            Ok(DialogActionState::DynamicCustom {
                id: decode_identifier(entries, "id", "dynamic custom action")?,
                additions,
            })
        }
        "open_file" | "minecraft:open_file" => {
            Err("dialog action type open_file is not allowed from a server".to_string())
        }
        action_type => Err(format!("unsupported dialog action type {action_type}")),
    }
}

fn decode_identifier(entries: &[NbtSummaryEntry], key: &str, what: &str) -> Result<String, String> {
    let raw = required_string(entries, key, what)?;
    normalize_codec_identifier(&raw).ok_or_else(|| format!("invalid {what} identifier {raw}"))
}

/// The value's own units, which `StringReader`-shaped server text may hold.
fn required_java_string(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<bbb_protocol::JavaString, String> {
    bbb_protocol::JavaString::from_nbt(required_entry(entries, key, what)?)
        .ok_or_else(|| format!("{what} {key} is not a string"))
}

fn decode_chat_string(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<bbb_protocol::JavaString, String> {
    let value = required_java_string(entries, key, what)?;
    bbb_protocol::validate_chat_string_units(&value, what)?;
    Ok(value)
}

fn validate_untrusted_url(value: &str) -> Result<(), String> {
    if value
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(format!("open-url action has invalid URI {value:?}"));
    }
    let url = Url::parse(value)
        .map_err(|error| format!("open-url action has invalid URI {value:?}: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!(
            "open-url action URI uses unsupported scheme {}",
            url.scheme()
        ));
    }
    Ok(())
}

/// `StringTemplate.fromString` splits on `$(name)` over Java chars, so every
/// literal segment keeps whatever units the server wrote.
fn parse_template(raw: &bbb_protocol::JavaString) -> Result<DialogCommandTemplateState, String> {
    const DOLLAR: u16 = b'$' as u16;
    const OPEN: u16 = b'(' as u16;
    const CLOSE: u16 = b')' as u16;
    let units = raw.units();
    let mut segments = Vec::new();
    let mut variables = Vec::new();
    let mut segment_start = 0;
    let mut cursor = 0;

    while let Some(offset) = units[cursor..].iter().position(|unit| *unit == DOLLAR) {
        let dollar = cursor + offset;
        if units.get(dollar + 1) != Some(&OPEN) {
            cursor = dollar + 1;
            continue;
        }
        segments.push(bbb_protocol::JavaString::from(
            units[segment_start..dollar].to_vec(),
        ));
        let variable_start = dollar + 2;
        let Some(variable_end_offset) = units[variable_start..]
            .iter()
            .position(|unit| *unit == CLOSE)
        else {
            return Err(format!("unterminated dialog action template {raw:?}"));
        };
        let variable_end = variable_start + variable_end_offset;
        let variable = String::from_utf16_lossy(&units[variable_start..variable_end]);
        if !is_valid_variable_name(&variable) {
            return Err(format!(
                "dialog action template variable {variable:?} is invalid"
            ));
        }
        variables.push(variable);
        segment_start = variable_end + 1;
        cursor = segment_start;
    }

    if variables.is_empty() {
        return Err(format!("dialog action template {raw:?} has no variables"));
    }
    if segment_start != units.len() {
        segments.push(bbb_protocol::JavaString::from(
            units[segment_start..].to_vec(),
        ));
    }
    Ok(DialogCommandTemplateState {
        raw: raw.clone(),
        segments,
        variables,
    })
}

fn is_valid_variable_name(value: &str) -> bool {
    value.chars().all(|character| {
        character == '_' || (character.len_utf16() == 1 && character.is_alphanumeric())
    })
}

#[cfg(test)]
mod tests {
    use crate::{RegistryPacketEntry, WorldStore};

    use super::*;

    #[test]
    fn decodes_static_action_variants_and_codec_fields() {
        let actions = [
            action(vec![
                string_entry("type", "open_url"),
                string_entry("url", "https://example.invalid/path?q=1"),
            ]),
            action(vec![
                string_entry("type", "run_command"),
                string_entry("command", "say hello"),
            ]),
            action(vec![
                string_entry("type", "suggest_command"),
                string_entry("command", "/help"),
            ]),
            action(vec![
                string_entry("type", "change_page"),
                value_entry("page", NbtSummaryValue::Int(3)),
            ]),
            action(vec![
                string_entry("type", "copy_to_clipboard"),
                string_entry("value", "copied"),
            ]),
            action(vec![
                string_entry("type", "custom"),
                string_entry("id", "example:static"),
                value_entry("payload", NbtSummaryValue::string("payload")),
            ]),
        ];
        let decoded = actions
            .iter()
            .map(|action| {
                decode_dialog_action(action, Some(RegistrySet::default().frozen_access()), 1)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(matches!(
            &decoded[0],
            DialogActionState::OpenUrl { url } if url == "https://example.invalid/path?q=1"
        ));
        assert!(matches!(
            &decoded[1],
            DialogActionState::RunCommand { command } if command == "say hello"
        ));
        assert!(matches!(
            &decoded[2],
            DialogActionState::SuggestCommand { command } if command == "/help"
        ));
        assert!(matches!(
            decoded[3],
            DialogActionState::ChangePage { page: 3 }
        ));
        assert!(matches!(
            &decoded[4],
            DialogActionState::CopyToClipboard { value } if value == "copied"
        ));
        assert!(matches!(
            &decoded[5],
            DialogActionState::Custom { id, payload: Some(NbtSummaryValue::String(payload)) }
                if id == "example:static" && payload == "payload"
        ));
    }

    #[test]
    fn rejects_disallowed_static_actions_and_field_validation_failures() {
        for action in [
            action(vec![
                string_entry("type", "open_file"),
                string_entry("path", "/tmp/not-allowed"),
            ]),
            action(vec![
                string_entry("type", "open_url"),
                string_entry("url", "file:///tmp/not-allowed"),
            ]),
            action(vec![
                string_entry("type", "open_url"),
                string_entry("url", "https://example.invalid/bad path"),
            ]),
            action(vec![
                string_entry("type", "run_command"),
                string_entry("command", "say\nno"),
            ]),
            action(vec![
                string_entry("type", "change_page"),
                value_entry("page", NbtSummaryValue::Int(0)),
            ]),
            action(vec![
                string_entry("type", "custom"),
                string_entry("id", "invalid id"),
            ]),
        ] {
            assert!(
                decode_dialog_action(&action, Some(RegistrySet::default().frozen_access()), 1,)
                    .is_err()
            );
        }
    }

    #[test]
    fn decodes_dynamic_templates_and_custom_additions() {
        let command = decode_dialog_action(
            &action(vec![
                string_entry("type", "minecraft:dynamic/run_command"),
                string_entry("template", "give $(target) $(item)_$(item)"),
            ]),
            None,
            1,
        )
        .unwrap();
        let DialogActionState::DynamicRunCommand { template } = command else {
            panic!("expected dynamic run-command action");
        };
        assert_eq!(template.raw, "give $(target) $(item)_$(item)");
        assert_eq!(template.segments, ["give ", " ", "_"]);
        assert_eq!(template.variables, ["target", "item", "item"]);

        let additions = compound(vec![value_entry("fixed", NbtSummaryValue::Int(7))]);
        let custom = decode_dialog_action(
            &action(vec![
                string_entry("type", "dynamic/custom"),
                string_entry("id", "example:dynamic"),
                value_entry("additions", additions.clone()),
            ]),
            None,
            1,
        )
        .unwrap();
        assert_eq!(
            custom,
            DialogActionState::DynamicCustom {
                id: "example:dynamic".to_string(),
                additions: Some(additions),
            }
        );

        for template in ["say constant", "say $(unterminated", "say $(bad-name)"] {
            assert!(decode_dialog_action(
                &action(vec![
                    string_entry("type", "dynamic/run_command"),
                    string_entry("template", template),
                ]),
                None,
                1,
            )
            .is_err());
        }
    }

    #[test]
    fn decodes_inline_and_referenced_show_dialog_actions() {
        let inline = decode_dialog_action(
            &action(vec![
                string_entry("type", "show_dialog"),
                value_entry("dialog", notice("Inline")),
            ]),
            None,
            1,
        )
        .unwrap();
        let DialogActionState::ShowDialog { dialog } = inline else {
            panic!("expected show-dialog action");
        };
        assert_eq!(dialog.holder_kind, "context_free");
        assert!(matches!(
            dialog.data,
            Some(super::super::DialogDataState::Notice { .. })
        ));

        let mut store = WorldStore::new();
        store.record_registry_entries(
            super::super::DIALOG_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:next",
                br#"{"type":"minecraft:notice","title":"Referenced"}"#.to_vec(),
            )],
        );
        let referenced = decode_dialog_action(
            &action(vec![
                string_entry("type", "show_dialog"),
                string_entry("dialog", "example:next"),
            ]),
            Some(store.frozen_registry_access()),
            1,
        )
        .unwrap();
        let DialogActionState::ShowDialog { dialog } = referenced else {
            panic!("expected show-dialog action");
        };
        assert_eq!(dialog.holder_kind, "reference");
        assert_eq!(dialog.registry_id, Some(0));
        assert_eq!(dialog.resource_id.as_deref(), Some("example:next"));

        assert!(decode_dialog_action(
            &action(vec![
                string_entry("type", "show_dialog"),
                string_entry("dialog", "example:next"),
            ]),
            None,
            1,
        )
        .is_err());
        assert!(decode_dialog_action(
            &action(vec![
                string_entry("type", "show_dialog"),
                value_entry("dialog", notice("Too Deep")),
            ]),
            None,
            super::super::MAX_DIALOG_SEMANTIC_DEPTH + 1,
        )
        .is_err());
    }

    fn action(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        compound(entries)
    }

    fn notice(title: &str) -> NbtSummaryValue {
        compound(vec![
            string_entry("type", "notice"),
            value_entry("title", NbtSummaryValue::string(title)),
        ])
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
        value_entry(name, NbtSummaryValue::string(value))
    }
}
