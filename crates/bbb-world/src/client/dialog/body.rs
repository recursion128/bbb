#[cfg(test)]
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
};
#[cfg(test)]
use bbb_protocol::item_component_value_types::ItemRaritySummary;
use bbb_protocol::packets::{
    validate_data_component_patch_summary, DataComponentPatchSummary, ItemStackSummary,
};
#[cfg(test)]
use bbb_protocol::tropical_fish_patterns::VANILLA_TROPICAL_FISH_PATTERN_FLOPPER_PACKED_ID;
use bbb_protocol::{
    data_component_types::{
        vanilla_data_component_type, vanilla_data_component_type_for_resource_id,
    },
    item_types::vanilla_item_type_for_resource_id,
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{registries::normalize_codec_identifier, FrozenRegistryAccess, RegistrySet};

use super::{
    compound_entries, decode_component, find_entry, optional_bool, optional_i32, required_entry,
    required_string, DialogDecodeContext, DialogTextState,
};

mod component_value;

#[cfg(test)]
use component_value::apply_item_patch_value;
pub use component_value::DialogDataComponentValueState;
use component_value::{
    apply_item_patch_entry_value, decode_supported_component_value_with_context,
    validate_component_player_profile as validate_player_profile,
};
pub(super) use component_value::{
    canonical_component_player_profile_json_utf16_len,
    canonical_data_component_patch_json_utf16_len_with_context,
    canonical_item_stack_template_json_utf16_len_with_context,
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
    decode_supported_component_value_with_context(
        type_id,
        resource_id,
        value,
        DialogDecodeContext::with_registry_access(Some(registry_access), 0),
    )
    .map(|decoded| decoded.is_some())
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

pub(crate) fn decode_persistent_data_component_patch_entry(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: &RegistrySet,
) -> Result<Option<DataComponentPatchSummary>, String> {
    let component_type = vanilla_data_component_type(type_id)
        .ok_or_else(|| format!("unknown persistent data component type id {type_id}"))?;
    if !component_type.persistent {
        return Err(format!(
            "data component {} is not persistent",
            component_type.resource_id
        ));
    }
    let Some(decoded) = decode_supported_component_value_with_context(
        type_id,
        component_type.resource_id,
        value,
        DialogDecodeContext::new(Some(registries), 0),
    )?
    else {
        return Ok(None);
    };

    let mut patch = DataComponentPatchSummary::default();
    if !apply_item_patch_entry_value(type_id, &decoded, Some(value), &mut patch) {
        return Ok(None);
    }
    patch.added = 1;
    patch.added_type_ids.push(type_id);
    validate_data_component_patch_summary(&patch).map_err(|error| error.to_string())?;
    Ok(Some(patch))
}

pub(crate) fn decode_persistent_data_component_patch_summary(
    value: &NbtSummaryValue,
    registries: &RegistrySet,
) -> Result<Option<DataComponentPatchSummary>, String> {
    let decoded =
        decode_data_component_patch(value, DialogDecodeContext::new(Some(registries), 0))?;
    Ok(decoded.data_component_patch_summary())
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogItemStackTemplateState {
    pub resource_id: String,
    pub count: i32,
    #[serde(default)]
    pub components: Option<NbtSummaryValue>,
    #[serde(default)]
    pub component_data: Option<DialogDataComponentPatchState>,
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

impl DialogItemStackTemplateState {
    pub fn item_stack_summary_for_protocol_id(&self, item_id: i32) -> Option<ItemStackSummary> {
        let component_patch = match &self.component_data {
            Some(patch) => patch.data_component_patch_summary()?,
            None => DataComponentPatchSummary::default(),
        };
        Some(ItemStackSummary {
            item_id: Some(item_id),
            count: self.count,
            component_patch,
        })
    }
}

impl DialogDataComponentPatchState {
    fn data_component_patch_summary(&self) -> Option<DataComponentPatchSummary> {
        let mut component_patch = DataComponentPatchSummary::default();
        for entry in &self.entries {
            if entry.removed {
                component_patch.removed_type_ids.push(entry.type_id);
                continue;
            }
            apply_item_patch_entry_value(
                entry.type_id,
                entry.value_data.as_ref()?,
                entry.value.as_ref(),
                &mut component_patch,
            )
            .then_some(())?;
            component_patch.added_type_ids.push(entry.type_id);
        }
        component_patch.added = component_patch.added_type_ids.len();
        validate_data_component_patch_summary(&component_patch).ok()?;
        Some(component_patch)
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
    match required_string(entries, "type", &what)? {
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
    match value {
        NbtSummaryValue::String(resource_id) => Ok(DialogItemStackTemplateState {
            resource_id: decode_item_resource_id(resource_id)?,
            count: 1,
            components: None,
            component_data: None,
        }),
        NbtSummaryValue::Compound(entries) => {
            let resource_id = decode_item_resource_id(required_string(
                entries,
                "id",
                "dialog item stack template",
            )?)?;
            let count = optional_i32(entries, "count", 1, "dialog item stack template")?;
            validate_range(count, 1, 99, "dialog item stack template count")?;
            let (components, component_data) = find_entry(entries, "components")
                .map(|components| {
                    Ok::<_, String>((
                        components.clone(),
                        decode_data_component_patch(components, context)?,
                    ))
                })
                .transpose()?
                .map_or((None, None), |(components, component_data)| {
                    (Some(components), Some(component_data))
                });
            Ok(DialogItemStackTemplateState {
                resource_id,
                count,
                components,
                component_data,
            })
        }
        _ => Err("dialog item stack template is neither an item id nor a compound".to_string()),
    }
}

pub(super) fn decode_data_component_patch(
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<DialogDataComponentPatchState, String> {
    let entries = compound_entries(value, "dialog item stack template components")?;
    let mut decoded_entries: Vec<DialogDataComponentPatchEntryState> = Vec::new();
    for entry in entries {
        let (removed, raw_resource_id) = entry
            .name
            .strip_prefix('!')
            .map_or((false, entry.name.as_str()), |resource_id| {
                (true, resource_id)
            });
        let resource_id = normalize_codec_identifier(raw_resource_id).ok_or_else(|| {
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
        let value_data = if removed {
            None
        } else {
            decode_supported_component_value_with_context(
                component_type.type_id,
                &resource_id,
                &entry.value,
                context,
            )?
        };
        let decoded = DialogDataComponentPatchEntryState {
            type_id: component_type.type_id,
            resource_id,
            removed,
            value: (!removed).then(|| entry.value.clone()),
            value_data,
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
    Ok(DialogDataComponentPatchState {
        entries: decoded_entries,
    })
}

fn decode_item_resource_id(raw: &str) -> Result<String, String> {
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid dialog item resource id {raw}"))?;
    if resource_id == "minecraft:air" {
        return Err("dialog item must not be minecraft:air".to_string());
    }
    vanilla_item_type_for_resource_id(&resource_id)
        .ok_or_else(|| format!("unknown dialog item {resource_id}"))?;
    Ok(resource_id)
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
                value_entry("contents", NbtSummaryValue::String("Message".to_string())),
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
            value_entry("item", NbtSummaryValue::String("diamond".to_string())),
            value_entry(
                "description",
                NbtSummaryValue::String("A diamond".to_string()),
            ),
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
            NbtSummaryValue::String("Named".to_string()),
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
                    value_entry("contents", NbtSummaryValue::String("Stone".to_string())),
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
                    value: Some(NbtSummaryValue::String("Named".to_string())),
                    value_data: Some(DialogDataComponentValueState::TextComponent {
                        component: DialogTextState {
                            text: "Named".to_string(),
                            runs: vec![bbb_protocol::StyledTextRun {
                                text: "Named".to_string(),
                                style: bbb_protocol::ComponentStyle::default(),
                            }],
                            component: NbtSummaryValue::String("Named".to_string()),
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
        let summary = item.item_stack_summary_for_protocol_id(7).unwrap();
        assert_eq!(summary.item_id, Some(7));
        assert_eq!(summary.count, 2);
        assert_eq!(summary.component_patch.added, 1);
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
            Some(NbtSummaryValue::String("Named".to_string()))
        );

        for (field, value) in [("width", 0), ("height", 257)] {
            let error = decode_dialog_bodies(&[compound(vec![
                string_entry("type", "item"),
                value_entry("item", NbtSummaryValue::String("stone".to_string())),
                value_entry(field, NbtSummaryValue::Int(value)),
            ])])
            .unwrap_err();
            assert!(error.contains("outside 1..=256"));
        }
        for item in [
            NbtSummaryValue::String("minecraft:air".to_string()),
            NbtSummaryValue::String("example:missing".to_string()),
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
    fn projects_item_patch_values_and_rejects_unbridged_values() {
        let type_id = |resource_id| {
            vanilla_data_component_type_for_resource_id(resource_id)
                .unwrap()
                .type_id
        };
        let max_damage_type_id = type_id("minecraft:max_damage");
        let glint_type_id = type_id("minecraft:enchantment_glint_override");
        let lore_type_id = type_id("minecraft:lore");
        let tropical_fish_pattern_type_id = type_id("minecraft:tropical_fish/pattern");
        let item = DialogItemStackTemplateState {
            resource_id: "minecraft:stick".to_string(),
            count: 3,
            components: None,
            component_data: Some(DialogDataComponentPatchState {
                entries: vec![
                    DialogDataComponentPatchEntryState {
                        type_id: max_damage_type_id,
                        resource_id: "minecraft:max_damage".to_string(),
                        removed: false,
                        value: None,
                        value_data: Some(DialogDataComponentValueState::Integer { value: 100 }),
                    },
                    DialogDataComponentPatchEntryState {
                        type_id: glint_type_id,
                        resource_id: "minecraft:enchantment_glint_override".to_string(),
                        removed: false,
                        value: None,
                        value_data: Some(DialogDataComponentValueState::Boolean { value: true }),
                    },
                    DialogDataComponentPatchEntryState {
                        type_id: lore_type_id,
                        resource_id: "minecraft:lore".to_string(),
                        removed: true,
                        value: None,
                        value_data: None,
                    },
                    DialogDataComponentPatchEntryState {
                        type_id: tropical_fish_pattern_type_id,
                        resource_id: "minecraft:tropical_fish/pattern".to_string(),
                        removed: false,
                        value: None,
                        value_data: Some(DialogDataComponentValueState::StringEnum {
                            value_id: VANILLA_TROPICAL_FISH_PATTERN_FLOPPER_PACKED_ID,
                            serialized_name: "flopper".to_string(),
                        }),
                    },
                ],
            }),
        };

        let summary = item.item_stack_summary_for_protocol_id(4).unwrap();
        assert_eq!(summary.component_patch.added, 3);
        assert_eq!(
            summary.component_patch.added_type_ids,
            vec![
                max_damage_type_id,
                glint_type_id,
                tropical_fish_pattern_type_id
            ]
        );
        assert_eq!(summary.component_patch.removed_type_ids, vec![lore_type_id]);
        assert_eq!(summary.component_patch.max_damage, Some(100));
        assert_eq!(
            summary.component_patch.enchantment_glint_override,
            Some(true)
        );
        assert_eq!(
            summary.component_patch.tropical_fish_pattern_id,
            Some(VANILLA_TROPICAL_FISH_PATTERN_FLOPPER_PACKED_ID)
        );

        let mut lock_patch =
            decode_data_component_patch(&compound(vec![value_entry("lock", compound(Vec::new()))]))
                .unwrap();
        let lock_entry = lock_patch.entries.remove(0);
        let mut lock_bridged = item.clone();
        lock_bridged
            .component_data
            .as_mut()
            .unwrap()
            .entries
            .push(lock_entry.clone());
        let lock_summary = lock_bridged.item_stack_summary_for_protocol_id(4).unwrap();
        assert_eq!(
            lock_summary.component_patch.lock_predicate,
            Some(bbb_protocol::packets::PersistentItemPredicateSummary::default())
        );
        assert!(!lock_summary
            .component_patch
            .codec_backed_nbt
            .contains_key(&type_id("minecraft:lock")));

        let mut unbridged = item;
        let mut rawless_lock = lock_entry;
        rawless_lock.value = None;
        unbridged
            .component_data
            .as_mut()
            .unwrap()
            .entries
            .push(rawless_lock);
        assert!(unbridged.item_stack_summary_for_protocol_id(4).is_none());
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
        let item = DialogItemStackTemplateState {
            resource_id: "minecraft:stick".to_string(),
            count: 1,
            components: None,
            component_data: Some(decoded),
        };

        let summary = item.item_stack_summary_for_protocol_id(4).unwrap();
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
        assert!(summary.component_patch.can_break.as_ref().unwrap().unknown);
    }

    #[test]
    fn decodes_use_remainder_with_shared_recursive_item_stack_template_codec() {
        let compact = decode_data_component_patch(&compound(vec![value_entry(
            "use_remainder",
            NbtSummaryValue::String("glass_bottle".to_string()),
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
            NbtSummaryValue::String("glass_bottle".to_string()),
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

        let mut patch = DataComponentPatchSummary::default();
        assert!(apply_item_patch_value(
            full.entries[0].type_id,
            full.entries[0].value_data.as_ref().unwrap(),
            &mut patch,
        ));
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
        for (value, expected) in [
            (
                NbtSummaryValue::List(Vec::new()),
                "is neither an item id nor a compound",
            ),
            (
                NbtSummaryValue::String("air".to_string()),
                "must not be minecraft:air",
            ),
            (compound(Vec::new()), "is missing field id"),
            (
                compound(vec![string_entry("id", "missing")]),
                "unknown dialog item minecraft:missing",
            ),
            (
                compound(vec![
                    string_entry("id", "bucket"),
                    value_entry("count", NbtSummaryValue::Int(0)),
                ]),
                "count 0 is outside 1..=99",
            ),
            (
                compound(vec![
                    string_entry("id", "bucket"),
                    value_entry("count", NbtSummaryValue::Int(100)),
                ]),
                "count 100 is outside 1..=99",
            ),
            (
                compound(vec![
                    string_entry("id", "bucket"),
                    value_entry("components", NbtSummaryValue::Int(0)),
                ]),
                "components is not a compound",
            ),
        ] {
            let error =
                decode_data_component_patch(&compound(vec![value_entry("use_remainder", value)]))
                    .unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    #[test]
    fn decodes_component_patch_removals_and_applies_last_entry() {
        let components = compound(vec![
            value_entry("custom_name", NbtSummaryValue::String("First".to_string())),
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
                NbtSummaryValue::String("not a map".to_string()),
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
                            value_entry(
                                "custom_name",
                                NbtSummaryValue::String("First".to_string()),
                            ),
                            value_entry(
                                "minecraft:custom_name",
                                NbtSummaryValue::String("Second".to_string()),
                            ),
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
            value_entry("item_name", NbtSummaryValue::String("Item".to_string())),
            value_entry("item_model", NbtSummaryValue::String("stone".to_string())),
            value_entry("rarity", NbtSummaryValue::String("rare".to_string())),
            value_entry("repair_cost", NbtSummaryValue::Short(7)),
            value_entry("enchantment_glint_override", NbtSummaryValue::Int(2)),
            value_entry("intangible_projectile", compound(Vec::new())),
            value_entry("glider", compound(Vec::new())),
            value_entry(
                "tooltip_style",
                NbtSummaryValue::String(":compact".to_string()),
            ),
            value_entry(
                "potion_duration_scale",
                NbtSummaryValue::Double(1.25f64.to_bits()),
            ),
            value_entry(
                "note_block_sound",
                NbtSummaryValue::String("block.note_block.harp".to_string()),
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
    fn canonicalizes_scalar_and_removed_component_patch_values() {
        let patch = decode_data_component_patch(&compound(vec![
            value_entry("max_stack_size", NbtSummaryValue::Byte(64)),
            value_entry(
                "minimum_attack_charge",
                NbtSummaryValue::Float(0.5f32.to_bits()),
            ),
            value_entry("enchantment_glint_override", NbtSummaryValue::Byte(0)),
            value_entry(
                "unbreakable",
                compound(vec![string_entry("ignored", "by unit codec")]),
            ),
            value_entry("item_model", NbtSummaryValue::String("stone".to_string())),
            value_entry("rarity", NbtSummaryValue::String("epic".to_string())),
            value_entry("fox/variant", NbtSummaryValue::String("snow".to_string())),
            value_entry("dyed_color", NbtSummaryValue::Int(0x12_34_56)),
            value_entry("!damage", compound(Vec::new())),
        ]))
        .unwrap();
        assert_eq!(
            component_value::canonical_data_component_patch_json_utf16_len(&patch),
            Ok(Some(
                concat!(
                    r#"{"minecraft:max_stack_size":64,"minecraft:minimum_attack_charge":0.5,"minecraft:enchantment_glint_override":false,"minecraft:unbreakable":{},"minecraft:item_model":"minecraft:stone","minecraft:rarity":"epic","minecraft:fox/variant":"snow","minecraft:dyed_color":1193046,"!minecraft:damage":{}}"#
                )
                .len()
            ))
        );

        let text = decode_data_component_patch(&compound(vec![value_entry(
            "custom_name",
            NbtSummaryValue::String("Name".to_string()),
        )]))
        .unwrap();
        assert_eq!(
            component_value::canonical_data_component_patch_json_utf16_len(&text),
            Ok(Some(r#"{"minecraft:custom_name":"Name"}"#.len()))
        );

        let empty = decode_data_component_patch(&compound(Vec::new())).unwrap();
        assert_eq!(
            component_value::canonical_data_component_patch_json_utf16_len(&empty),
            Ok(Some(2))
        );
    }

    #[test]
    fn rejects_invalid_primitive_component_value_codecs() {
        for (name, value, expected) in [
            ("max_stack_size", NbtSummaryValue::Int(0), "outside 1..=99"),
            ("max_damage", NbtSummaryValue::Int(0), "outside 1..="),
            ("damage", NbtSummaryValue::Int(-1), "outside 0..="),
            (
                "minimum_attack_charge",
                NbtSummaryValue::Float((-0.0f32).to_bits()),
                "outside 0..=1",
            ),
            (
                "potion_duration_scale",
                NbtSummaryValue::Float(f32::INFINITY.to_bits()),
                "outside 0..=",
            ),
            (
                "enchantment_glint_override",
                NbtSummaryValue::String("true".to_string()),
                "is not a boolean",
            ),
            ("unbreakable", NbtSummaryValue::Byte(1), "is not a compound"),
            (
                "item_model",
                NbtSummaryValue::String("Invalid".to_string()),
                "invalid dialog item component",
            ),
            (
                "rarity",
                NbtSummaryValue::String("legendary".to_string()),
                "unknown dialog item component",
            ),
        ] {
            let error =
                decode_data_component_patch(&compound(vec![value_entry(name, value)])).unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
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
                    value_entry(component, NbtSummaryValue::String((*value).to_string()))
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
                NbtSummaryValue::String("scarlet".to_string()),
                "unknown dialog item component minecraft:dye enum value scarlet",
            ),
            (
                "fox/variant",
                NbtSummaryValue::Int(1),
                "is not an enum name",
            ),
            (
                "tropical_fish/pattern",
                NbtSummaryValue::String("missing".to_string()),
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
            name: name.to_string(),
            value,
        }
    }

    fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
        value_entry(name, NbtSummaryValue::String(value.to_string()))
    }
}
