use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(test)]
use crate::RegistrySet;
use crate::{registries::normalize_codec_identifier, FrozenRegistryAccess};

use super::{
    compound_entries, decode_dialog_registry_entry, find_entry, optional_i32, required_entry,
    required_string, DialogState, DIALOG_REGISTRY,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogActionState {
    OpenUrl {
        url: String,
    },
    RunCommand {
        command: String,
    },
    SuggestCommand {
        command: String,
    },
    ShowDialog {
        dialog: Box<DialogState>,
    },
    ChangePage {
        page: i32,
    },
    CopyToClipboard {
        value: String,
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
    pub raw: String,
    pub segments: Vec<String>,
    pub variables: Vec<String>,
}

pub(super) fn decode_action_dialog_holder(
    value: &NbtSummaryValue,
    registry_access: Option<FrozenRegistryAccess<'_>>,
    depth: usize,
) -> Result<DialogState, String> {
    match value {
        dialog @ NbtSummaryValue::Compound(_) => {
            checked_action_dialog_state(DialogState::resolved_at_depth(
                if registry_access.is_some() {
                    "direct"
                } else {
                    "context_free"
                },
                None,
                None,
                dialog.clone(),
                registry_access,
                depth,
            ))
        }
        NbtSummaryValue::String(resource_id) => {
            let registry_access = registry_access.ok_or_else(|| {
                "context-free show-dialog action cannot reference a dialog resource".to_string()
            })?;
            let resource_id = normalize_codec_identifier(resource_id)
                .ok_or_else(|| format!("invalid dialog resource id {resource_id}"))?;
            let entry = registry_access
                .require_entry_by_resource_id(DIALOG_REGISTRY, &resource_id)
                .map_err(|error| error.to_string())?;
            let dialog = decode_dialog_registry_entry(entry)?.clone();
            checked_action_dialog_state(DialogState::resolved_at_depth(
                "reference",
                Some(entry.holder_id),
                Some(entry.resource_id.to_string()),
                dialog,
                Some(registry_access),
                depth,
            ))
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

pub(super) fn decode_dialog_action(
    value: &NbtSummaryValue,
    registry_access: Option<FrozenRegistryAccess<'_>>,
    depth: usize,
) -> Result<DialogActionState, String> {
    let entries = compound_entries(value, "button action")?;
    match required_string(entries, "type", "button action")? {
        "open_url" | "minecraft:open_url" => {
            let url = required_string(entries, "url", "open-url action")?;
            validate_untrusted_url(url)?;
            Ok(DialogActionState::OpenUrl {
                url: url.to_string(),
            })
        }
        "run_command" | "minecraft:run_command" => Ok(DialogActionState::RunCommand {
            command: decode_chat_string(entries, "command", "run-command action")?.to_string(),
        }),
        "suggest_command" | "minecraft:suggest_command" => Ok(DialogActionState::SuggestCommand {
            command: decode_chat_string(entries, "command", "suggest-command action")?.to_string(),
        }),
        "show_dialog" | "minecraft:show_dialog" => Ok(DialogActionState::ShowDialog {
            dialog: Box::new(decode_action_dialog_holder(
                required_entry(entries, "dialog", "show-dialog action")?,
                registry_access,
                depth,
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
                value: required_string(entries, "value", "copy-to-clipboard action")?.to_string(),
            })
        }
        "custom" | "minecraft:custom" => Ok(DialogActionState::Custom {
            id: decode_identifier(entries, "id", "custom action")?,
            payload: find_entry(entries, "payload").cloned(),
        }),
        "dynamic/run_command" | "minecraft:dynamic/run_command" => {
            Ok(DialogActionState::DynamicRunCommand {
                template: parse_template(required_string(
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
    normalize_codec_identifier(raw).ok_or_else(|| format!("invalid {what} identifier {raw}"))
}

fn decode_chat_string<'a>(
    entries: &'a [NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<&'a str, String> {
    let value = required_string(entries, key, what)?;
    if let Some(character) = value
        .chars()
        .find(|character| *character == '§' || (*character as u32) < 32 || *character == '\u{7f}')
    {
        return Err(format!(
            "{what} contains disallowed chat character {character:?}"
        ));
    }
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

fn parse_template(raw: &str) -> Result<DialogCommandTemplateState, String> {
    let mut segments = Vec::new();
    let mut variables = Vec::new();
    let mut segment_start = 0;
    let mut search_start = 0;

    while let Some(offset) = raw[search_start..].find('$') {
        let dollar = search_start + offset;
        if raw.as_bytes().get(dollar + 1) == Some(&b'(') {
            segments.push(raw[segment_start..dollar].to_string());
            let variable_start = dollar + 2;
            let Some(variable_end_offset) = raw[variable_start..].find(')') else {
                return Err(format!("unterminated dialog action template {raw:?}"));
            };
            let variable_end = variable_start + variable_end_offset;
            let variable = &raw[variable_start..variable_end];
            if !is_valid_variable_name(variable) {
                return Err(format!(
                    "dialog action template variable {variable:?} is invalid"
                ));
            }
            variables.push(variable.to_string());
            segment_start = variable_end + 1;
            search_start = segment_start;
        } else {
            search_start = dollar + 1;
        }
    }

    if variables.is_empty() {
        return Err(format!("dialog action template {raw:?} has no variables"));
    }
    if segment_start != raw.len() {
        segments.push(raw[segment_start..].to_string());
    }
    Ok(DialogCommandTemplateState {
        raw: raw.to_string(),
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
                value_entry("payload", NbtSummaryValue::String("payload".to_string())),
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
            value_entry("title", NbtSummaryValue::String(title.to_string())),
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
}
