use super::*;

fn component(json: serde_json::Value) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt(crate::json_value_to_nbt_summary(&json).unwrap())
        .unwrap()
}

#[test]
fn component_shadow_color_preserves_argb_and_zero_override() {
    let rich = component(serde_json::json!({
        "text": "parent", "shadow_color": -2130771713i32,
        "extra": [
            {"text": " inherited"},
            {"text": " suppressed", "shadow_color": 0},
            {"text": " transparent", "shadow_color": 0x00ff0000}
        ]
    }));
    assert_eq!(rich.styled().len(), 3);
    assert_eq!(rich.styled()[0].text, "parent inherited");
    assert_eq!(rich.styled()[0].style.shadow_color, Some(0x80ff00ff));
    assert_eq!(rich.styled()[1].style.shadow_color, Some(0));
    assert_eq!(rich.styled()[2].style.shadow_color, Some(0x00ff0000));
    assert_eq!(rich.styled()[1].style.color, None);
}

#[test]
fn component_shadow_color_alone_separates_adjacent_styles() {
    let rich = component(serde_json::json!({
        "text": "unset", "extra": [
            {"text": "zero", "shadow_color": 0},
            {"text": "also zero", "shadow_color": [0, 0, 0, 0]},
            {"text": "unset again"}
        ]
    }));
    assert_eq!(
        rich.styled()
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>(),
        ["unset", "zeroalso zero", "unset again"]
    );
    assert_ne!(rich.styled()[0].style, rich.styled()[1].style);
}

#[test]
fn component_shadow_color_vector_uses_float_floor_and_channel_masks() {
    for (channels, expected) in [
        ([1.0, 0.5, 0.0, 1.0], 0xffff7f00),
        ([-0.1, 1.1, 2.0, 0.5], 0x7fe618fe),
        (
            [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.0],
            0xff00ff00,
        ),
    ] {
        let raw = NbtSummaryValue::Compound(vec![nbt_entry(
            "shadow_color",
            NbtSummaryValue::List(
                channels
                    .map(|v| NbtSummaryValue::Float(v.to_bits()))
                    .to_vec(),
            ),
        )]);
        let style = RichTextStyle::try_from_style_nbt(raw).unwrap();
        assert_eq!(style.style().shadow_color, Some(expected));
        let encoded = RichTextStyle::try_from_style(style.style().clone()).unwrap();
        assert_eq!(
            find_entry(
                match encoded.component() {
                    NbtSummaryValue::Compound(entries) => entries,
                    _ => unreachable!(),
                },
                "shadow_color"
            ),
            Some(&NbtSummaryValue::Int(expected as i32))
        );
    }
}

#[test]
fn component_shadow_color_scalar_narrowing_and_roundtrip() {
    for (value, expected) in [
        (NbtSummaryValue::Byte(-1), u32::MAX),
        (NbtSummaryValue::Short(-2), u32::MAX - 1),
        (NbtSummaryValue::Int(i32::MIN), 0x80000000),
        (NbtSummaryValue::Long(0x180ff00ff), 0x80ff00ff),
        (NbtSummaryValue::Float(1.9f32.to_bits()), 1),
        (NbtSummaryValue::Double((-1.9f64).to_bits()), u32::MAX),
    ] {
        let raw = NbtSummaryValue::Compound(vec![nbt_entry("shadow_color", value)]);
        let rich = RichTextStyle::try_from_style_nbt(raw.clone()).unwrap();
        assert_eq!(rich.style().shadow_color, Some(expected));
        assert_eq!(rich.component(), &raw);
        let json = serde_json::to_vec(&rich).unwrap();
        assert_eq!(
            serde_json::from_slice::<RichTextStyle>(&json).unwrap(),
            rich
        );
    }
}

