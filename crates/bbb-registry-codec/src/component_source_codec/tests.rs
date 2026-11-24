use super::*;
use crate::registry_codec::{
    decode_client_known_pack_registry_json, decode_direct_registry_json, RegistryElementSource,
};
use bbb_protocol::nbt_summary::{entry, NbtSummaryEntry};

const OWNER: &str = "minecraft:chat_type";
const CODEC: &str = "ComponentSerialization.CODEC";

#[test]
fn raw_translatable_preserves_key_fallback_and_primitive_arguments() {
    let source = br#"{"translate":"\ud800","fallback":"\ud801%s","with":["\udc00"]}"#;
    for data in [
        decode_direct_registry_json(source).unwrap(),
        decode_client_known_pack_registry_json(source).unwrap(),
    ] {
        let value =
            decode_component_source(data.root(), "component", &mut context(data.source)).unwrap();
        let nbt = value.network_nbt_unchecked();
        assert_eq!(
            field(nbt, "translate"),
            Some(&NbtSummaryValue::string_from_utf16(vec![0xd800]))
        );
        assert_eq!(
            field(nbt, "fallback"),
            Some(&NbtSummaryValue::string_from_utf16(vec![0xd801, 37, 115]))
        );
        assert_eq!(
            field(nbt, "with"),
            Some(&NbtSummaryValue::List(vec![
                NbtSummaryValue::string_from_utf16(vec![0xdc00])
            ]))
        );
    }
}

#[test]
fn raw_literal_component_json_nbt_and_display_share_canonical_text() {
    for json in [
        br#""\ud800""#.as_slice(),
        br#"{"text":"\ud800"}"#.as_slice(),
    ] {
        for data in [
            decode_direct_registry_json(json).unwrap(),
            decode_client_known_pack_registry_json(json).unwrap(),
        ] {
            let value =
                decode_component_source(data.root(), "component", &mut context(data.source))
                    .unwrap();
            let expected = NbtSummaryValue::string_from_utf16(vec![0xd800]);
            assert_eq!(value.network_nbt_unchecked(), &expected);
            let rendered =
                bbb_protocol::RichTextComponent::try_from_component_nbt(expected).unwrap();
            assert_eq!(rendered.text(), "�");
        }
    }
    let value = decode_json(br#"{"text":"\ud800","bold":true,"extra":["\udc00"]}"#).unwrap();
    let nbt = value.network_nbt_unchecked().clone();
    let rendered = bbb_protocol::RichTextComponent::try_from_component_nbt(nbt).unwrap();
    assert_eq!(rendered.text(), "��");
    assert!(rendered
        .source_styled_parts()
        .iter()
        .all(|part| part.style.bold == Some(true)));
}

fn context(source: RegistryElementSource) -> RegistryCodecDecodeContext<'static> {
    RegistryCodecDecodeContext::new(OWNER, CODEC, source)
}

fn decode_json(json: &[u8]) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let data = decode_direct_registry_json(json).unwrap();
    decode_component_source(data.root(), "component", &mut context(data.source))
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    fields
        .iter()
        .rev()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

#[test]
fn all_three_sources_write_the_same_canonical_nbt() {
    let json = br##"{"type":"text","text":"hello","color":"#ff00aa","bold":true,"ignored":1}"##;
    let server = decode_direct_registry_json(json).unwrap();
    let client = decode_client_known_pack_registry_json(json).unwrap();
    let nbt = compound(vec![
        entry("ignored", NbtSummaryValue::Int(1)),
        entry("type", string("text")),
        entry("text", string("hello")),
        entry("color", string("#ff00aa")),
        entry("bold", NbtSummaryValue::Int(1)),
    ]);
    let server =
        decode_component_source(server.root(), "component", &mut context(server.source)).unwrap();
    let client =
        decode_component_source(client.root(), "component", &mut context(client.source)).unwrap();
    let network = decode_component_source(
        RegistryElementValueRef::Nbt(&nbt),
        "component",
        &mut context(RegistryElementSource::NetworkNbt),
    )
    .unwrap();
    let expected = compound(vec![
        entry("text", string("hello")),
        entry("color", string("#FF00AA")),
        entry("bold", NbtSummaryValue::Byte(1)),
    ]);
    assert_eq!(server.network_nbt_unchecked(), &expected);
    assert_eq!(client.network_nbt_unchecked(), &expected);
    assert_eq!(network.network_nbt_unchecked(), &expected);
}

