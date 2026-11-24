use std::cell::RefCell;

use bbb_protocol::{
    content_registry::ContentRegistry, styled_component_runs_from_nbt_value,
    styled_runs_summary_text, visit_component_payloads_with_registry, ComponentPayload,
    NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};
use bbb_registry_codec::registry_codec::{RegistryElementSource, RegistryElementValueRef};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::{
    registries::normalize_codec_identifier, FrozenRegistryAccess, FrozenRegistryEntry, RegistrySet,
};

mod action;
mod body;
mod input;
mod pack_prompt;

pub use action::{DialogActionState, DialogCommandTemplateState};
#[cfg(test)]
pub(crate) use body::decode_persistent_component_select_value;
pub(crate) use body::{
    decode_command_item_stack_template_summary, validate_command_item_component,
    validate_command_item_predicate_value, validate_command_item_stack_template,
};
pub use body::{
    DialogBodyState, DialogDataComponentPatchEntryState, DialogDataComponentPatchState,
    DialogDataComponentValueState, DialogItemStackTemplateState, DialogPlainMessageState,
};
pub use input::{
    DialogInputControlState, DialogInputState, DialogNumberRangeState,
    DialogSingleOptionEntryState, DialogTextMultilineState,
};

const MAX_DIALOG_SEMANTIC_DEPTH: usize = 64;
const MAX_DIALOG_DECODE_NODES: usize = 1024;
pub(super) const DIALOG_REGISTRY: &str = "minecraft:dialog";

#[derive(Debug, Default)]
pub(super) struct DialogDecodeBudget {
    decoded_nodes: usize,
    validated_registry_ids: FxHashSet<i32>,
    pending_registry_ids: Vec<i32>,
}

impl DialogDecodeBudget {
    fn consume_node(&mut self) -> Result<(), String> {
        if self.decoded_nodes >= MAX_DIALOG_DECODE_NODES {
            return Err(format!(
                "dialog decode exceeds {MAX_DIALOG_DECODE_NODES} nodes"
            ));
        }
        self.decoded_nodes += 1;
        Ok(())
    }

