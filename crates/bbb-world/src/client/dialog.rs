use bbb_protocol::{
    styled_component_runs_from_nbt_value, styled_runs_summary_text, NbtSummaryEntry,
    NbtSummaryValue, StyledTextRun,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogState {
    pub holder_kind: String,
    pub registry_id: Option<i32>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default, alias = "direct_dialog")]
    pub dialog: Option<NbtSummaryValue>,
    #[serde(default)]
    pub resolution_error: Option<String>,
    #[serde(default)]
    pub data: Option<DialogDataState>,
    #[serde(default)]
    pub semantic_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogDataState {
    Notice {
        common: CommonDialogDataState,
        action: DialogButtonState,
    },
    Confirmation {
        common: CommonDialogDataState,
        yes: DialogButtonState,
        no: DialogButtonState,
    },
    MultiAction {
        common: CommonDialogDataState,
        actions: Vec<DialogButtonState>,
        #[serde(default)]
        exit_action: Option<DialogButtonState>,
        columns: i32,
    },
    ServerLinks {
        common: CommonDialogDataState,
        #[serde(default)]
        exit_action: Option<DialogButtonState>,
        columns: i32,
        button_width: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonDialogDataState {
    pub title: DialogTextState,
    #[serde(default)]
    pub external_title: Option<DialogTextState>,
    pub can_close_with_escape: bool,
    pub pause: bool,
    pub after_action: String,
    #[serde(default)]
    pub body: Vec<NbtSummaryValue>,
    #[serde(default)]
    pub inputs: Vec<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogButtonState {
    pub label: DialogTextState,
    #[serde(default)]
    pub tooltip: Option<DialogTextState>,
    pub width: i32,
    #[serde(default)]
    pub action: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTextState {
    pub text: String,
    #[serde(default)]
    pub runs: Vec<StyledTextRun>,
    pub component: NbtSummaryValue,
}

impl DialogState {
    pub(super) fn direct(dialog: NbtSummaryValue) -> Self {
        Self::resolved("direct", None, None, dialog)
    }

    pub(super) fn reference(
        registry_id: i32,
        resource_id: String,
        dialog: NbtSummaryValue,
    ) -> Self {
        Self::resolved("reference", Some(registry_id), Some(resource_id), dialog)
    }

    pub(super) fn unresolved_reference(
        registry_id: i32,
        resource_id: Option<String>,
        resolution_error: String,
    ) -> Self {
        Self {
            holder_kind: "reference".to_string(),
            registry_id: Some(registry_id),
            resource_id,
            dialog: None,
            resolution_error: Some(resolution_error),
            data: None,
            semantic_error: None,
        }
    }

    fn resolved(
        holder_kind: &str,
        registry_id: Option<i32>,
        resource_id: Option<String>,
        dialog: NbtSummaryValue,
    ) -> Self {
        let (data, semantic_error) = match decode_dialog_data(&dialog) {
            Ok(data) => (Some(data), None),
            Err(error) => (None, Some(error)),
        };
        Self {
            holder_kind: holder_kind.to_string(),
            registry_id,
            resource_id,
            dialog: Some(dialog),
            resolution_error: None,
            data,
            semantic_error,
        }
    }
}

pub(super) fn decode_dialog_data(dialog: &NbtSummaryValue) -> Result<DialogDataState, String> {
    let entries = compound_entries(dialog, "dialog root")?;
    let dialog_type = required_string(entries, "type", "dialog")?;
    let common = decode_common_dialog_data(entries)?;
    match dialog_type {
        "notice" | "minecraft:notice" => Ok(DialogDataState::Notice {
            common,
            action: match find_entry(entries, "action") {
                Some(action) => decode_button(action, "notice action")?,
                None => default_notice_action(),
            },
        }),
        "confirmation" | "minecraft:confirmation" => Ok(DialogDataState::Confirmation {
            common,
            yes: decode_button(
                required_entry(entries, "yes", "confirmation dialog")?,
                "confirmation yes button",
            )?,
            no: decode_button(
                required_entry(entries, "no", "confirmation dialog")?,
                "confirmation no button",
            )?,
        }),
        "multi_action" | "minecraft:multi_action" => Ok(DialogDataState::MultiAction {
            common,
            actions: decode_non_empty_button_list(
                required_entry(entries, "actions", "multi-action dialog")?,
                "multi-action actions",
            )?,
            exit_action: find_entry(entries, "exit_action")
                .map(|value| decode_button(value, "multi-action exit button"))
                .transpose()?,
            columns: optional_positive_i32(entries, "columns", 2, "multi-action dialog")?,
        }),
        "server_links" | "minecraft:server_links" => {
            let button_width = optional_i32(entries, "button_width", 150, "server-links dialog")?;
            if !(1..=1024).contains(&button_width) {
                return Err(format!(
                    "server-links dialog button_width {button_width} is outside 1..=1024"
                ));
            }
            Ok(DialogDataState::ServerLinks {
                common,
                exit_action: find_entry(entries, "exit_action")
                    .map(|value| decode_button(value, "server-links exit button"))
                    .transpose()?,
                columns: optional_positive_i32(entries, "columns", 2, "server-links dialog")?,
                button_width,
            })
        }
        other => Err(format!("unsupported dialog type {other}")),
    }
}

fn decode_common_dialog_data(entries: &[NbtSummaryEntry]) -> Result<CommonDialogDataState, String> {
    let title = decode_component(
        required_entry(entries, "title", "common dialog data")?,
        "dialog title",
    )?;
    let external_title = find_entry(entries, "external_title")
        .map(|value| decode_component(value, "dialog external title"))
        .transpose()?;
    let can_close_with_escape = optional_bool(entries, "can_close_with_escape", true)?;
    let pause = optional_bool(entries, "pause", true)?;
    let after_action = find_entry(entries, "after_action")
        .map(|value| string_value(value, "dialog after_action"))
        .transpose()?
        .unwrap_or("close");
    if !matches!(after_action, "close" | "none" | "wait_for_response") {
        return Err(format!("invalid dialog after_action {after_action}"));
    }
    if pause && after_action == "none" {
        return Err("pausing dialog after_action must close or wait_for_response".to_string());
    }
    let body = match find_entry(entries, "body") {
        None => Vec::new(),
        Some(NbtSummaryValue::List(values)) => values.clone(),
        Some(value) => vec![value.clone()],
    };
    let inputs = optional_list(entries, "inputs", "dialog inputs")?;
    Ok(CommonDialogDataState {
        title,
        external_title,
        can_close_with_escape,
        pause,
        after_action: after_action.to_string(),
        body,
        inputs,
    })
}

fn decode_button(value: &NbtSummaryValue, what: &str) -> Result<DialogButtonState, String> {
    let entries = compound_entries(value, what)?;
    let label = decode_component(required_entry(entries, "label", what)?, "button label")?;
    let tooltip = find_entry(entries, "tooltip")
        .map(|value| decode_component(value, "button tooltip"))
        .transpose()?;
    let width = optional_i32(entries, "width", 150, what)?;
    if !(1..=1024).contains(&width) {
        return Err(format!("{what} width {width} is outside 1..=1024"));
    }
    let action = find_entry(entries, "action").cloned();
    if let Some(action) = &action {
        compound_entries(action, "button action")?;
    }
    Ok(DialogButtonState {
        label,
        tooltip,
        width,
        action,
    })
}

fn decode_non_empty_button_list(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<Vec<DialogButtonState>, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    if values.is_empty() {
        return Err(format!("{what} must not be empty"));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_button(value, &format!("{what}[{index}]")))
        .collect()
}

fn default_notice_action() -> DialogButtonState {
    DialogButtonState {
        label: decode_component(&translated_component("gui.ok"), "default notice label")
            .expect("built-in gui.ok component is valid"),
        tooltip: None,
        width: 150,
        action: None,
    }
}

fn translated_component(key: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "translate".to_string(),
        value: NbtSummaryValue::String(key.to_string()),
    }])
}

fn decode_component(value: &NbtSummaryValue, what: &str) -> Result<DialogTextState, String> {
    match value {
        NbtSummaryValue::String(_) | NbtSummaryValue::Compound(_) => {}
        NbtSummaryValue::List(values) if !values.is_empty() => {}
        _ => return Err(format!("{what} is not a component")),
    }
    let runs = styled_component_runs_from_nbt_value(value);
    Ok(DialogTextState {
        text: styled_runs_summary_text(&runs),
        runs,
        component: value.clone(),
    })
}

fn optional_bool(entries: &[NbtSummaryEntry], key: &str, default: bool) -> Result<bool, String> {
    match find_entry(entries, key) {
        None => Ok(default),
        Some(NbtSummaryValue::Byte(value)) => Ok(*value != 0),
        Some(_) => Err(format!("dialog field {key} is not a boolean")),
    }
}

fn optional_i32(
    entries: &[NbtSummaryEntry],
    key: &str,
    default: i32,
    what: &str,
) -> Result<i32, String> {
    match find_entry(entries, key) {
        None => Ok(default),
        Some(NbtSummaryValue::Byte(value)) => Ok(i32::from(*value)),
        Some(NbtSummaryValue::Short(value)) => Ok(i32::from(*value)),
        Some(NbtSummaryValue::Int(value)) => Ok(*value),
        Some(_) => Err(format!("{what} field {key} is not an integer")),
    }
}

fn optional_positive_i32(
    entries: &[NbtSummaryEntry],
    key: &str,
    default: i32,
    what: &str,
) -> Result<i32, String> {
    let value = optional_i32(entries, key, default, what)?;
    if value <= 0 {
        return Err(format!("{what} field {key} must be positive"));
    }
    Ok(value)
}

fn optional_list(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Vec<NbtSummaryValue>, String> {
    match find_entry(entries, key) {
        None => Ok(Vec::new()),
        Some(NbtSummaryValue::List(values)) => Ok(values.clone()),
        Some(_) => Err(format!("{what} is not a list")),
    }
}

fn compound_entries<'a>(
    value: &'a NbtSummaryValue,
    what: &str,
) -> Result<&'a [NbtSummaryEntry], String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    Ok(entries)
}

