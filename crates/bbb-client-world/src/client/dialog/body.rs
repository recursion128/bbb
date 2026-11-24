#[cfg(test)]
use crate::registries::item_component_values::{
    materialize_patch_named_holder_sets, read_persistent_component_into,
};
use std::sync::Arc;

#[cfg(test)]
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;

#[cfg(test)]
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
};
#[cfg(test)]
use bbb_protocol::item_component_value_types::ItemRaritySummary;
#[cfg(test)]
use bbb_protocol::packets::validate_data_component_patch_summary;
use bbb_protocol::packets::{
    DataComponentPatchSummary, ItemStackSummary, ItemStackTemplateSummary,
};
#[cfg(test)]
use bbb_protocol::tropical_fish_patterns::VANILLA_TROPICAL_FISH_PATTERN_FLOPPER_PACKED_ID;
use bbb_protocol::{
    data_component_types::vanilla_data_component_type_for_resource_id,
    item_types::vanilla_item_type, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::RegistrySet;
use crate::{registries::normalize_codec_identifier, FrozenRegistryAccess};

use super::{
    compound_entries, decode_component, find_entry, optional_bool, optional_i32, required_entry,
    required_string, DialogDecodeContext, DialogTextState,
};

mod component_value;

#[cfg(test)]
use component_value::prepare_component_patch_summary;
pub use component_value::DialogDataComponentValueState;
use component_value::{
    decode_supported_component_value_with_patch, prepare_component_patch_summary_into,
    validate_component_player_profile as validate_player_profile,
    validate_supported_component_value,
};

pub(super) fn validate_component_player_profile(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<(), String> {
    validate_player_profile(value, what)
}

pub(crate) fn validate_command_item_component(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<bool, String> {
    validate_supported_component_value(
        type_id,
        resource_id,
        value,
        DialogDecodeContext::with_registry_access(Some(registry_access), 0),
    )
}

pub(crate) fn validate_command_item_predicate_value(
    resource_id: &str,
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    component_value::validate_command_item_predicate_value(resource_id, value, registry_access)
}

pub(crate) fn validate_command_item_stack_template(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    decode_item_stack_template(
        value,
        DialogDecodeContext::with_registry_access(Some(registry_access), 0),
    )
    .map(|_| ())
}

pub(crate) fn decode_command_item_stack_template_summary(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Result<ItemStackSummary, String> {
    let decoded = decode_item_stack_template(
        value,
        DialogDecodeContext::with_registry_access(Some(registry_access), 0),
    )?;
    decoded
        .prepared_summary
        .map(Arc::unwrap_or_clone)
        .ok_or_else(|| "item stack template component patch is not fully projectable".to_string())
}

/// Decodes one component-select `when` value through the selected persistent
/// component codec.
#[cfg(test)]
pub(crate) fn decode_persistent_component_select_value(
    type_id: i32,
    value: &serde_json::Value,
    registries: Option<&RegistrySet>,
) -> Result<Option<DataComponentPatchSummary>, String> {
    let value = bbb_protocol::json_value_to_nbt_summary(value)
        .map_err(|error| format!("component select value is not representable as NBT: {error}"))?;
    decode_persistent_data_component_patch_entry_with_context(
        type_id,
        &value,
        registries.map(RegistrySet::frozen_access),
        bbb_registry_codec::registry_codec::RegistryElementSource::ClientKnownPackJson,
    )
}

#[cfg(test)]
fn decode_persistent_data_component_patch_entry_with_context(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    _source: bbb_registry_codec::registry_codec::RegistryElementSource,
) -> Result<Option<DataComponentPatchSummary>, String> {
    let mut patch = DataComponentPatchSummary::default();
    read_persistent_component_into(type_id, value, registries, &mut patch)?;
    materialize_patch_named_holder_sets(&mut patch, registries)?;
    validate_data_component_patch_summary(&patch).map_err(|error| error.to_string())?;
    Ok(Some(patch))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogBodyState {
    PlainMessage {
        message: DialogPlainMessageState,
    },
    Item {
        item: DialogItemStackTemplateState,
        #[serde(default)]
        description: Option<DialogPlainMessageState>,
        show_decorations: bool,
        show_tooltip: bool,
        width: i32,
        height: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogPlainMessageState {
    pub contents: DialogTextState,
    pub width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogItemStackTemplateState {
    pub resource_id: String,
    pub count: i32,
    #[serde(default)]
    pub components: Option<NbtSummaryValue>,
    #[serde(default)]
    pub component_data: Option<DialogDataComponentPatchState>,
    #[serde(skip)]
    prepared_summary: Option<Arc<ItemStackSummary>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDataComponentPatchState {
    pub entries: Vec<DialogDataComponentPatchEntryState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDataComponentPatchEntryState {
    pub type_id: i32,
    pub resource_id: String,
    pub removed: bool,
    #[serde(default)]
    pub value: Option<NbtSummaryValue>,
    #[serde(default)]
    pub value_data: Option<DialogDataComponentValueState>,
}

impl PartialEq for DialogItemStackTemplateState {
    fn eq(&self, other: &Self) -> bool {
        self.resource_id == other.resource_id
            && self.count == other.count
            && self.components == other.components
            && self.component_data == other.component_data
    }
}

impl Eq for DialogItemStackTemplateState {}

impl DialogItemStackTemplateState {
    pub fn item_stack_summary_for_protocol_id(
        &self,
        item_id: i32,
    ) -> Option<Arc<ItemStackSummary>> {
        let summary = self.prepared_summary.as_ref()?;
        (summary.item_id == Some(item_id)).then(|| Arc::clone(summary))
    }
}

pub(super) fn decode_dialog_bodies(
    values: &[NbtSummaryValue],
    context: DialogDecodeContext<'_>,
) -> Result<Vec<DialogBodyState>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_dialog_body(value, index, context))
        .collect()
}

fn decode_dialog_body(
    value: &NbtSummaryValue,
    index: usize,
    context: DialogDecodeContext<'_>,
) -> Result<DialogBodyState, String> {
    let what = format!("dialog body[{index}]");
    let entries = compound_entries(value, &what)?;
    match required_string(entries, "type", &what)?.as_str() {
        "plain_message" | "minecraft:plain_message" => Ok(DialogBodyState::PlainMessage {
            message: decode_plain_message_map(entries, &what, context)?,
        }),
        "item" | "minecraft:item" => {
            let width = optional_i32(entries, "width", 16, "item dialog body")?;
            validate_range(width, 1, 256, "item dialog body width")?;
            let height = optional_i32(entries, "height", 16, "item dialog body")?;
            validate_range(height, 1, 256, "item dialog body height")?;
            Ok(DialogBodyState::Item {
                item: decode_item_stack_template(required_entry(entries, "item", &what)?, context)?,
                description: find_entry(entries, "description")
                    .map(|value| decode_plain_message(value, context))
                    .transpose()?,
                show_decorations: optional_bool(entries, "show_decorations", true)?,
                show_tooltip: optional_bool(entries, "show_tooltip", true)?,
                width,
                height,
            })
        }
        body_type => Err(format!("unsupported dialog body type {body_type}")),
    }
}

fn decode_plain_message(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogPlainMessageState, String> {
    if let NbtSummaryValue::Compound(entries) = value {
        if find_entry(entries, "contents").is_some() {
            return decode_plain_message_map(entries, "dialog plain message", context);
        }
    }
    Ok(DialogPlainMessageState {
        contents: decode_component(value, "dialog plain message contents", context)?,
        width: 200,
    })
}

fn decode_plain_message_map(
    entries: &[NbtSummaryEntry],
    what: &str,
    context: DialogDecodeContext<'_>,
) -> Result<DialogPlainMessageState, String> {
    let width = optional_i32(entries, "width", 200, what)?;
    validate_range(width, 1, 1024, &format!("{what} width"))?;
    Ok(DialogPlainMessageState {
        contents: decode_component(
            required_entry(entries, "contents", what)?,
            "dialog plain message contents",
            context,
        )?,
        width,
    })
}

fn decode_item_stack_template(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogItemStackTemplateState, String> {
    let content_registry = context.registry_access.map_or(
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
        |registries| registries.content_registry(),
    );
    let fields = bbb_registry_codec::read_item_stack_template_fields_with_content_registry(
        value,
        content_registry,
    )?;
    let mut prepared_summary = context.prepare_item_summary.then(|| {
        Arc::new(ItemStackSummary {
            item_id: Some(fields.item_id),
            count: fields.count,
            component_patch: DataComponentPatchSummary::default().into(),
        })
    });
    let component_data = fields
        .components
        .map(|components| {
            let patch = prepared_summary
                .as_mut()
                .map(|summary| &mut *Arc::get_mut(summary).unwrap().component_patch);
            decode_data_component_patch_into(components, context, patch, None)
        })
        .transpose()?;
    Ok(DialogItemStackTemplateState {
        resource_id: fields.resource_id,
        count: fields.count,
        components: fields.components.cloned(),
        component_data,
        prepared_summary,
    })
}

fn project_item_stack_template(
    item: &ItemStackTemplateSummary,
    source: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogItemStackTemplateState, String> {
    let context = DialogDecodeContext {
        prepare_item_summary: false,
        ..context
    };
    let components = match source {
        NbtSummaryValue::Compound(fields) => find_entry(fields, "components"),
        NbtSummaryValue::String(_) => None,
        _ => return Err("item stack template must be a compound or string".to_string()),
    };
    let component_data = components
        .map(|components| {
            decode_data_component_patch_into(components, context, None, Some(&item.component_patch))
        })
        .transpose()?;
    let content_registry = context.registry_access.map_or(
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
        |registries| registries.content_registry(),
    );
    let resource_id = vanilla_item_type(item.item_id)
        .map(|entry| entry.resource_id)
        .or_else(|| {
            (!content_registry.is_vanilla_only())
                .then(|| {
                    content_registry
                        .item_by_id(item.item_id)
                        .map(|spec| spec.resource_id.as_str())
                })
                .flatten()
        })
        .ok_or_else(|| format!("unknown item id {}", item.item_id))?
        .to_string();
    Ok(DialogItemStackTemplateState {
        resource_id,
        count: item.count,
        components: components.cloned(),
        component_data,
        prepared_summary: None,
    })
}

pub(super) fn decode_data_component_patch(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogDataComponentPatchState, String> {
    decode_data_component_patch_into(
        value,
        DialogDecodeContext {
            prepare_item_summary: false,
            ..context
        },
        None,
        None,
    )
}

fn decode_data_component_patch_into(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    mut patch: Option<&mut DataComponentPatchSummary>,
    prepared: Option<&DataComponentPatchSummary>,
) -> Result<DialogDataComponentPatchState, String> {
    let entries = compound_entries(value, "dialog item stack template components")?;
    let mut decoded_entries: Vec<DialogDataComponentPatchEntryState> = Vec::new();
    for entry in entries {
        let (removed, raw_resource_id) = entry
            .name
            .strip_prefix('!')
            .map_or((false, entry.name.clone()), |resource_id| {
                (true, resource_id)
            });
        // A data component patch key is a resource location; that grammar owns the bound.
        let raw_resource_id = raw_resource_id
            .try_to_string()
            .map_err(|_| "invalid component resource id UTF-16".to_string())?;
        let resource_id = normalize_codec_identifier(&raw_resource_id).ok_or_else(|| {
            format!("invalid dialog item component resource id {raw_resource_id}")
        })?;
        let component_type = vanilla_data_component_type_for_resource_id(&resource_id)
            .ok_or_else(|| format!("unknown dialog item component {resource_id}"))?;
        if !component_type.persistent {
            return Err(format!(
                "dialog item component {resource_id} is not persistent"
            ));
        }
        if removed && !matches!(entry.value, NbtSummaryValue::Compound(_)) {
            return Err(format!(
                "removed dialog item component {resource_id} payload is not a compound"
            ));
        }
        let decoded = DialogDataComponentPatchEntryState {
            type_id: component_type.type_id,
            resource_id,
            removed,
            value: (!removed).then(|| entry.value.clone()),
            value_data: None,
        };
        if let Some(existing) = decoded_entries
            .iter_mut()
            .find(|existing| existing.type_id == decoded.type_id)
        {
            if existing.removed == decoded.removed {
                return Err(format!(
                    "duplicate dialog item component patch key {}",
                    decoded.resource_id
                ));
            }
            *existing = decoded;
        } else {
            decoded_entries.push(decoded);
        }
    }
    let mut data = DialogDataComponentPatchState {
        entries: decoded_entries,
    };
    if let Some(patch) = patch.as_deref_mut() {
        let effective = if entries.len() == data.entries.len() {
            std::borrow::Cow::Borrowed(value)
        } else {
            std::borrow::Cow::Owned(effective_component_patch(&data))
        };
        prepare_component_patch_summary_into(&effective, context.registry_access, patch)?;
    }
    for entry in &mut data.entries {
        if let Some(value) = entry.value.as_ref() {
            entry.value_data = decode_supported_component_value_with_patch(
                entry.type_id,
                &entry.resource_id,
                value,
                context,
                prepared.or(patch.as_deref()),
            )?;
        }
    }
    Ok(data)
}

fn effective_component_patch(data: &DialogDataComponentPatchState) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        data.entries
            .iter()
            .map(|entry| NbtSummaryEntry {
                name: if entry.removed {
                    format!("!{}", entry.resource_id).into()
                } else {
                    entry.resource_id.clone().into()
                },
                value: entry
                    .value
                    .clone()
                    .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new())),
            })
            .collect(),
    )
}

fn validate_range(value: i32, min: i32, max: i32, what: &str) -> Result<(), String> {
    if !(min..=max).contains(&value) {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_dialog_bodies(values: &[NbtSummaryValue]) -> Result<Vec<DialogBodyState>, String> {
        super::decode_dialog_bodies(
            values,
            DialogDecodeContext::new(Some(&RegistrySet::default()), 0),
        )
    }

    fn decode_data_component_patch(
        value: &NbtSummaryValue,
    ) -> Result<DialogDataComponentPatchState, String> {
        super::decode_data_component_patch(
            value,
            DialogDecodeContext::new(Some(&RegistrySet::default()), 0),
        )
    }

    #[test]
    fn decodes_plain_message_contents_and_width() {
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "minecraft:plain_message"),
            value_entry(
                "contents",
                compound(vec![string_entry("translate", "dialog.example.body")]),
            ),
            value_entry("width", NbtSummaryValue::Int(320)),
        ])])
        .unwrap();

        let DialogBodyState::PlainMessage { message } = &bodies[0] else {
            panic!("expected plain message body");
        };
        assert_eq!(message.contents.text, "dialog.example.body");
        assert_eq!(message.width, 320);

        for width in [0, 1025] {
            let error = decode_dialog_bodies(&[compound(vec![
                string_entry("type", "plain_message"),
                value_entry("contents", NbtSummaryValue::string("Message")),
                value_entry("width", NbtSummaryValue::Int(width)),
            ])])
            .unwrap_err();
            assert!(error.contains("outside 1..=1024"));
        }
    }

    #[test]
    fn decodes_item_body_template_description_defaults_and_flags() {
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "minecraft:item"),
            value_entry("item", NbtSummaryValue::string("diamond")),
            value_entry("description", NbtSummaryValue::string("A diamond")),
            value_entry("show_decorations", NbtSummaryValue::Byte(0)),
            value_entry("show_tooltip", NbtSummaryValue::Byte(0)),
            value_entry("width", NbtSummaryValue::Int(32)),
            value_entry("height", NbtSummaryValue::Int(48)),
        ])])
        .unwrap();

        let DialogBodyState::Item {
            item,
            description,
            show_decorations,
            show_tooltip,
            width,
            height,
        } = &bodies[0]
        else {
            panic!("expected item body");
        };
        assert_eq!(item.resource_id, "minecraft:diamond");
        assert_eq!(item.count, 1);
        assert_eq!(item.components, None);
        assert_eq!(item.component_data, None);
        assert_eq!(description.as_ref().unwrap().contents.text, "A diamond");
        assert_eq!(description.as_ref().unwrap().width, 200);
        assert!(!show_decorations);
        assert!(!show_tooltip);
        assert_eq!(*width, 32);
        assert_eq!(*height, 48);
    }

    #[test]
    fn decodes_compound_item_template_and_rejects_codec_ranges() {
        let components = compound(vec![value_entry(
            "minecraft:custom_name",
            NbtSummaryValue::string("Named"),
        )]);
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "item"),
            value_entry(
                "item",
                compound(vec![
                    string_entry("id", "minecraft:stone"),
                    value_entry("count", NbtSummaryValue::Int(2)),
                    value_entry("components", components.clone()),
                ]),
            ),
            value_entry(
                "description",
                compound(vec![
                    value_entry("contents", NbtSummaryValue::string("Stone")),
                    value_entry("width", NbtSummaryValue::Int(250)),
                ]),
            ),
        ])])
        .unwrap();
        let DialogBodyState::Item {
            item,
            description,
            show_decorations,
            show_tooltip,
            width,
            height,
        } = &bodies[0]
        else {
            panic!("expected item body");
        };
        assert_eq!(item.resource_id, "minecraft:stone");
        assert_eq!(item.count, 2);
        assert_eq!(item.components, Some(components));
        assert_eq!(
            item.component_data,
            Some(DialogDataComponentPatchState {
                entries: vec![DialogDataComponentPatchEntryState {
                    type_id: VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    resource_id: "minecraft:custom_name".to_string(),
                    removed: false,
                    value: Some(NbtSummaryValue::string("Named")),
                    value_data: Some(DialogDataComponentValueState::TextComponent {
                        component: DialogTextState {
                            text: "Named".to_string(),
                            runs: vec![bbb_protocol::StyledTextRun {
                                text: "Named".to_string(),
                                style: bbb_protocol::ComponentStyle::default(),
                            }],
                            component: NbtSummaryValue::string("Named"),
                        },
                    }),
                }],
            })
        );
        assert_eq!(description.as_ref().unwrap().width, 250);
        assert!(show_decorations);
        assert!(show_tooltip);
        assert_eq!(*width, 16);
        assert_eq!(*height, 16);
        let stone_id = vanilla_item_type_for_resource_id("minecraft:stone")
            .unwrap()
            .id;
        let summary = item.item_stack_summary_for_protocol_id(stone_id).unwrap();
        assert_eq!(summary.item_id, Some(stone_id));
        assert_eq!(summary.count, 2);
        assert_eq!(summary.component_patch.added_type_ids.len(), 1);
        assert_eq!(
            summary.component_patch.added_type_ids,
            vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID]
        );
        assert_eq!(
            summary.component_patch.custom_name.as_deref(),
            Some("Named")
        );
        assert_eq!(
            summary.component_patch.custom_name_nbt,
            Some(NbtSummaryValue::string("Named"))
        );

        for (field, value) in [("width", 0), ("height", 257)] {
            let error = decode_dialog_bodies(&[compound(vec![
                string_entry("type", "item"),
                value_entry("item", NbtSummaryValue::string("stone")),
                value_entry(field, NbtSummaryValue::Int(value)),
            ])])
            .unwrap_err();
            assert!(error.contains("outside 1..=256"));
        }
        for item in [
            NbtSummaryValue::string("minecraft:air"),
            NbtSummaryValue::string("example:missing"),
            compound(vec![
                string_entry("id", "minecraft:stone"),
                value_entry("count", NbtSummaryValue::Int(100)),
            ]),
        ] {
            assert!(decode_dialog_bodies(&[compound(vec![
                string_entry("type", "item"),
                value_entry("item", item),
            ])])
            .is_err());
        }
    }

    #[test]
    fn prepares_item_patch_from_persistent_values_for_runtime_and_control() {
        let components = compound(vec![
            value_entry("max_damage", NbtSummaryValue::Int(100)),
            value_entry("enchantment_glint_override", NbtSummaryValue::Byte(1)),
            value_entry("!lore", compound(Vec::new())),
            string_entry("tropical_fish/pattern", "flopper"),
            value_entry("lock", compound(Vec::new())),
        ]);
        let item = super::decode_item_stack_template(
            &compound(vec![
                string_entry("id", "stick"),
                value_entry("count", NbtSummaryValue::Int(3)),
                value_entry("components", components),
            ]),
            DialogDecodeContext::new(Some(&RegistrySet::default()), 0),
        )
        .unwrap();
        let id = vanilla_item_type_for_resource_id("minecraft:stick")
            .unwrap()
            .id;
        let summary = item.item_stack_summary_for_protocol_id(id).unwrap();
        assert_eq!(summary.component_patch.added_type_ids.len(), 4);
        assert_eq!(summary.component_patch.max_damage, Some(100));
        assert_eq!(
            summary.component_patch.enchantment_glint_override,
            Some(true)
        );
        assert_eq!(
            summary.component_patch.tropical_fish_pattern_id,
            Some(VANILLA_TROPICAL_FISH_PATTERN_FLOPPER_PACKED_ID)
        );
        assert_eq!(
            summary.component_patch.lock_predicate,
            Some(bbb_protocol::packets::PersistentItemPredicateSummary::default())
        );
        assert!(item.item_stack_summary_for_protocol_id(id + 1).is_none());
        let control = serde_json::to_value(&item).unwrap();
        assert_eq!(
            control["component_data"]["entries"][0]["value_data"],
            serde_json::json!({"codec": "integer", "value": 100})
        );
        assert!(control.get("prepared_summary").is_none());
        let restored: DialogItemStackTemplateState = serde_json::from_value(control).unwrap();
        assert_eq!(restored, item);
        assert!(restored.item_stack_summary_for_protocol_id(id).is_none());
        let repeated = item.item_stack_summary_for_protocol_id(id).unwrap();
        assert_eq!(repeated.as_ref(), summary.as_ref());
    }

    #[test]
    fn projects_adventure_partial_component_predicates_with_persistent_values() {
        let damage_predicate = compound(vec![value_entry(
            "durability",
            compound(vec![value_entry("min", NbtSummaryValue::Short(1))]),
        )]);
        let any_custom_name = compound(Vec::new());
        let decoded = decode_data_component_patch(&compound(vec![value_entry(
            "can_break",
            compound(vec![value_entry(
                "predicates",
                compound(vec![
                    value_entry("damage", damage_predicate.clone()),
                    value_entry("custom_name", any_custom_name.clone()),
                ]),
            )]),
        )]))
        .unwrap();
        let item = super::decode_item_stack_template(
            &compound(vec![
                string_entry("id", "stick"),
                value_entry("components", effective_component_patch(&decoded)),
            ]),
            DialogDecodeContext::new(Some(&RegistrySet::default()), 0),
        )
        .unwrap();

        let summary = item
            .item_stack_summary_for_protocol_id(
                vanilla_item_type_for_resource_id("minecraft:stick")
                    .unwrap()
                    .id,
            )
            .unwrap();
        let predicate = &summary
            .component_patch
            .can_break_persistent
            .as_ref()
            .unwrap()
            .predicates[0];
        assert_eq!(predicate.partial_components.len(), 2);
        assert_eq!(
            predicate.partial_components[0].predicate_type,
            bbb_protocol::packets::DataComponentPredicateTypeSummary::Concrete(0)
        );
        assert_eq!(
            predicate.partial_components[0].wire_value.as_ref(),
            Some(&damage_predicate)
        );
        assert_eq!(
            predicate.partial_components[1].predicate_type,
            bbb_protocol::packets::DataComponentPredicateTypeSummary::AnyComponent(6)
        );
        assert_eq!(
            predicate.partial_components[1].wire_value.as_ref(),
            Some(&any_custom_name)
        );
        assert!(
            bbb_protocol::packets::data_components::catalog::adventure_mode_predicate_tooltip_summary(
                summary.component_patch.can_break_persistent.as_ref().unwrap()
            )
            .unknown
        );
    }

    #[test]
    fn decodes_use_remainder_with_shared_recursive_item_stack_template_codec() {
        let compact = decode_data_component_patch(&compound(vec![value_entry(
            "use_remainder",
            NbtSummaryValue::string("glass_bottle"),
        )]))
        .unwrap();
        let Some(DialogDataComponentValueState::UseRemainder { convert_into }) =
            compact.entries[0].value_data.as_ref()
        else {
            panic!("expected compact use-remainder component");
        };
        assert_eq!(convert_into.resource_id, "minecraft:glass_bottle");
        assert_eq!(convert_into.count, 1);
        assert_eq!(convert_into.component_data, None);

        let nested_components = compound(vec![value_entry(
            "use_remainder",
            NbtSummaryValue::string("glass_bottle"),
        )]);
        let full_value = compound(vec![
            string_entry("id", "bucket"),
            value_entry("count", NbtSummaryValue::Int(2)),
            value_entry("components", nested_components.clone()),
        ]);
        let full = decode_data_component_patch(&compound(vec![value_entry(
            "minecraft:use_remainder",
            full_value,
        )]))
        .unwrap();
        let Some(DialogDataComponentValueState::UseRemainder { convert_into }) =
            full.entries[0].value_data.as_ref()
        else {
            panic!("expected full use-remainder component");
        };
        assert_eq!(convert_into.resource_id, "minecraft:bucket");
        assert_eq!(convert_into.count, 2);
        assert_eq!(convert_into.components.as_ref(), Some(&nested_components));
        let nested = convert_into.component_data.as_ref().unwrap();
        let Some(DialogDataComponentValueState::UseRemainder { convert_into }) =
            nested.entries[0].value_data.as_ref()
        else {
            panic!("expected nested use-remainder component");
        };
        assert_eq!(convert_into.resource_id, "minecraft:glass_bottle");

        let patch = prepare_component_patch_summary(
            &effective_component_patch(&full),
            Some(RegistrySet::default().frozen_access()),
        )
        .unwrap();
        let remainder = patch.use_remainder.as_ref().unwrap();
        assert_eq!(
            remainder.item_id,
            vanilla_item_type_for_resource_id("minecraft:bucket")
                .unwrap()
                .id
        );
        assert_eq!(remainder.count, 2);
        let nested = remainder.component_patch.use_remainder.as_ref().unwrap();
        assert_eq!(
            nested.item_id,
            vanilla_item_type_for_resource_id("minecraft:glass_bottle")
                .unwrap()
                .id
        );
    }

    #[test]
    fn rejects_invalid_use_remainder_item_stack_templates() {
        for value in [
            NbtSummaryValue::List(Vec::new()),
            NbtSummaryValue::string("air"),
            compound(Vec::new()),
            compound(vec![string_entry("id", "missing")]),
            compound(vec![
                string_entry("id", "bucket"),
                value_entry("count", NbtSummaryValue::Int(0)),
            ]),
            compound(vec![
                string_entry("id", "bucket"),
                value_entry("count", NbtSummaryValue::Int(100)),
            ]),
            compound(vec![
                string_entry("id", "bucket"),
                value_entry("components", NbtSummaryValue::Int(0)),
            ]),
        ] {
            assert!(decode_data_component_patch(&compound(vec![value_entry(
                "use_remainder",
                value
            )]))
            .is_err());
        }
    }

    #[test]
    fn decodes_component_patch_removals_and_applies_last_entry() {
        let components = compound(vec![
            value_entry("custom_name", NbtSummaryValue::string("First")),
            value_entry("!minecraft:custom_name", compound(Vec::new())),
            value_entry("minecraft:repair_cost", NbtSummaryValue::Int(3)),
        ]);
        let bodies = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "item"),
            value_entry(
                "item",
                compound(vec![
                    string_entry("id", "stone"),
                    value_entry("components", components.clone()),
                ]),
            ),
        ])])
        .unwrap();
        let DialogBodyState::Item { item, .. } = &bodies[0] else {
            panic!("expected item body");
        };

        assert_eq!(item.components, Some(components));
        assert_eq!(item.component_data.as_ref().unwrap().entries.len(), 2);
        assert_eq!(
            item.component_data.as_ref().unwrap().entries[0],
            DialogDataComponentPatchEntryState {
                type_id: VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                resource_id: "minecraft:custom_name".to_string(),
                removed: true,
                value: None,
                value_data: None,
            }
        );
        assert_eq!(
            item.component_data.as_ref().unwrap().entries[1],
            DialogDataComponentPatchEntryState {
                type_id: VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
                resource_id: "minecraft:repair_cost".to_string(),
                removed: false,
                value: Some(NbtSummaryValue::Int(3)),
                value_data: Some(DialogDataComponentValueState::Integer { value: 3 }),
            }
        );
    }

    #[test]
    fn rejects_unknown_transient_and_malformed_component_patch_entries() {
        for (name, value, expected) in [
            (
                "minecraft:missing",
                compound(Vec::new()),
                "unknown dialog item component",
            ),
            (
                "minecraft:creative_slot_lock",
                compound(Vec::new()),
                "is not persistent",
            ),
            (
                "!minecraft:custom_name",
                NbtSummaryValue::string("not a map"),
                "payload is not a compound",
            ),
        ] {
            let error = decode_dialog_bodies(&[compound(vec![
                string_entry("type", "item"),
                value_entry(
                    "item",
                    compound(vec![
                        string_entry("id", "stone"),
                        value_entry("components", compound(vec![value_entry(name, value)])),
                    ]),
                ),
            ])])
            .unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }

        let error = decode_dialog_bodies(&[compound(vec![
            string_entry("type", "item"),
            value_entry(
                "item",
                compound(vec![
                    string_entry("id", "stone"),
                    value_entry(
                        "components",
                        compound(vec![
                            value_entry("custom_name", NbtSummaryValue::string("First")),
                            value_entry("minecraft:custom_name", NbtSummaryValue::string("Second")),
                        ]),
                    ),
                ]),
            ),
        ])])
        .unwrap_err();
        assert!(error.contains("duplicate dialog item component patch key"));
    }

    #[test]
    fn decodes_primitive_component_value_codecs() {
        let patch = decode_data_component_patch(&compound(vec![
            value_entry("max_stack_size", NbtSummaryValue::Byte(99)),
            value_entry("max_damage", NbtSummaryValue::Long(10)),
            value_entry("damage", NbtSummaryValue::Double(3.75f64.to_bits())),
            value_entry("unbreakable", compound(Vec::new())),
            value_entry(
                "custom_name",
                compound(vec![string_entry("translate", "item.example")]),
            ),
            value_entry(
                "minimum_attack_charge",
                NbtSummaryValue::Double(0.5f64.to_bits()),
            ),
            value_entry("item_name", NbtSummaryValue::string("Item")),
            value_entry("item_model", NbtSummaryValue::string("stone")),
            value_entry("rarity", NbtSummaryValue::string("rare")),
            value_entry("repair_cost", NbtSummaryValue::Short(7)),
            value_entry("enchantment_glint_override", NbtSummaryValue::Int(2)),
            value_entry("intangible_projectile", compound(Vec::new())),
            value_entry("glider", compound(Vec::new())),
            value_entry("tooltip_style", NbtSummaryValue::string(":compact")),
            value_entry(
                "potion_duration_scale",
                NbtSummaryValue::Double(1.25f64.to_bits()),
            ),
            value_entry(
                "note_block_sound",
                NbtSummaryValue::string("block.note_block.harp"),
            ),
            value_entry("lore", NbtSummaryValue::List(Vec::new())),
        ]))
        .unwrap();
        let value = |resource_id: &str| {
            patch
                .entries
                .iter()
                .find(|entry| entry.resource_id == resource_id)
                .unwrap()
                .value_data
                .as_ref()
        };

        assert_eq!(
            value("minecraft:max_stack_size"),
            Some(&DialogDataComponentValueState::Integer { value: 99 })
        );
        assert_eq!(
            value("minecraft:max_damage"),
            Some(&DialogDataComponentValueState::Integer { value: 10 })
        );
        assert_eq!(
            value("minecraft:damage"),
            Some(&DialogDataComponentValueState::Integer { value: 3 })
        );
        assert_eq!(
            value("minecraft:repair_cost"),
            Some(&DialogDataComponentValueState::Integer { value: 7 })
        );
        assert_eq!(
            value("minecraft:unbreakable"),
            Some(&DialogDataComponentValueState::Unit)
        );
        assert_eq!(
            value("minecraft:intangible_projectile"),
            Some(&DialogDataComponentValueState::Unit)
        );
        assert_eq!(
            value("minecraft:glider"),
            Some(&DialogDataComponentValueState::Unit)
        );
        assert_eq!(
            value("minecraft:enchantment_glint_override"),
            Some(&DialogDataComponentValueState::Boolean { value: true })
        );
        assert_eq!(
            value("minecraft:minimum_attack_charge")
                .unwrap()
                .float_value(),
            Some(0.5)
        );
        assert_eq!(
            value("minecraft:potion_duration_scale")
                .unwrap()
                .float_value(),
            Some(1.25)
        );
        assert_eq!(
            value("minecraft:item_model"),
            Some(&DialogDataComponentValueState::ResourceId {
                resource_id: "minecraft:stone".to_string(),
            })
        );
        assert_eq!(
            value("minecraft:tooltip_style"),
            Some(&DialogDataComponentValueState::ResourceId {
                resource_id: "minecraft:compact".to_string(),
            })
        );
        assert_eq!(
            value("minecraft:note_block_sound"),
            Some(&DialogDataComponentValueState::ResourceId {
                resource_id: "minecraft:block.note_block.harp".to_string(),
            })
        );
        assert_eq!(
            value("minecraft:rarity"),
            Some(&DialogDataComponentValueState::Rarity {
                value: ItemRaritySummary::Rare,
            })
        );
        assert!(matches!(
            value("minecraft:custom_name"),
            Some(DialogDataComponentValueState::TextComponent { component })
                if component.text == "item.example"
        ));
        assert!(matches!(
            value("minecraft:item_name"),
            Some(DialogDataComponentValueState::TextComponent { component })
                if component.text == "Item"
        ));
        assert!(matches!(
            value("minecraft:lore"),
            Some(DialogDataComponentValueState::Lore { lines }) if lines.is_empty()
        ));
    }

    #[test]
    fn rejects_invalid_primitive_component_value_codecs() {
        for (name, value) in [
            ("max_stack_size", NbtSummaryValue::Int(0)),
            ("max_damage", NbtSummaryValue::Int(0)),
            ("damage", NbtSummaryValue::Int(-1)),
            (
                "minimum_attack_charge",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
            ),
            (
                "potion_duration_scale",
                NbtSummaryValue::Float(f32::INFINITY.to_bits()),
            ),
            (
                "enchantment_glint_override",
                NbtSummaryValue::string("true"),
            ),
            ("unbreakable", NbtSummaryValue::Byte(1)),
            ("item_model", NbtSummaryValue::string("Invalid")),
            ("rarity", NbtSummaryValue::string("legendary")),
        ] {
            let _ =
                decode_data_component_patch(&compound(vec![value_entry(name, value)])).unwrap_err();
        }
    }

    #[test]
    fn decodes_all_string_enum_component_value_codecs() {
        let expected = [
            ("dye", "red", 14),
            ("base_color", "blue", 11),
            ("wolf/collar", "lime", 5),
            ("fox/variant", "snow", 1),
            ("salmon/size", "large", 2),
            ("parrot/variant", "gray", 4),
            ("tropical_fish/pattern", "spotty", 1280),
            ("tropical_fish/base_color", "cyan", 9),
            ("tropical_fish/pattern_color", "purple", 10),
            ("mooshroom/variant", "brown", 1),
            ("rabbit/variant", "evil", 99),
            ("horse/variant", "dark_brown", 6),
            ("llama/variant", "gray", 3),
            ("axolotl/variant", "blue", 4),
            ("cat/collar", "orange", 1),
            ("sheep/color", "white", 0),
            ("shulker/color", "black", 15),
        ];
        let patch = decode_data_component_patch(&compound(
            expected
                .iter()
                .map(|(component, value, _)| {
                    value_entry(component, NbtSummaryValue::string(*value))
                })
                .collect(),
        ))
        .unwrap();

        assert_eq!(patch.entries.len(), expected.len());
        for ((_, serialized_name, value_id), entry) in expected.iter().zip(&patch.entries) {
            assert_eq!(
                entry.value_data,
                Some(DialogDataComponentValueState::StringEnum {
                    value_id: *value_id,
                    serialized_name: (*serialized_name).to_string(),
                })
            );
        }
    }

    #[test]
    fn rejects_invalid_string_enum_component_value_codecs() {
        for (name, value, expected) in [
            (
                "dye",
                NbtSummaryValue::string("scarlet"),
                "unknown dialog item component minecraft:dye enum value scarlet",
            ),
            (
                "fox/variant",
                NbtSummaryValue::Int(1),
                "is not an enum name",
            ),
            (
                "tropical_fish/pattern",
                NbtSummaryValue::string("missing"),
                "unknown dialog item component minecraft:tropical_fish/pattern enum value missing",
            ),
        ] {
            let error =
                decode_data_component_patch(&compound(vec![value_entry(name, value)])).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
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