    fn queue_registry_validation(&mut self, holder_id: i32) {
        if self.validated_registry_ids.insert(holder_id) {
            self.pending_registry_ids.push(holder_id);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DialogDecodeContext<'a> {
    registry_access: Option<FrozenRegistryAccess<'a>>,
    depth: usize,
    json_source: bool,
    prepare_item_summary: bool,
    budget: Option<&'a RefCell<DialogDecodeBudget>>,
}

impl<'a> DialogDecodeContext<'a> {
    fn new(registries: Option<&'a RegistrySet>, depth: usize) -> Self {
        Self::with_registry_access(registries.map(RegistrySet::frozen_access), depth)
    }

    fn with_registry_access(
        registry_access: Option<FrozenRegistryAccess<'a>>,
        depth: usize,
    ) -> Self {
        Self {
            registry_access,
            depth,
            json_source: false,
            prepare_item_summary: true,
            budget: None,
        }
    }

    fn with_budget<'b>(self, budget: &'b RefCell<DialogDecodeBudget>) -> DialogDecodeContext<'b>
    where
        'a: 'b,
    {
        DialogDecodeContext {
            registry_access: self.registry_access,
            depth: self.depth,
            json_source: self.json_source,
            prepare_item_summary: self.prepare_item_summary,
            budget: Some(budget),
        }
    }

    fn consume_node(self) -> Result<(), String> {
        if let Some(budget) = self.budget {
            budget.borrow_mut().consume_node()?;
        }
        Ok(())
    }

    pub(super) fn queue_registry_validation(self, holder_id: i32) {
        if let Some(budget) = self.budget {
            budget.borrow_mut().queue_registry_validation(holder_id);
        }
    }

    fn nested(self) -> Self {
        Self {
            depth: self.depth.saturating_add(1),
            ..self
        }
    }

    fn for_entry(self, entry: FrozenRegistryEntry<'_>) -> Self {
        self.for_source(
            entry
                .element_data
                .map_or(RegistryElementSource::NetworkNbt, |data| data.source),
        )
    }

    fn for_source(mut self, source: RegistryElementSource) -> Self {
        self.json_source = source != RegistryElementSource::NetworkNbt;
        self
    }
}

fn drain_pending_registry_dialog_validations(
    context: DialogDecodeContext<'_>,
    budget: &RefCell<DialogDecodeBudget>,
) -> Result<(), String> {
    let Some(registry_access) = context.registry_access else {
        return Ok(());
    };
    let root_context = DialogDecodeContext {
        depth: 0,
        ..context.with_budget(budget)
    };
    loop {
        let next_holder_id = budget.borrow_mut().pending_registry_ids.pop();
        let Some(holder_id) = next_holder_id else {
            return Ok(());
        };
        let Some(entry) = registry_access.entry_by_holder_id(DIALOG_REGISTRY, holder_id) else {
            continue;
        };
        let dialog = decode_dialog_registry_entry(entry)?;
        decode_dialog_data_with_budget(&dialog, root_context.for_entry(entry))?;
    }
}

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
    DialogList {
        common: CommonDialogDataState,
        dialogs: DialogHolderSetState,
        #[serde(default)]
        exit_action: Option<DialogButtonState>,
        columns: i32,
        button_width: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogHolderSetState {
    pub holder_set_kind: String,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub entries: Vec<DialogListEntryState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogListEntryState {
    pub holder_kind: String,
    #[serde(default)]
    pub registry_id: Option<i32>,
    #[serde(default)]
    pub resource_id: Option<String>,
    pub external_title: DialogTextState,
    #[serde(default)]
    pub dialog: Option<NbtSummaryValue>,
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
    pub body_data: Vec<DialogBodyState>,
    #[serde(default)]
    pub inputs: Vec<NbtSummaryValue>,
    #[serde(default)]
    pub input_data: Vec<DialogInputState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogButtonState {
    pub label: DialogTextState,
    #[serde(default)]
    pub tooltip: Option<DialogTextState>,
    pub width: i32,
    #[serde(default)]
    pub action: Option<NbtSummaryValue>,
    #[serde(default)]
    pub action_data: Option<DialogActionState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogTextState {
    pub text: String,
    #[serde(default)]
    pub runs: Vec<StyledTextRun>,
    pub component: NbtSummaryValue,
}

impl DialogState {
    pub(super) fn direct(dialog: NbtSummaryValue, registries: &RegistrySet) -> Self {
        Self::resolved(
            "direct",
            None,
            None,
            dialog,
            Some(registries.frozen_access()),
        )
    }

    pub(super) fn context_free(dialog: NbtSummaryValue) -> Self {
        Self::resolved("context_free", None, None, dialog, None)
    }

    pub(super) fn reference(
        registry_id: i32,
        resource_id: String,
        dialog: NbtSummaryValue,
        registries: &RegistrySet,
        entry: FrozenRegistryEntry<'_>,
    ) -> Self {
        Self::resolved_with_context(
            "reference",
            Some(registry_id),
            Some(resource_id),
            dialog,
            DialogDecodeContext::new(Some(registries), 0).for_entry(entry),
        )
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
        registry_access: Option<FrozenRegistryAccess<'_>>,
    ) -> Self {
        Self::resolved_at_depth(
            holder_kind,
            registry_id,
            resource_id,
            dialog,
            registry_access,
            0,
        )
    }

    fn resolved_at_depth(
        holder_kind: &str,
        registry_id: Option<i32>,
        resource_id: Option<String>,
        dialog: NbtSummaryValue,
        registry_access: Option<FrozenRegistryAccess<'_>>,
        depth: usize,
    ) -> Self {
        Self::resolved_with_context(
            holder_kind,
            registry_id,
            resource_id,
            dialog,
            DialogDecodeContext::with_registry_access(registry_access, depth),
        )
    }

    pub(super) fn lazy_reference(registry_id: i32, resource_id: String) -> Self {
        Self {
            holder_kind: "reference".to_string(),
            registry_id: Some(registry_id),
            resource_id: Some(resource_id),
            dialog: None,
            resolution_error: None,
            data: None,
            semantic_error: None,
        }
    }

    fn resolved_with_context(
        holder_kind: &str,
        registry_id: Option<i32>,
        resource_id: Option<String>,
        dialog: NbtSummaryValue,
        context: DialogDecodeContext<'_>,
    ) -> Self {
        let decoded = if let Some(budget) = context.budget {
            if let Some(id) = registry_id {
                budget.borrow_mut().validated_registry_ids.insert(id);
            }
            decode_dialog_data_with_budget(&dialog, context)
        } else {
            let budget = RefCell::new(DialogDecodeBudget::default());
            if let Some(id) = registry_id {
                budget.borrow_mut().validated_registry_ids.insert(id);
            }
            let budgeted = context.with_budget(&budget);
            decode_dialog_data_with_budget(&dialog, budgeted).and_then(|data| {
                drain_pending_registry_dialog_validations(budgeted, &budget)?;
                Ok(data)
            })
        };
        let (data, semantic_error) = match decoded {
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

#[cfg(test)]
pub(super) fn decode_dialog_data(
    dialog: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<DialogDataState, String> {
    decode_dialog_data_with_registry_access(dialog, registries.map(RegistrySet::frozen_access))
}

fn decode_dialog_data_with_registry_access(
    dialog: &NbtSummaryValue,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogDataState, String> {
    decode_dialog_data_with_context(
        dialog,
        DialogDecodeContext::with_registry_access(registry_access, 0),
    )
}

pub(crate) fn decode_component_click_dialog(
    dialog: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<DialogState, String> {
    action::decode_eager_click_dialog_holder(dialog, registries.map(RegistrySet::frozen_access))
}

#[cfg(test)]
thread_local! {
    pub(crate) static STACK_PROBE_EXTREMES: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((0, usize::MAX)) };
}

fn decode_dialog_data_with_context(
    dialog: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogDataState, String> {
    if context.budget.is_some() {
        return decode_dialog_data_with_budget(dialog, context);
    }
    let budget = RefCell::new(DialogDecodeBudget::default());
    let budgeted = context.with_budget(&budget);
    let data = decode_dialog_data_with_budget(dialog, budgeted)?;
    drain_pending_registry_dialog_validations(budgeted, &budget)?;
    Ok(data)
}

fn decode_dialog_data_with_budget(
    dialog: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogDataState, String> {
    #[cfg(test)]
    {
        let marker = 0u8;
        let address = std::ptr::from_ref(&marker) as usize;
        STACK_PROBE_EXTREMES.with(|cell| {
            let (top, bottom) = cell.get();
            cell.set((top.max(address), bottom.min(address)));
        });
    }
    context.consume_node()?;
    if context.depth > MAX_DIALOG_SEMANTIC_DEPTH {
        return Err(format!(
            "dialog semantic nesting exceeds {MAX_DIALOG_SEMANTIC_DEPTH}"
        ));
    }
    let entries = compound_entries(dialog, "dialog root")?;
    let dialog_type = required_string(entries, "type", "dialog")?;
    let common = decode_common_dialog_data(entries, context)?;
    match dialog_type.as_str() {
        "notice" | "minecraft:notice" => Ok(DialogDataState::Notice {
            common,
            action: match find_entry(entries, "action") {
                Some(action) => decode_button(action, "notice action", context)?,
                None => default_notice_action(),
            },
        }),
        "confirmation" | "minecraft:confirmation" => Ok(DialogDataState::Confirmation {
            common,
            yes: decode_button(
                required_entry(entries, "yes", "confirmation dialog")?,
                "confirmation yes button",
                context,
            )?,
            no: decode_button(
                required_entry(entries, "no", "confirmation dialog")?,
                "confirmation no button",
                context,
            )?,
        }),
        "multi_action" | "minecraft:multi_action" => Ok(DialogDataState::MultiAction {
            common,
            actions: decode_non_empty_button_list(
                required_entry(entries, "actions", "multi-action dialog")?,
                "multi-action actions",
                context,
            )?,
            exit_action: find_entry(entries, "exit_action")
                .map(|value| decode_button(value, "multi-action exit button", context))
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
                    .map(|value| decode_button(value, "server-links exit button", context))
                    .transpose()?,
                columns: optional_positive_i32(entries, "columns", 2, "server-links dialog")?,
                button_width,
            })
        }
        "dialog_list" | "minecraft:dialog_list" => {
            let button_width = optional_i32(entries, "button_width", 150, "dialog-list dialog")?;
            if !(1..=1024).contains(&button_width) {
                return Err(format!(
                    "dialog-list dialog button_width {button_width} is outside 1..=1024"
                ));
            }
            Ok(DialogDataState::DialogList {
                common,
                dialogs: decode_dialog_holder_set(
                    required_entry(entries, "dialogs", "dialog-list dialog")?,
                    context,
                )?,
                exit_action: find_entry(entries, "exit_action")
                    .map(|value| decode_button(value, "dialog-list exit button", context))
                    .transpose()?,
                columns: optional_positive_i32(entries, "columns", 2, "dialog-list dialog")?,
                button_width,
            })
        }
        other => Err(format!("unsupported dialog type {other}")),
    }
}

fn decode_dialog_holder_set(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogHolderSetState, String> {
    let Some(registry_access) = context.registry_access else {
        return decode_context_free_dialog_holder_set(value, context);
    };
    if let NbtSummaryValue::String(tag) = value {
        if let Some(tag) = tag.strip_prefix('#') {
            let tag = tag
                .with_utf8(normalize_codec_identifier)
                .flatten()
                .ok_or_else(|| format!("invalid dialog tag #{tag}"))?;
            let holder_set = registry_access
                .holder_set_by_tag(DIALOG_REGISTRY, &tag)
                .map_err(|error| error.to_string())?;
            let entries = holder_set
                .entries
                .into_iter()
                .map(|entry| decode_referenced_dialog_entry(entry, context))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(DialogHolderSetState {
                holder_set_kind: "named".to_string(),
                tag: Some(tag),
                entries,
            });
        }
    }

    let values = match value {
        NbtSummaryValue::List(values) => values.as_slice(),
        value => std::slice::from_ref(value),
    };
    let holder_kind = values.first().map(dialog_holder_kind).transpose()?;
    let mut entries = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let value_kind = dialog_holder_kind(value)?;
        if holder_kind != Some(value_kind) {
            return Err(format!(
                "dialog holder set mixes {} and {value_kind} entries at index {index}",
                holder_kind.unwrap_or("unknown")
            ));
        }
        entries.push(match value {
            NbtSummaryValue::String(resource_id) => {
                let resource_id = resource_id
                    .with_utf8(normalize_codec_identifier)
                    .flatten()
                    .ok_or_else(|| format!("invalid dialog resource id {resource_id}"))?;
                let entry = registry_access
                    .require_entry_by_resource_id(DIALOG_REGISTRY, &resource_id)
                    .map_err(|error| error.to_string())?;
                decode_referenced_dialog_entry(entry, context)?
            }
            dialog @ NbtSummaryValue::Compound(_) => {
                let data = decode_dialog_data_with_context(dialog, context.nested())?;
                DialogListEntryState {
                    holder_kind: "direct".to_string(),
                    registry_id: None,
                    resource_id: None,
                    external_title: dialog_external_title(&data),
                    dialog: Some(dialog.clone()),
                }
            }
            _ => unreachable!("dialog_holder_kind accepted only strings and compounds"),
        });
    }
    Ok(DialogHolderSetState {
        holder_set_kind: "direct".to_string(),
        tag: None,
        entries,
    })
}

fn decode_context_free_dialog_holder_set(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogHolderSetState, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err("context-free dialog holder set is not a list".to_string());
    };
    let entries = values
        .iter()
        .enumerate()
        .map(|(index, dialog)| {
            if !matches!(dialog, NbtSummaryValue::Compound(_)) {
                return Err(format!(
                    "context-free dialog holder set entry {index} is not an inline dialog compound"
                ));
            }
            let data = decode_dialog_data_with_context(dialog, context.nested())?;
            Ok(DialogListEntryState {
                holder_kind: "direct".to_string(),
                registry_id: None,
                resource_id: None,
                external_title: dialog_external_title(&data),
                dialog: Some(dialog.clone()),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogHolderSetState {
        holder_set_kind: "direct".to_string(),
        tag: None,
        entries,
    })
}

fn dialog_holder_kind(value: &NbtSummaryValue) -> Result<&'static str, String> {
    match value {
        NbtSummaryValue::String(_) => Ok("reference"),
        NbtSummaryValue::Compound(_) => Ok("direct"),
        _ => Err(
            "dialog holder set entry is neither a resource id nor a dialog compound".to_string(),
        ),
    }
}

fn decode_referenced_dialog_entry(
    entry: crate::FrozenRegistryEntry<'_>,
    context: DialogDecodeContext<'_>,
) -> Result<DialogListEntryState, String> {
    context.consume_node()?;
    let dialog = decode_dialog_registry_entry(entry)?;
    let entries = compound_entries(&dialog, "dialog root")?;
    let entry_context = context.for_entry(entry);
    let title_value = find_entry(entries, "external_title")
        .unwrap_or(required_entry(entries, "title", "dialog")?);
    let external_title = decode_component(title_value, "dialog external title", entry_context)?;
    context.queue_registry_validation(entry.holder_id);
    Ok(DialogListEntryState {
        holder_kind: "reference".to_string(),
        registry_id: Some(entry.holder_id),
        resource_id: Some(entry.resource_id.to_string()),
        external_title,
        dialog: None,
    })
}

pub(super) fn decode_dialog_registry_entry<'a>(
    entry: FrozenRegistryEntry<'a>,
) -> Result<NbtSummaryValue, String> {
    if let Some(error) = entry.element_data_error {
        return Err(format!(
            "invalid dialog registry entry {}: {error}",
            entry.resource_id
        ));
    }
    let Some(data) = entry.element_data else {
        return Err(format!(
            "dialog registry entry {} has no synchronized payload",
            entry.resource_id
        ));
    };
    match data
        .source_order_nbt_projection()
        .map_err(|error| error.to_string())?
    {
        dialog @ NbtSummaryValue::Compound(_) => Ok(dialog),
        _ => Err(format!(
            "dialog registry entry {} root is not a compound tag",
            entry.resource_id
        )),
    }
}

fn dialog_external_title(data: &DialogDataState) -> DialogTextState {
    let common = match data {
        DialogDataState::Notice { common, .. }
        | DialogDataState::Confirmation { common, .. }
        | DialogDataState::MultiAction { common, .. }
        | DialogDataState::ServerLinks { common, .. }
        | DialogDataState::DialogList { common, .. } => common,
    };
    common
        .external_title
        .clone()
        .unwrap_or_else(|| common.title.clone())
}

fn decode_common_dialog_data(
    entries: &[NbtSummaryEntry],
    context: DialogDecodeContext<'_>,
) -> Result<CommonDialogDataState, String> {
    let title = decode_component(
        required_entry(entries, "title", "common dialog data")?,
        "dialog title",
        context,
    )?;
    let external_title = find_entry(entries, "external_title")
        .map(|value| decode_component(value, "dialog external title", context))
        .transpose()?;
    let can_close_with_escape = optional_bool(entries, "can_close_with_escape", true)?;
    let pause = optional_bool(entries, "pause", true)?;
    let after_action = find_entry(entries, "after_action")
        .map(|value| string_value(value, "dialog after_action"))
        .transpose()?
        .unwrap_or_else(|| "close".to_string());
    if !matches!(
        after_action.as_str(),
        "close" | "none" | "wait_for_response"
    ) {
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
    let body_data = body::decode_dialog_bodies(&body, context)?;
    let input_data = input::decode_dialog_inputs_with_context(&inputs, context)?;
    Ok(CommonDialogDataState {
        title,
        external_title,
        can_close_with_escape,
        pause,
        after_action: after_action.to_string(),
        body,
        body_data,
        inputs,
        input_data,
    })
}

fn decode_button(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogButtonState, String> {
    let entries = compound_entries(value, what)?;
    let label = decode_component(
        required_entry(entries, "label", what)?,
        "button label",
        context,
    )?;
    let tooltip = find_entry(entries, "tooltip")
        .map(|value| decode_component(value, "button tooltip", context))
        .transpose()?;
    let width = optional_i32(entries, "width", 150, what)?;
    if !(1..=1024).contains(&width) {
        return Err(format!("{what} width {width} is outside 1..=1024"));
    }
    let action = find_entry(entries, "action").cloned();
    let action_data = action
        .as_ref()
        .map(|action| action::decode_dialog_action_with_context(action, context.nested()))
        .transpose()?;
    Ok(DialogButtonState {
        label,
        tooltip,
        width,
        action,
        action_data,
    })
}

fn decode_non_empty_button_list(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
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
        .map(|(index, value)| decode_button(value, &format!("{what}[{index}]"), context))
        .collect()
}

fn default_notice_action() -> DialogButtonState {
    DialogButtonState {
        label: decode_component(
            &translated_component("gui.ok"),
            "default notice label",
            DialogDecodeContext::new(None, 0),
        )
        .expect("built-in gui.ok component is valid"),
        tooltip: None,
        width: 150,
        action: None,
        action_data: None,
    }
}

fn translated_component(key: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "translate".into(),
        value: NbtSummaryValue::string(key),
    }])
}

fn decode_component(
    value: &NbtSummaryValue,
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogTextState, String> {
    let registry = context.registry_access.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        FrozenRegistryAccess::content_registry,
    );
    visit_component_payloads_with_registry(value, what, registry, |payload, payload_what| {
        match payload {
            ComponentPayload::PlayerProfile(profile) => {
                body::validate_component_player_profile(profile, payload_what)
            }
            ComponentPayload::ShowDialog(dialog) => {
                action::decode_action_dialog_holder_with_context(dialog, context.nested())
                    .map(|_| ())
                    .map_err(|error| format!("{payload_what}: {error}"))
            }
            ComponentPayload::ShowItemComponents(components) => {
                body::decode_data_component_patch(components, context)
                    .map(|_| ())
                    .map_err(|error| format!("{payload_what}: {error}"))
            }
        }
    })?;
    let runs = styled_component_runs_from_nbt_value(value);
    Ok(DialogTextState {
        text: styled_runs_summary_text(&runs),
        runs,
        component: value.clone(),
    })
}

pub(crate) fn validate_rich_component_with_registry(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    let budget = RefCell::new(DialogDecodeBudget::default());
    let context =
        DialogDecodeContext::with_registry_access(Some(registry_access), 0).with_budget(&budget);
    decode_component(value, "command component", context)?;
    drain_pending_registry_dialog_validations(context, &budget)
}

pub(crate) fn validate_command_rich_text(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    validate_rich_component_with_registry(value, registry_access)
}

pub(crate) fn validate_command_style(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err("command style is not an SNBT compound".to_string());
    };
    let mut component_entries = entries.clone();
    component_entries.push(NbtSummaryEntry {
        name: "text".into(),
        value: NbtSummaryValue::string(""),
    });
    validate_command_rich_text(
        &NbtSummaryValue::Compound(component_entries),
        registry_access,
    )
}

pub(crate) fn validate_command_dialog(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    decode_dialog_data_with_registry_access(value, Some(registry_access)).map(|_| ())
}

fn optional_bool(entries: &[NbtSummaryEntry], key: &str, default: bool) -> Result<bool, String> {
    match find_entry(entries, key) {
        None => Ok(default),
        Some(value) => RegistryElementValueRef::Nbt(value)
            .as_bool()
            .ok_or_else(|| format!("dialog field {key} is not a boolean")),
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
        Some(value) => RegistryElementValueRef::Nbt(value)
            .codec_i32()
            .ok_or_else(|| format!("{what} field {key} is not an integer")),
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

fn required_string(entries: &[NbtSummaryEntry], key: &str, what: &str) -> Result<String, String> {
    string_value(required_entry(entries, key, what)?, key)
}

fn string_value(value: &NbtSummaryValue, what: &str) -> Result<String, String> {
    let NbtSummaryValue::String(value) = value else {
        return Err(format!("{what} is not a string"));
    };
    value
        .try_to_string()
        .map_err(|_| format!("{what} contains invalid UTF-16"))
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{RegistryTags, TagNetworkPayload, UpdateTags};

    use crate::{RegistryPacketEntry, WorldStore};

    use super::*;

    #[test]
    fn decodes_notice_defaults_and_common_dialog_fields() {
        let data = decode_test_dialog(&compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", translated_component("dialog.example.title")),
            value_entry("external_title", NbtSummaryValue::string("External")),
            value_entry("can_close_with_escape", NbtSummaryValue::Byte(0)),
            value_entry("pause", NbtSummaryValue::Byte(0)),
            string_entry("after_action", "none"),
            value_entry(
                "body",
                compound(vec![
                    string_entry("type", "minecraft:plain_message"),
                    value_entry("contents", NbtSummaryValue::string("Compact body")),
                ]),
            ),
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
        assert!(matches!(
            common.body_data.as_slice(),
            [DialogBodyState::PlainMessage { message }] if message.contents.text == "Compact body" && message.width == 200
        ));
        assert!(common.inputs.is_empty());
        assert!(common.input_data.is_empty());
        assert_eq!(action.label.text, "gui.ok");
        assert_eq!(action.width, 150);
        assert_eq!(action.action, None);
    }

    #[test]
    fn projects_common_dialog_inputs_while_retaining_raw_values() {
        let raw_input = compound(vec![
            string_entry("key", "confirmed"),
            string_entry("type", "minecraft:boolean"),
            value_entry("label", NbtSummaryValue::string("Confirmed")),
            value_entry("initial", NbtSummaryValue::Byte(1)),
        ]);
        let data = decode_test_dialog(&compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", NbtSummaryValue::string("Input")),
            value_entry("inputs", NbtSummaryValue::List(vec![raw_input.clone()])),
        ]))
        .unwrap();

        let DialogDataState::Notice { common, .. } = data else {
            panic!("expected notice dialog");
        };
        assert_eq!(common.inputs, vec![raw_input]);
        assert_eq!(common.input_data.len(), 1);
        assert_eq!(common.input_data[0].key, "confirmed");
        assert!(matches!(
            common.input_data[0].control,
            DialogInputControlState::Boolean { initial: true, .. }
        ));
    }

    #[test]
    fn projects_repairable_item_component_with_dialog_registry_context() {
        let dialog = compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", NbtSummaryValue::string("Repair")),
            value_entry(
                "body",
                compound(vec![
                    string_entry("type", "minecraft:item"),
                    value_entry(
                        "item",
                        compound(vec![
                            string_entry("id", "stone"),
                            value_entry(
                                "components",
                                compound(vec![value_entry(
                                    "repairable",
                                    compound(vec![string_entry("items", "diamond")]),
                                )]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);

        let data = decode_test_dialog(&dialog).unwrap();
        let DialogDataState::Notice { common, .. } = data else {
            panic!("expected notice dialog");
        };
        let [DialogBodyState::Item { item, .. }] = common.body_data.as_slice() else {
            panic!("expected item dialog body");
        };
        let [DialogDataComponentPatchEntryState {
            value_data: Some(DialogDataComponentValueState::Repairable { items }),
            ..
        }] = item.component_data.as_ref().unwrap().entries.as_slice()
        else {
            panic!("expected typed repairable component");
        };
        assert_eq!(items.holder_set_kind, "direct");
        assert_eq!(items.entries.len(), 1);
        assert_eq!(items.entries[0].resource_id, "minecraft:diamond");

        let context_free = DialogState::context_free(dialog.clone());
        assert_eq!(context_free.dialog, Some(dialog));
        assert!(context_free
            .semantic_error
            .as_deref()
            .unwrap()
            .contains("without registry context"));
    }

    #[test]
    fn decodes_confirmation_buttons_tooltip_width_and_raw_action() {
        let yes_action = compound(vec![
            string_entry("type", "minecraft:dynamic/custom"),
            string_entry("id", "example:confirm"),
        ]);
        let data = decode_test_dialog(&compound(vec![
            string_entry("type", "minecraft:confirmation"),
            value_entry("title", NbtSummaryValue::string("Confirm")),
            value_entry(
                "yes",
                compound(vec![
                    value_entry("label", NbtSummaryValue::string("Yes")),
                    value_entry("tooltip", NbtSummaryValue::string("Proceed")),
                    value_entry("width", NbtSummaryValue::Int(200)),
                    value_entry("action", yes_action.clone()),
                ]),
            ),
            value_entry(
                "no",
                compound(vec![value_entry("label", NbtSummaryValue::string("No"))]),
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
        assert!(matches!(
            yes.action_data,
            Some(DialogActionState::DynamicCustom {
                ref id,
                additions: None,
            }) if id == "example:confirm"
        ));
        assert_eq!(no.label.text, "No");
        assert_eq!(no.width, 150);
        assert_eq!(no.action_data, None);
    }

    #[test]
    fn rejects_invalid_simple_dialog_semantics_without_dropping_raw_nbt() {
        let invalid_pause = compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", NbtSummaryValue::string("Title")),
            string_entry("after_action", "none"),
        ]);
        assert!(decode_test_dialog(&invalid_pause)
            .unwrap_err()
            .contains("pausing dialog"));

        let invalid_width = compound(vec![
            string_entry("type", "minecraft:confirmation"),
            value_entry("title", NbtSummaryValue::string("Title")),
            value_entry(
                "yes",
                compound(vec![
                    value_entry("label", NbtSummaryValue::string("Yes")),
                    value_entry("width", NbtSummaryValue::Int(0)),
                ]),
            ),
            value_entry(
                "no",
                compound(vec![value_entry("label", NbtSummaryValue::string("No"))]),
            ),
        ]);
        let state = DialogState::direct(invalid_width.clone(), &RegistrySet::default());
        assert_eq!(state.dialog, Some(invalid_width));
        assert_eq!(state.data, None);
        assert!(state
            .semantic_error
            .as_deref()
            .unwrap()
            .contains("outside 1..=1024"));
    }

    #[test]
    fn dialog_components_reject_non_numeric_hover_item_damage() {
        let title = compound(vec![
            string_entry("text", "Hover"),
            value_entry(
                "hover_event",
                compound(vec![
                    string_entry("action", "show_item"),
                    string_entry("id", "diamond"),
                    value_entry(
                        "components",
                        compound(vec![string_entry("damage", "not numeric")]),
                    ),
                ]),
            ),
        ]);
        assert!(decode_test_dialog(&compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", title),
        ]))
        .is_err());
    }

    #[test]
    fn dialog_components_decode_player_object_profiles_through_central_owner() {
        let dialog = |profile| {
            compound(vec![
                string_entry("type", "minecraft:notice"),
                value_entry(
                    "title",
                    compound(vec![
                        string_entry("type", "object"),
                        string_entry("object", "player"),
                        value_entry("player", profile),
                    ]),
                ),
            ])
        };
        decode_test_dialog(&dialog(NbtSummaryValue::string("Steve"))).unwrap();

        assert!(decode_test_dialog(&dialog(compound(vec![string_entry(
            "name",
            "abcdefghijklmnopq",
        )])))
        .is_err());
    }

    #[test]
    fn dialog_components_decode_inline_click_dialog_values_through_central_owner() {
        let title =
            component_with_show_dialog(compound(vec![string_entry("type", "minecraft:notice")]));
        let error = decode_test_dialog(&compound(vec![
            string_entry("type", "minecraft:notice"),
            value_entry("title", title),
        ]))
        .unwrap_err();
        assert!(
            error.contains("dialog title field click_event field dialog")
                && error.contains("missing field title"),
            "{error:?}"
        );
    }

    #[test]
    fn dialog_item_components_preserve_click_dialog_decode_context() {
        let page =
            component_with_show_dialog(compound(vec![string_entry("type", "minecraft:notice")]));
        for component in [
            value_entry("custom_name", page.clone()),
            value_entry(
                "written_book_content",
                compound(vec![
                    string_entry("title", "Book"),
                    string_entry("author", "Author"),
                    value_entry("pages", NbtSummaryValue::List(vec![page.clone()])),
                ]),
            ),
            value_entry(
                "attribute_modifiers",
                NbtSummaryValue::List(vec![compound(vec![
                    string_entry("type", "armor"),
                    string_entry("id", "example:armor"),
                    value_entry("amount", NbtSummaryValue::Int(1)),
                    string_entry("operation", "add_value"),
                    value_entry(
                        "display",
                        compound(vec![
                            string_entry("type", "override"),
                            value_entry("value", page.clone()),
                        ]),
                    ),
                ])]),
            ),
            value_entry(
                "instrument",
                compound(vec![
                    string_entry("sound_event", "block.note_block.harp"),
                    value_entry("use_duration", NbtSummaryValue::Int(1)),
                    value_entry("range", NbtSummaryValue::Int(1)),
                    value_entry("description", page.clone()),
                ]),
            ),
            value_entry(
                "provides_trim_material",
                compound(vec![
                    string_entry("asset_name", "quartz"),
                    value_entry("description", page.clone()),
                ]),
            ),
            value_entry(
                "trim",
                compound(vec![
                    value_entry(
                        "material",
                        compound(vec![
                            string_entry("asset_name", "quartz"),
                            string_entry("description", "Quartz"),
                        ]),
                    ),
                    value_entry(
                        "pattern",
                        compound(vec![
                            string_entry("asset_id", "coast"),
                            value_entry("description", page.clone()),
                        ]),
                    ),
                ]),
            ),
            value_entry(
                "bundle_contents",
                NbtSummaryValue::List(vec![compound(vec![
                    string_entry("id", "stone"),
                    value_entry(
                        "components",
                        compound(vec![value_entry("custom_name", page)]),
                    ),
                ])]),
            ),
        ] {
            assert!(decode_test_dialog(&compound(vec![
                string_entry("type", "minecraft:notice"),
                string_entry("title", "Item"),
                value_entry(
                    "body",
                    compound(vec![
                        string_entry("type", "minecraft:item"),
                        value_entry(
                            "item",
                            compound(vec![
                                string_entry("id", "stone"),
                                value_entry("components", compound(vec![component])),
                            ]),
                        ),
                    ]),
                ),
            ]))
            .is_err());
        }
    }

    #[test]
    fn dialog_components_resolve_click_dialog_references_and_bound_cycles() {
        // The full-depth self-reference walk needs an explicit stack: at
        // opt-level 0 the 64 semantic levels consume more than the default
        // 2 MiB test-thread stack. The companion budget test below pins the
        // actual consumption.
        run_with_deep_dialog_stack(|| {
            let mut store = WorldStore::new();
            store.record_registry_entries(
                DIALOG_REGISTRY,
                0,
                vec![
                    RegistryPacketEntry::with_raw_data(
                        "example:next",
                        notice_dialog_nbt("Referenced", None),
                    ),
                    RegistryPacketEntry::with_raw_data(
                        "example:self",
                        click_dialog_notice_nbt("example:self"),
                    ),
                ],
            );

            let referencing = |resource_id: &str| {
                compound(vec![
                    string_entry("type", "minecraft:notice"),
                    value_entry(
                        "title",
                        component_with_show_dialog(NbtSummaryValue::string(resource_id)),
                    ),
                ])
            };
            decode_dialog_data(&referencing("example:next"), Some(store.registries())).unwrap();

            // Circular registry dialog menus are valid in Vanilla 26.1 because
            // registry references resolve lazily without expanding copies.
            decode_dialog_data(&referencing("example:self"), Some(store.registries())).unwrap();
        });
    }

    /// Reverse guard for `MAX_DIALOG_SEMANTIC_DEPTH`: circular and DAG
    /// registry references decode iteratively in O(V + E) within the stack
    /// budget without exponential blowup.
    #[test]
    fn full_depth_dialog_reference_walk_stays_within_stack_budget() {
        const STACK_BUDGET_BYTES: usize = 4 * 1024 * 1024;
        run_with_deep_dialog_stack(|| {
            let mut store = WorldStore::new();
            let mut entries = Vec::new();
            for level in 0..33 {
                let next = format!("example:d{}", level + 1);
                let title = if level == 32 {
                    notice_dialog_nbt("Leaf", None)
                } else {
                    // Two-branch DAG: 2^32 paths if expanded eagerly.
                    let mut nbt = Vec::new();
                    nbt.push(10);
                    write_nbt_named_string(&mut nbt, "type", "minecraft:notice");
                    nbt.push(10);
                    write_nbt_string(&mut nbt, "title");
                    write_nbt_named_string(&mut nbt, "text", "Branch");
                    nbt.push(9);
                    write_nbt_string(&mut nbt, "extra");
                    nbt.push(10);
                    nbt.extend_from_slice(&2_i32.to_be_bytes());
                    for _ in 0..2 {
                        write_nbt_named_string(&mut nbt, "text", "Go");
                        nbt.push(10);
                        write_nbt_string(&mut nbt, "click_event");
                        write_nbt_named_string(&mut nbt, "action", "show_dialog");
                        write_nbt_named_string(&mut nbt, "dialog", &next);
                        nbt.push(0);
                        nbt.push(0);
                    }
                    nbt.push(0);
                    nbt.push(0);
                    nbt
                };
                entries.push(RegistryPacketEntry::with_raw_data(
                    format!("example:d{level}"),
                    title,
                ));
            }
            store.record_registry_entries(DIALOG_REGISTRY, 0, entries);
            let referencing = compound(vec![
                string_entry("type", "minecraft:notice"),
                value_entry(
                    "title",
                    component_with_show_dialog(NbtSummaryValue::string("example:d0")),
                ),
            ]);
            let caller_marker = 0u8;
            reset_dialog_stack_probe(&caller_marker);
            decode_dialog_data(&referencing, Some(store.registries())).unwrap();
            let used = dialog_stack_used_bytes();
            assert!(
                used <= STACK_BUDGET_BYTES,
                "33-level binary DAG dialog walk used {used} bytes of \
                 stack, over the {STACK_BUDGET_BYTES}-byte budget"
            );
        });
    }

    #[test]
    fn dialog_holder_set_stack_budget_pins_direct_recursive_boundary() {
        const STACK_BUDGET_BYTES: usize = 4 * 1024 * 1024;

        run_with_deep_dialog_stack(|| {
            let caller_marker = 0u8;
            let accepted = nested_direct_dialog_holder_set(MAX_DIALOG_SEMANTIC_DEPTH);
            reset_dialog_stack_probe(&caller_marker);
            let decoded = decode_test_dialog(&accepted).unwrap();
            assert!(matches!(decoded, DialogDataState::DialogList { .. }));
            let used = dialog_stack_used_bytes();
            assert!(
                used <= STACK_BUDGET_BYTES,
                "depth-{MAX_DIALOG_SEMANTIC_DEPTH} direct dialog HolderSet walk used {used} bytes \
                 of stack, over the {STACK_BUDGET_BYTES}-byte budget"
            );

            let rejected = nested_direct_dialog_holder_set(MAX_DIALOG_SEMANTIC_DEPTH + 1);
            let error = decode_test_dialog(&rejected).unwrap_err();
            assert!(
                error.contains("dialog semantic nesting exceeds 64"),
                "{error}"
            );

            decode_test_dialog(&compound(vec![
                string_entry("type", "minecraft:notice"),
                string_entry("title", "Recovered"),
            ]))
            .unwrap();
        });
    }

    fn reset_dialog_stack_probe(caller_marker: &u8) {
        let address = std::ptr::from_ref(caller_marker) as usize;
        super::STACK_PROBE_EXTREMES.with(|cell| cell.set((address, address)));
    }

    fn dialog_stack_used_bytes() -> usize {
        super::STACK_PROBE_EXTREMES.with(|cell| {
            let (top, bottom) = cell.get();
            top.saturating_sub(bottom)
        })
    }

    fn run_with_deep_dialog_stack(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(test)
            .expect("spawn deep-dialog test thread")
            .join()
            .expect("deep-dialog test thread");
    }

    #[test]
    fn decodes_multi_action_buttons_exit_and_columns() {
        let data = decode_test_dialog(&compound(vec![
            string_entry("type", "minecraft:multi_action"),
            value_entry("title", NbtSummaryValue::string("Choose")),
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
        let data = decode_test_dialog(&compound(vec![
            string_entry("type", "minecraft:server_links"),
            value_entry("title", NbtSummaryValue::string("Links")),
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
            value_entry("title", NbtSummaryValue::string("Empty")),
            value_entry("actions", NbtSummaryValue::List(Vec::new())),
        ]);
        assert!(decode_test_dialog(&empty_actions)
            .unwrap_err()
            .contains("must not be empty"));

        let invalid_columns = compound(vec![
            string_entry("type", "minecraft:server_links"),
            value_entry("title", NbtSummaryValue::string("Links")),
            value_entry("columns", NbtSummaryValue::Int(0)),
        ]);
        assert!(decode_test_dialog(&invalid_columns)
            .unwrap_err()
            .contains("columns must be positive"));

        let invalid_width = compound(vec![
            string_entry("type", "minecraft:server_links"),
            value_entry("title", NbtSummaryValue::string("Links")),
            value_entry("button_width", NbtSummaryValue::Int(1025)),
        ]);
        assert!(decode_test_dialog(&invalid_width)
            .unwrap_err()
            .contains("button_width 1025"));
    }

    #[test]
    fn decodes_dialog_list_tag_in_session_tag_order() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:first",
                    br#"{"type":"minecraft:notice","title":"First"}"#.to_vec(),
                ),
                RegistryPacketEntry::with_raw_data(
                    "example:second",
                    notice_dialog_nbt("Second", Some("External Second")),
                ),
            ],
        );
        store.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: DIALOG_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:menu".to_string(),
                    entries: vec![1, 0],
                }],
            }],
        });

        let data = decode_dialog_data(
            &compound(vec![
                string_entry("type", "minecraft:dialog_list"),
                value_entry("title", NbtSummaryValue::string("Menu")),
                string_entry("dialogs", "#example:menu"),
                value_entry("exit_action", button("Back", None)),
                value_entry("columns", NbtSummaryValue::Int(3)),
                value_entry("button_width", NbtSummaryValue::Int(220)),
            ]),
            Some(store.registries()),
        )
        .unwrap();

        let DialogDataState::DialogList {
            common,
            dialogs,
            exit_action,
            columns,
            button_width,
        } = data
        else {
            panic!("expected dialog-list dialog");
        };
        assert_eq!(common.title.text, "Menu");
        assert_eq!(dialogs.holder_set_kind, "named");
        assert_eq!(dialogs.tag.as_deref(), Some("example:menu"));
        assert_eq!(dialogs.entries.len(), 2);
        assert_eq!(dialogs.entries[0].registry_id, Some(1));
        assert_eq!(
            dialogs.entries[0].resource_id.as_deref(),
            Some("example:second")
        );
        assert_eq!(dialogs.entries[0].external_title.text, "External Second");
        assert_eq!(dialogs.entries[1].registry_id, Some(0));
        assert_eq!(dialogs.entries[1].external_title.text, "First");
        assert_eq!(exit_action.unwrap().label.text, "Back");
        assert_eq!(columns, 3);
        assert_eq!(button_width, 220);
    }

    #[test]
    fn decodes_compact_dialog_holders_and_rejects_invalid_holder_sets() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DIALOG_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:first",
                    notice_dialog_nbt("First", None),
                ),
                RegistryPacketEntry::stub("example:stub"),
            ],
        );
        let referenced = decode_dialog_data(
            &dialog_list(NbtSummaryValue::string("example:first")),
            Some(store.registries()),
        )
        .unwrap();
        let DialogDataState::DialogList {
            dialogs,
            exit_action,
            columns,
            button_width,
            ..
        } = referenced
        else {
            panic!("expected dialog-list dialog");
        };
        assert_eq!(dialogs.holder_set_kind, "direct");
        assert_eq!(dialogs.tag, None);
        assert_eq!(dialogs.entries.len(), 1);
        assert_eq!(dialogs.entries[0].holder_kind, "reference");
        assert_eq!(dialogs.entries[0].external_title.text, "First");
        assert_eq!(dialogs.entries[0].dialog, None);
        assert_eq!(exit_action, None);
        assert_eq!(columns, 2);
        assert_eq!(button_width, 150);

        let inline = decode_dialog_data(
            &dialog_list(compound(vec![
                string_entry("type", "minecraft:notice"),
                value_entry("title", NbtSummaryValue::string("Inline")),
            ])),
            Some(store.registries()),
        )
        .unwrap();
        let DialogDataState::DialogList { dialogs, .. } = inline else {
            panic!("expected dialog-list dialog");
        };
        assert_eq!(dialogs.entries[0].holder_kind, "direct");
        assert_eq!(dialogs.entries[0].registry_id, None);
        assert_eq!(dialogs.entries[0].external_title.text, "Inline");
        assert!(matches!(
            dialogs.entries[0].dialog,
            Some(NbtSummaryValue::Compound(_))
        ));

        let mixed = dialog_list(NbtSummaryValue::List(vec![
            NbtSummaryValue::string("example:first"),
            compound(vec![
                string_entry("type", "minecraft:notice"),
                value_entry("title", NbtSummaryValue::string("Inline")),
            ]),
        ]));
        assert!(decode_dialog_data(&mixed, Some(store.registries()))
            .unwrap_err()
            .contains("mixes reference and direct"));
        assert!(decode_dialog_data(
            &dialog_list(NbtSummaryValue::string("#example:missing")),
            Some(store.registries()),
        )
        .unwrap_err()
        .contains("missing minecraft:dialog tag"));
        assert!(decode_dialog_data(
            &dialog_list(NbtSummaryValue::string("example:stub")),
            Some(store.registries()),
        )
        .unwrap_err()
        .contains("has no synchronized payload"));
    }

    #[test]
    fn decodes_context_free_dialog_list_requires_inline_list() {
        let context_free = decode_dialog_data(
            &dialog_list(NbtSummaryValue::List(vec![compound(vec![
                string_entry("type", "minecraft:notice"),
                value_entry("title", NbtSummaryValue::string("Context Free")),
            ])])),
            None,
        )
        .unwrap();
        let DialogDataState::DialogList { dialogs, .. } = context_free else {
            panic!("expected dialog-list dialog");
        };
        assert_eq!(dialogs.entries[0].external_title.text, "Context Free");
        assert!(decode_dialog_data(
            &dialog_list(compound(vec![
                string_entry("type", "minecraft:notice"),
                value_entry("title", NbtSummaryValue::string("Scalar")),
            ])),
            None,
        )
        .unwrap_err()
        .contains("context-free dialog holder set is not a list"));
        assert!(decode_dialog_data(
            &dialog_list(NbtSummaryValue::List(vec![NbtSummaryValue::string(
                "example:referenced"
            )])),
            None,
        )
        .unwrap_err()
        .contains("is not an inline dialog compound"));
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn decode_test_dialog(dialog: &NbtSummaryValue) -> Result<DialogDataState, String> {
        decode_dialog_data(dialog, Some(&RegistrySet::default()))
    }

    fn dialog_list(dialogs: NbtSummaryValue) -> NbtSummaryValue {
        compound(vec![
            string_entry("type", "minecraft:dialog_list"),
            value_entry("title", NbtSummaryValue::string("Menu")),
            value_entry("dialogs", dialogs),
        ])
    }

    fn nested_direct_dialog_holder_set(depth: usize) -> NbtSummaryValue {
        assert!(depth > 0);
        let mut dialog = compound(vec![
            string_entry("type", "minecraft:notice"),
            string_entry("title", "Leaf"),
        ]);
        for _ in 0..depth {
            dialog = dialog_list(NbtSummaryValue::List(vec![dialog]));
        }
        dialog
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

    fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
        value_entry(name, NbtSummaryValue::string(value))
    }

    fn button(label: &str, width: Option<i32>) -> NbtSummaryValue {
        let mut entries = vec![value_entry("label", NbtSummaryValue::string(label))];
        if let Some(width) = width {
            entries.push(value_entry("width", NbtSummaryValue::Int(width)));
        }
        compound(entries)
    }

    fn notice_dialog_nbt(title: &str, external_title: Option<&str>) -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_named_string(&mut out, "type", "minecraft:notice");
        write_nbt_named_string(&mut out, "title", title);
        if let Some(external_title) = external_title {
            write_nbt_named_string(&mut out, "external_title", external_title);
        }
        out.push(0);
        out
    }

    fn click_dialog_notice_nbt(resource_id: &str) -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_named_string(&mut out, "type", "minecraft:notice");
        write_nbt_named_compound(&mut out, "title", |out| {
            write_nbt_named_string(out, "text", "Open");
            write_nbt_named_compound(out, "click_event", |out| {
                write_nbt_named_string(out, "action", "show_dialog");
                write_nbt_named_string(out, "dialog", resource_id);
            });
        });
        out.push(0);
        out
    }

    fn write_nbt_named_compound(
        out: &mut Vec<u8>,
        name: &str,
        write_entries: impl FnOnce(&mut Vec<u8>),
    ) {
        out.push(10);
        write_nbt_string(out, name);
        write_entries(out);
        out.push(0);
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
