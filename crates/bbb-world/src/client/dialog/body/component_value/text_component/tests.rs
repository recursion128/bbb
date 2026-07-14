use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
};
use bbb_protocol::{ComponentStyle, NbtSummaryEntry, NbtSummaryValue, StyledTextRun};

use super::*;
use crate::client::dialog::body::component_value::{
    decode_supported_component_value, DialogDataComponentValueState,
};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn decode(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
) -> DialogDataComponentValueState {
    decode_supported_component_value(type_id, resource_id, value, None)
        .unwrap()
        .unwrap()
}

#[test]
fn decodes_custom_and_item_name_component_alternatives() {
    assert_eq!(
        decode(
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            "minecraft:custom_name",
            &NbtSummaryValue::String("Named".to_string()),
        ),
        DialogDataComponentValueState::TextComponent {
            component: DialogTextState {
                text: "Named".to_string(),
                runs: vec![StyledTextRun {
                    text: "Named".to_string(),
                    style: ComponentStyle::default(),
                }],
                component: NbtSummaryValue::String("Named".to_string()),
            },
        }
    );

    let styled = compound(vec![
        entry("text", NbtSummaryValue::String("Sword".to_string())),
        entry("bold", NbtSummaryValue::Byte(1)),
        entry("color", NbtSummaryValue::String("dark_purple".to_string())),
    ]);
    let DialogDataComponentValueState::TextComponent { component } = decode(
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        "minecraft:item_name",
        &styled,
    ) else {
        panic!("expected text component");
    };
    assert_eq!(component.text, "Sword");
    assert_eq!(component.component, styled);
    assert_eq!(
        component.runs,
        vec![StyledTextRun {
            text: "Sword".to_string(),
            style: ComponentStyle {
                bold: Some(true),
                color: Some(0xaa00aa),
                ..ComponentStyle::default()
            },
        }]
    );
}

#[test]
fn decodes_lore_components_in_order_and_at_collection_boundaries() {
    let first = compound(vec![entry(
        "text",
        NbtSummaryValue::String("First".to_string()),
    )]);
    let second = compound(vec![
        entry(
            "translate",
            NbtSummaryValue::String("item.second".to_string()),
        ),
        entry("italic", NbtSummaryValue::Byte(0)),
    ]);
    assert_eq!(
        decode(
            VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
            "minecraft:lore",
            &NbtSummaryValue::List(vec![first.clone(), second.clone()]),
        ),
        DialogDataComponentValueState::Lore {
            lines: vec![
                DialogTextState {
                    text: "First".to_string(),
                    runs: vec![StyledTextRun {
                        text: "First".to_string(),
                        style: ComponentStyle::default(),
                    }],
                    component: first,
                },
                DialogTextState {
                    text: "item.second".to_string(),
                    runs: vec![StyledTextRun {
                        text: "item.second".to_string(),
                        style: ComponentStyle {
                            italic: Some(false),
                            ..ComponentStyle::default()
                        },
                    }],
                    component: second,
                },
            ],
        }
    );

    assert!(matches!(
        decode(
            VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
            "minecraft:lore",
            &NbtSummaryValue::List(Vec::new()),
        ),
        DialogDataComponentValueState::Lore { lines } if lines.is_empty()
    ));
    let maximum = NbtSummaryValue::List(vec![NbtSummaryValue::String(String::new()); 256]);
    let DialogDataComponentValueState::Lore { lines } = decode(
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        "minecraft:lore",
        &maximum,
    ) else {
        panic!("expected lore");
    };
    assert_eq!(lines.len(), 256);
}

#[test]
fn rejects_invalid_text_component_roots_and_lore_bounds() {
    let invalid_components = [
        NbtSummaryValue::Int(1),
        NbtSummaryValue::List(Vec::new()),
        compound(vec![
            entry("type", NbtSummaryValue::String("text".to_string())),
            entry(
                "translate",
                NbtSummaryValue::String("typed.dispatch.must.not.fallback".to_string()),
            ),
        ]),
        compound(vec![
            entry("text", NbtSummaryValue::String("value".to_string())),
            entry("extra", NbtSummaryValue::List(Vec::new())),
        ]),
        compound(vec![
            entry("text", NbtSummaryValue::String("value".to_string())),
            entry("bold", NbtSummaryValue::String("true".to_string())),
        ]),
        compound(vec![
            entry("text", NbtSummaryValue::String("value".to_string())),
            entry(
                "click_event",
                compound(vec![
                    entry("action", NbtSummaryValue::String("open_file".to_string())),
                    entry("path", NbtSummaryValue::String("debug".to_string())),
                ]),
            ),
        ]),
    ];
    for (type_id, resource_id) in [
        (
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            "minecraft:custom_name",
        ),
        (
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
            "minecraft:item_name",
        ),
    ] {
        for value in &invalid_components {
            assert!(decode_supported_component_value(type_id, resource_id, value, None).is_err());
        }
    }

    for value in [
        NbtSummaryValue::String("not a list".to_string()),
        NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
        NbtSummaryValue::List(vec![NbtSummaryValue::String(String::new()); 257]),
    ] {
        assert!(decode_supported_component_value(
            VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
            "minecraft:lore",
            &value,
            None,
        )
        .is_err());
    }
}