#[test]
fn nbt_content_selects_the_named_source_and_honors_explicit_source() {
    for (json, source, value) in [
        (
            r#"{"nbt":"foo","entity":"@s","block":"~ ~ ~","storage":"test"}"#,
            "entity",
            "@s",
        ),
        (
            r#"{"nbt":"foo","block":"~ ~ ~","storage":"test"}"#,
            "block",
            "~ ~ ~",
        ),
        (
            r#"{"nbt":"foo","storage":"test"}"#,
            "storage",
            "minecraft:test",
        ),
        (
            r#"{"nbt":"foo","source":"storage","entity":"@s","storage":"test"}"#,
            "storage",
            "minecraft:test",
        ),
    ] {
        let decoded = decode_json(json.as_bytes()).unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &compound(vec![
                entry("nbt", string("foo")),
                entry(source, string(value))
            ])
        );
    }
}

#[test]
fn root_alternatives_and_writer_collapse_match_component_codec() {
    let literal = decode_json(br#""plain""#).unwrap();
    assert_eq!(literal.network_nbt_unchecked(), &string("plain"));
    let collapsed = decode_json(br#"{"text":"plain"}"#).unwrap();
    assert_eq!(collapsed.network_nbt_unchecked(), &string("plain"));
    let list = decode_json(br#"["first","second"]"#).unwrap();
    assert_eq!(
        list.network_nbt_unchecked(),
        &compound(vec![
            entry("text", string("first")),
            entry("extra", NbtSummaryValue::List(vec![string("second")]))
        ])
    );
    let nested = decode_json(
        br#"[{"text":"first","extra":[{"text":"nested","click_event":{"action":"show_dialog","dialog":"example:nested"}}]},{"text":"second","click_event":{"action":"show_dialog","dialog":"example:second"}}]"#,
    )
    .unwrap();
    let references = nested
        .references()
        .map(|reference| (reference.target_registry, reference.resource_id.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        references,
        std::collections::BTreeSet::from([
            ("minecraft:dialog", "example:nested"),
            ("minecraft:dialog", "example:second"),
        ])
    );
    let rendered = bbb_protocol::RichTextComponent::try_from_component_nbt(
        nested.network_nbt_unchecked().clone(),
    )
    .unwrap();
    assert_eq!(rendered.text(), "firstnestedsecond");
    assert!(decode_json(br#"[]"#).is_err());
}

#[test]
fn style_writer_orders_and_normalizes_every_field() {
    let value = decode_json(
        br#"{"text":"styled","color":"red","shadow_color":[1,0,0,1],"bold":false,"italic":true,"underlined":false,"strikethrough":true,"obfuscated":false,"click_event":{"action":"copy_to_clipboard","value":"copy"},"hover_event":{"action":"show_text","value":"hover"},"insertion":"insert","font":"uniform"}"#,
    )
    .unwrap();
    let NbtSummaryValue::Compound(fields) = value.network_nbt_unchecked() else {
        panic!("styled component must be a map")
    };
    assert_eq!(
        fields.iter().map(|field| &field.name).collect::<Vec<_>>(),
        vec![
            "text",
            "color",
            "shadow_color",
            "bold",
            "italic",
            "underlined",
            "strikethrough",
            "obfuscated",
            "click_event",
            "hover_event",
            "insertion",
            "font",
        ]
    );
    assert_eq!(
        field(value.network_nbt_unchecked(), "color"),
        Some(&string("red"))
    );
    assert_eq!(
        field(value.network_nbt_unchecked(), "shadow_color"),
        Some(&NbtSummaryValue::Int(0xFFFF_0000u32 as i32))
    );
    assert_eq!(
        field(value.network_nbt_unchecked(), "font"),
        Some(&string("minecraft:uniform"))
    );
}

#[test]
fn dialog_static_actions_reuse_click_payloads_and_append_the_registry_type() {
    let data = decode_direct_registry_json(
        br#"{"type":"copy_to_clipboard","value":"copy","ignored":true}"#,
    )
    .unwrap();
    let value =
        decode_dialog_static_action_source(data.root(), "action", &mut context(data.source))
            .unwrap();
    assert_eq!(
        value.network_nbt_unchecked(),
        &compound(vec![
            entry("value", string("copy")),
            entry("type", string("minecraft:copy_to_clipboard")),
        ])
    );

    let dynamic =
        decode_direct_registry_json(br#"{"type":"dynamic/run_command","template":"$(x)"}"#)
            .unwrap();
    assert!(decode_dialog_static_action_source(
        dynamic.root(),
        "action",
        &mut context(dynamic.source),
    )
    .is_err());
}

#[test]
fn every_hover_action_uses_its_semantic_owner() {
    for event in [
        r#"{"action":"show_text","value":{"translate":"hover.key"}}"#,
        r#"{"action":"show_item","id":"stone","count":2}"#,
        r#"{"action":"show_entity","id":"pig","uuid":"1-1-1-1-1","name":"Pig"}"#,
        r#"{"action":"show_entity","id":"pig","uuid":[1,65537,65536,1],"name":"Pig"}"#,
    ] {
        let json = format!(r#"{{"text":"x","hover_event":{event}}}"#);
        let value = decode_json(json.as_bytes()).unwrap();
        if event.contains("show_item") {
            let hover = field(value.network_nbt_unchecked(), "hover_event").unwrap();
            assert_eq!(field(hover, "id"), Some(&string("minecraft:stone")));
            assert_eq!(field(hover, "count"), Some(&NbtSummaryValue::Int(2)));
        }
        if event.contains("show_entity") {
            let hover = field(value.network_nbt_unchecked(), "hover_event").unwrap();
            assert_eq!(
                field(hover, "uuid"),
                Some(&NbtSummaryValue::IntArray(vec![1, 65537, 65536, 1]))
            );
        }
    }
}

#[test]
fn resolvable_profile_alternatives_encode_through_the_full_codec() {
    for source in [
        br#"{"object":"player","player":"Alex"}"#.as_slice(),
        br#"{"player":"Alex"}"#.as_slice(),
    ] {
        let bare = decode_json(source).unwrap();
        assert_eq!(
            bare.network_nbt_unchecked(),
            &compound(vec![entry(
                "player",
                compound(vec![entry("name", string("Alex"))]),
            )])
        );
    }
    let value = decode_json(
        br#"{"object":"player","player":{"id":[0,0,0,1],"name":"Alex","properties":[{"name":"textures","value":"v","signature":"s"}],"texture":"skin","model":"wide"},"hat":false}"#,
    )
    .unwrap();
    let player = field(value.network_nbt_unchecked(), "player").unwrap();
    assert_eq!(
        field(player, "id"),
        Some(&NbtSummaryValue::IntArray(vec![0, 0, 0, 1]))
    );
    assert_eq!(field(player, "texture"), Some(&string("minecraft:skin")));
    assert_eq!(
        field(value.network_nbt_unchecked(), "hat"),
        Some(&NbtSummaryValue::Byte(0))
    );

    let ordered =
        decode_json(br#"{"object":"player","player":{"properties":{"z":["1"],"a":["2"]}}}"#)
            .unwrap();
    let player = field(ordered.network_nbt_unchecked(), "player").unwrap();
    assert_eq!(
        field(player, "properties"),
        Some(&NbtSummaryValue::List(vec![
            compound(vec![
                entry("name", string("z")),
                entry("value", string("1"))
            ]),
            compound(vec![
                entry("name", string("a")),
                entry("value", string("2"))
            ]),
        ]))
    );

    let property_map = format!(
        r#"{{"object":"player","player":{{"properties":{{"textures":[{}]}}}}}}"#,
        (0..17).map(|_| r#""v""#).collect::<Vec<_>>().join(",")
    );
    // Exceeding the 16-property wire cap must be rejected at decode time to avoid Chunk/Join Ban.
    assert!(decode_json(property_map.as_bytes()).is_err());
}