#[test]
fn component_shadow_color_keeps_nested_hover_and_trusted_projection_checks() {
    let rich = component(serde_json::json!({
        "text": "outer", "shadow_color": 123,
        "hover_event": {"action": "show_text", "value": {
            "text": "hover", "shadow_color": -1
        }}
    }));
    let Some(ComponentHoverEvent::ShowText { value, .. }) = &rich.styled()[0].style.hover_event
    else {
        panic!("missing hover text")
    };
    assert_eq!(value[0].style.shadow_color, Some(u32::MAX));
    let mut json = serde_json::to_value(&rich).unwrap();
    assert_eq!(
        serde_json::from_value::<RichTextComponent>(json.clone()).unwrap(),
        rich
    );
    json["styled"][0]["style"]["shadow_color"] = serde_json::json!(0);
    assert!(serde_json::from_value::<RichTextComponent>(json.clone()).is_err());
    json["styled"][0]["style"]
        .as_object_mut()
        .unwrap()
        .remove("shadow_color");
    assert!(serde_json::from_value::<RichTextComponent>(json).is_err());
    let style = RichTextStyle::try_from_style(ComponentStyle {
        shadow_color: Some(0),
        ..Default::default()
    })
    .unwrap();
    let mut json = serde_json::to_value(&style).unwrap();
    json["style"]
        .as_object_mut()
        .unwrap()
        .remove("shadow_color");
    assert!(serde_json::from_value::<RichTextStyle>(json).is_err());
}

#[test]
fn component_shadow_color_wire_decode_keeps_inherited_style() {
    let rich = component(serde_json::json!({
        "text": "a", "shadow_color": [1, 0.5, 0, 1],
        "extra": [{"text": "b", "shadow_color": 0}]
    }));
    let wire = crate::encode_nbt_summary(rich.component(), 65536).unwrap();
    assert_eq!(
        decode_styled_component_summary(&wire).unwrap(),
        rich.styled()
    );
}

#[test]
fn component_source_parts_preserve_legacy_boundaries_without_changing_saved_projection() {
    let rich = component(serde_json::json!({
        "text": "§", "font": "test:font", "bold": true,
        "extra": [{"text": "aX"}, {"text": "§rY", "bold": false}]
    }));
    let saved = serde_json::to_value(&rich).unwrap();
    assert_eq!(rich.styled()[0].text, "§aX");
    let parts = rich.source_styled_parts();
    assert_eq!(
        parts
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["§", "aX", "§rY"]
    );
    assert!(parts
        .iter()
        .all(|run| run.style.font.as_deref() == Some("test:font")));
    assert_eq!(parts[0].style.bold, Some(true));
    assert_eq!(parts[2].style.bold, Some(false));
    assert_eq!(
        parts
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        rich.text()
    );
    assert_eq!(serde_json::to_value(&rich).unwrap(), saved);
    let restored: RichTextComponent = serde_json::from_value(saved).unwrap();
    assert_eq!(restored.source_styled_parts(), parts);
}

#[test]
fn component_source_list_inherits_first_root_without_leaking_extra_style() {
    let rich = component(serde_json::json!([
        {"text": "A", "bold": true, "font": "test:root", "extra": [
            {"text": "B", "bold": false}, "C"
        ]},
        "D",
        {"text": "E", "font": "test:override", "extra": ["F"]}
    ]));
    let saved = serde_json::to_value(&rich).unwrap();
    let parts = rich.source_styled_parts();
    assert_eq!(
        parts.iter().map(|p| p.text.as_str()).collect::<Vec<_>>(),
        ["A", "B", "C", "D", "E", "F"]
    );
    assert_eq!(
        parts.iter().map(|p| p.style.bold).collect::<Vec<_>>(),
        [
            Some(true),
            Some(false),
            Some(true),
            Some(true),
            Some(true),
            Some(true)
        ]
    );
    assert!(parts[..4]
        .iter()
        .all(|p| p.style.font.as_deref() == Some("test:root")));
    assert!(parts[4..]
        .iter()
        .all(|p| p.style.font.as_deref() == Some("test:override")));
    assert_eq!(serde_json::to_value(&rich).unwrap(), saved);
    let restored: RichTextComponent = serde_json::from_value(saved).unwrap();
    assert_eq!(restored.source_styled_parts(), parts);
}

#[test]
fn component_source_nested_list_uses_first_root_not_last_descendant() {
    let rich = component(serde_json::json!({
        "text": "P", "italic": true, "extra": [
            [[{"text": "A", "bold": true}, {"text": "B", "bold": false}], "C"],
            "D"
        ]
    }));
    let parts = rich.source_styled_parts();
    assert_eq!(
        parts.iter().map(|p| p.text.as_str()).collect::<Vec<_>>(),
        ["P", "A", "B", "C", "D"]
    );
    assert_eq!(
        parts.iter().map(|p| p.style.bold).collect::<Vec<_>>(),
        [None, Some(true), Some(false), Some(true), None]
    );
    assert!(parts.iter().all(|p| p.style.italic == Some(true)));
}
