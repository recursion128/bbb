use bbb_protocol::{
    item_types::vanilla_item_type_for_resource_id, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::{
    compound_entries, decode_component, find_entry, optional_bool, optional_i32, required_entry,
    required_string, DialogTextState,
};

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
}

pub(super) fn decode_dialog_bodies(
    values: &[NbtSummaryValue],
) -> Result<Vec<DialogBodyState>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| decode_dialog_body(value, index))
        .collect()
}

fn decode_dialog_body(value: &NbtSummaryValue, index: usize) -> Result<DialogBodyState, String> {
    let what = format!("dialog body[{index}]");
    let entries = compound_entries(value, &what)?;
    match required_string(entries, "type", &what)? {
        "plain_message" | "minecraft:plain_message" => Ok(DialogBodyState::PlainMessage {
            message: decode_plain_message_map(entries, &what)?,
        }),
        "item" | "minecraft:item" => {
            let width = optional_i32(entries, "width", 16, "item dialog body")?;
            validate_range(width, 1, 256, "item dialog body width")?;
            let height = optional_i32(entries, "height", 16, "item dialog body")?;
            validate_range(height, 1, 256, "item dialog body height")?;
            Ok(DialogBodyState::Item {
                item: decode_item_stack_template(required_entry(entries, "item", &what)?)?,
                description: find_entry(entries, "description")
                    .map(decode_plain_message)
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

fn decode_plain_message(value: &NbtSummaryValue) -> Result<DialogPlainMessageState, String> {
    if let NbtSummaryValue::Compound(entries) = value {
        if find_entry(entries, "contents").is_some() {
            return decode_plain_message_map(entries, "dialog plain message");
        }
    }
    Ok(DialogPlainMessageState {
        contents: decode_component(value, "dialog plain message contents")?,
        width: 200,
    })
}

fn decode_plain_message_map(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogPlainMessageState, String> {
    let width = optional_i32(entries, "width", 200, what)?;
    validate_range(width, 1, 1024, &format!("{what} width"))?;
    Ok(DialogPlainMessageState {
        contents: decode_component(
            required_entry(entries, "contents", what)?,
            "dialog plain message contents",
        )?,
        width,
    })
}

fn decode_item_stack_template(
    value: &NbtSummaryValue,
) -> Result<DialogItemStackTemplateState, String> {
    match value {
        NbtSummaryValue::String(resource_id) => Ok(DialogItemStackTemplateState {
            resource_id: decode_item_resource_id(resource_id)?,
            count: 1,
            components: None,
        }),
        NbtSummaryValue::Compound(entries) => {
            let resource_id = decode_item_resource_id(required_string(
                entries,
                "id",
                "dialog item stack template",
            )?)?;
            let count = optional_i32(entries, "count", 1, "dialog item stack template")?;
            validate_range(count, 1, 99, "dialog item stack template count")?;
            let components = find_entry(entries, "components")
                .map(|components| {
                    compound_entries(components, "dialog item stack template components")?;
                    Ok::<_, String>(components.clone())
                })
                .transpose()?;
            Ok(DialogItemStackTemplateState {
                resource_id,
                count,
                components,
            })
        }
        _ => Err("dialog item stack template is neither an item id nor a compound".to_string()),
    }
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
        assert_eq!(description.as_ref().unwrap().width, 250);
        assert!(show_decorations);
        assert!(show_tooltip);
        assert_eq!(*width, 16);
        assert_eq!(*height, 16);

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