fn required_entry<'a>(
    entries: &'a [NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<&'a NbtSummaryValue, String> {
    find_entry(entries, key).ok_or_else(|| format!("{what} is missing field {key}"))
}

fn required_string<'a>(
    entries: &'a [NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<&'a str, String> {
    string_value(required_entry(entries, key, what)?, key)
}

fn string_value<'a>(value: &'a NbtSummaryValue, what: &str) -> Result<&'a str, String> {
    let NbtSummaryValue::String(value) = value else {
        return Err(format!("{what} is not a string"));
    };
    Ok(value)
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_notice_defaults_and_common_dialog_fields() {
        let data = decode_dialog_data(&compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", translated_component("dialog.example.title")),
            value_entry(
                "external_title",
                NbtSummaryValue::String("External".to_string()),
            ),
            value_entry("can_close_with_escape", NbtSummaryValue::Byte(0)),
            value_entry("pause", NbtSummaryValue::Byte(0)),
            string_entry("after_action", "none"),
            value_entry("body", NbtSummaryValue::String("Compact body".to_string())),
            value_entry("inputs", NbtSummaryValue::List(Vec::new())),
        ]))
        .unwrap();

        let DialogDataState::Notice { common, action } = data else {
            panic!("expected notice dialog");
        };
        assert_eq!(common.title.text, "dialog.example.title");
        assert_eq!(
            common
                .external_title
                .as_ref()
                .map(|title| title.text.as_str()),
            Some("External")
        );
        assert!(!common.can_close_with_escape);
        assert!(!common.pause);
        assert_eq!(common.after_action, "none");
        assert_eq!(common.body.len(), 1);
        assert!(common.inputs.is_empty());
        assert_eq!(action.label.text, "gui.ok");
        assert_eq!(action.width, 150);
        assert_eq!(action.action, None);
    }

    #[test]
    fn decodes_confirmation_buttons_tooltip_width_and_raw_action() {
        let yes_action = compound(vec![
            string_entry("type", "minecraft:dynamic/custom"),
            string_entry("id", "example:confirm"),
        ]);
        let data = decode_dialog_data(&compound(vec![
            string_entry("type", "minecraft:confirmation"),
            value_entry("title", NbtSummaryValue::String("Confirm".to_string())),
            value_entry(
                "yes",
                compound(vec![
                    value_entry("label", NbtSummaryValue::String("Yes".to_string())),
                    value_entry("tooltip", NbtSummaryValue::String("Proceed".to_string())),
                    value_entry("width", NbtSummaryValue::Int(200)),
                    value_entry("action", yes_action.clone()),
                ]),
            ),
            value_entry(
                "no",
                compound(vec![value_entry(
                    "label",
                    NbtSummaryValue::String("No".to_string()),
                )]),
            ),
        ]))
        .unwrap();

        let DialogDataState::Confirmation { common, yes, no } = data else {
            panic!("expected confirmation dialog");
        };
        assert_eq!(common.after_action, "close");
        assert!(common.pause);
        assert_eq!(yes.label.text, "Yes");
        assert_eq!(
            yes.tooltip.as_ref().map(|tooltip| tooltip.text.as_str()),
            Some("Proceed")
        );
        assert_eq!(yes.width, 200);
        assert_eq!(yes.action, Some(yes_action));
        assert_eq!(no.label.text, "No");
        assert_eq!(no.width, 150);
    }

    #[test]
    fn rejects_invalid_simple_dialog_semantics_without_dropping_raw_nbt() {
        let invalid_pause = compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", NbtSummaryValue::String("Title".to_string())),
            string_entry("after_action", "none"),
        ]);
        assert!(decode_dialog_data(&invalid_pause)
            .unwrap_err()
            .contains("pausing dialog"));

        let invalid_width = compound(vec![
            string_entry("type", "minecraft:confirmation"),
            value_entry("title", NbtSummaryValue::String("Title".to_string())),
            value_entry(
                "yes",
                compound(vec![
                    value_entry("label", NbtSummaryValue::String("Yes".to_string())),
                    value_entry("width", NbtSummaryValue::Int(0)),
                ]),
            ),
            value_entry(
                "no",
                compound(vec![value_entry(
                    "label",
                    NbtSummaryValue::String("No".to_string()),
                )]),
            ),
        ]);
        let state = DialogState::direct(invalid_width.clone());
        assert_eq!(state.dialog, Some(invalid_width));
        assert_eq!(state.data, None);
        assert!(state
            .semantic_error
            .as_deref()
            .unwrap()
            .contains("outside 1..=1024"));
    }

    #[test]
    fn decodes_multi_action_buttons_exit_and_columns() {
        let data = decode_dialog_data(&compound(vec![
            string_entry("type", "minecraft:multi_action"),
            value_entry("title", NbtSummaryValue::String("Choose".to_string())),
            value_entry(
                "actions",
                NbtSummaryValue::List(vec![button("First", None), button("Second", Some(180))]),
            ),
            value_entry("exit_action", button("Back", Some(200))),
            value_entry("columns", NbtSummaryValue::Int(3)),
        ]))
        .unwrap();

        let DialogDataState::MultiAction {
            common,
            actions,
            exit_action,
            columns,
        } = data
        else {
            panic!("expected multi-action dialog");
        };
        assert_eq!(common.title.text, "Choose");
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].width, 150);
        assert_eq!(actions[1].width, 180);
        assert_eq!(exit_action.unwrap().label.text, "Back");
        assert_eq!(columns, 3);
    }

    #[test]
    fn decodes_server_links_defaults_and_validates_list_dialog_ranges() {
        let data = decode_dialog_data(&compound(vec![
            string_entry("type", "minecraft:server_links"),
            value_entry("title", NbtSummaryValue::String("Links".to_string())),
        ]))
        .unwrap();
        let DialogDataState::ServerLinks {
            common,
            exit_action,
            columns,
            button_width,
        } = data
        else {
            panic!("expected server-links dialog");
        };
        assert_eq!(common.title.text, "Links");
        assert_eq!(exit_action, None);
        assert_eq!(columns, 2);
        assert_eq!(button_width, 150);

        let empty_actions = compound(vec![
            string_entry("type", "minecraft:multi_action"),
            value_entry("title", NbtSummaryValue::String("Empty".to_string())),
            value_entry("actions", NbtSummaryValue::List(Vec::new())),
        ]);
        assert!(decode_dialog_data(&empty_actions)
            .unwrap_err()
            .contains("must not be empty"));

        let invalid_columns = compound(vec![
            string_entry("type", "minecraft:server_links"),
            value_entry("title", NbtSummaryValue::String("Links".to_string())),
            value_entry("columns", NbtSummaryValue::Int(0)),
        ]);
        assert!(decode_dialog_data(&invalid_columns)
            .unwrap_err()
            .contains("columns must be positive"));

        let invalid_width = compound(vec![
            string_entry("type", "minecraft:server_links"),
            value_entry("title", NbtSummaryValue::String("Links".to_string())),
            value_entry("button_width", NbtSummaryValue::Int(1025)),
        ]);
        assert!(decode_dialog_data(&invalid_width)
            .unwrap_err()
            .contains("button_width 1025"));
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

    fn button(label: &str, width: Option<i32>) -> NbtSummaryValue {
        let mut entries = vec![value_entry(
            "label",
            NbtSummaryValue::String(label.to_string()),
        )];
        if let Some(width) = width {
            entries.push(value_entry("width", NbtSummaryValue::Int(width)));
        }
        compound(entries)
    }
}
