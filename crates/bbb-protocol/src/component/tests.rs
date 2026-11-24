use super::*;
#[test]
fn raw_literal_component_preserves_value_and_projects_visible_styled_text() {
    let raw = NbtSummaryValue::string_from_utf16(vec![0xd800, 65, 0xdc00]);
    let plain = RichTextComponent::try_from_component_nbt(raw.clone()).unwrap();
    assert_eq!(plain.component(), &raw);
    assert_eq!(plain.text(), "�A�");
    assert!(!plain.is_empty());
    assert_eq!(plain.source_styled_parts()[0].text, "�A�");
    let wrapped = NbtSummaryValue::Compound(vec![nbt_entry("text", raw.clone())]);
    assert_eq!(collapsed_component_nbt(&wrapped), raw);
    assert_eq!(
        canonical_component_codec_value(&wrapped),
        canonical_component_codec_value(&raw)
    );
    let joined = component_with_siblings_nbt(raw.clone(), vec![raw.clone()]).unwrap();
    let joined = RichTextComponent::try_from_component_nbt(joined).unwrap();
    assert_eq!(joined.text(), "�A��A�");
    assert_eq!(joined.source_styled_parts().len(), 2);
    let styled = NbtSummaryValue::Compound(vec![
        nbt_entry("text", raw.clone()),
        nbt_entry("bold", NbtSummaryValue::Byte(1)),
        nbt_entry("extra", NbtSummaryValue::List(vec![raw])),
    ]);
    let styled = RichTextComponent::try_from_component_nbt(styled).unwrap();
    assert_eq!(styled.source_styled_parts().len(), 2);
    assert!(styled
        .source_styled_parts()
        .iter()
        .all(|part| part.style.bold == Some(true)));
    assert_eq!(styled.text(), "�A��A�");
}

fn entries(pairs: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        pairs
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

#[test]
fn an_unstyled_literal_collapses_and_a_styled_one_does_not() {
    let plain = entries(vec![("text", NbtSummaryValue::String("bbb".into()))]);
    assert_eq!(
        collapsed_component_nbt(&plain),
        NbtSummaryValue::String("bbb".into())
    );
    let styled = entries(vec![
        ("text", NbtSummaryValue::String("bbb".into())),
        ("bold", NbtSummaryValue::Byte(1)),
    ]);
    assert_eq!(collapsed_component_nbt(&styled), styled);
    let translatable = entries(vec![(
        "translate",
        NbtSummaryValue::String("commands.help.header".into()),
    )]);
    assert_eq!(collapsed_component_nbt(&translatable), translatable);
}

#[test]
fn the_two_component_bearing_fields_collapse_with_their_parent() {
    let nested = entries(vec![
        ("text", NbtSummaryValue::String("head".into())),
        (
            "extra",
            NbtSummaryValue::List(vec![entries(vec![(
                "text",
                NbtSummaryValue::String("tail".into()),
            )])]),
        ),
    ]);
    assert_eq!(
        collapsed_component_nbt(&nested),
        entries(vec![
            ("text", NbtSummaryValue::String("head".into())),
            (
                "extra",
                NbtSummaryValue::List(vec![NbtSummaryValue::String("tail".into())]),
            ),
        ])
    );
    let hover = entries(vec![
        (
            "translate",
            NbtSummaryValue::String("chat.type.text".into()),
        ),
        (
            "with",
            NbtSummaryValue::List(vec![entries(vec![(
                "text",
                NbtSummaryValue::String("name".into()),
            )])]),
        ),
    ]);
    assert_eq!(
        collapsed_component_nbt(&hover),
        entries(vec![
            (
                "translate",
                NbtSummaryValue::String("chat.type.text".into())
            ),
            (
                "with",
                NbtSummaryValue::List(vec![NbtSummaryValue::String("name".into())]),
            ),
        ])
    );
}

#[test]
fn decodes_string_component_nbt() {
    let payload = nbt_string_root("Disconnected");
    assert_eq!(
        decode_component_summary(&payload).unwrap(),
        "Disconnected".to_string()
    );
}

#[test]
fn direct_component_projection_reads_duplicate_fields_last_wins() {
    let component = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("first".into()),
        },
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("last".into()),
        },
    ]);
    assert_eq!(
        styled_component_runs_from_nbt_value(&component),
        vec![StyledTextRun {
            text: "last".to_string(),
            style: ComponentStyle::default(),
        }]
    );
}

#[test]
fn decodes_component_text_without_trimming_magic_names() {
    let payload = nbt_string_root(" jeb_ ");
    assert_eq!(decode_component_summary(&payload).unwrap(), " jeb_ ");
}

#[test]
fn decodes_compound_text_and_extra_component_nbt() {
    let mut payload = vec![10];
    write_named_string(&mut payload, "text", "Kicked");
    payload.push(9);
    write_mutf8(&mut payload, "extra");
    payload.push(10);
    payload.extend_from_slice(&1i32.to_be_bytes());
    write_named_string(&mut payload, "text", " by server");
    payload.push(0);
    payload.push(0);

    assert_eq!(
        decode_component_summary(&payload).unwrap(),
        "Kicked by server".to_string()
    );
}

#[test]
fn styled_runs_of_plain_string_root_carry_default_style() {
    let payload = nbt_string_root("Hello");
    assert_eq!(
        decode_styled_component_summary(&payload).unwrap(),
        vec![StyledTextRun {
            text: "Hello".to_string(),
            style: ComponentStyle::default(),
        }]
    );
}

#[test]
fn component_codec_equality_normalizes_shorthand_and_preserves_style_identity() {
    let shorthand = NbtSummaryValue::String("Hello".into());
    let object = crate::json_value_to_nbt_summary(&serde_json::json!({"text": "Hello"})).unwrap();
    let bold =
        crate::json_value_to_nbt_summary(&serde_json::json!({"text": "Hello", "bold": true}))
            .unwrap();

    assert!(component_codec_values_equal(&shorthand, &object).unwrap());
    assert!(component_codec_values_equal(&object, &object).unwrap());
    assert!(!component_codec_values_equal(&shorthand, &bold).unwrap());
    assert!(component_codec_values_equal(&shorthand, &shorthand).unwrap());
    assert!(
        !component_codec_values_equal(&shorthand, &NbtSummaryValue::String("Other".into()))
            .unwrap()
    );
    assert!(
        component_codec_values_equal(&NbtSummaryValue::Int(1), &NbtSummaryValue::Int(1)).is_err()
    );
    let malformed = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "text".into(),
        value: NbtSummaryValue::Int(1),
    }]);
    assert!(component_codec_values_equal(&malformed, &malformed).is_err());
    let translate = entries(vec![(
        "translate",
        NbtSummaryValue::String("item.minecraft.bowl".into()),
    )]);
    assert!(component_codec_values_equal(&translate, &translate).unwrap());
    let a = NbtSummaryValue::String("A".into());
    let b = NbtSummaryValue::String("B".into());
    let c = NbtSummaryValue::String("C".into());
    let flat = NbtSummaryValue::List(vec![a.clone(), b.clone(), c.clone()]);
    for nested in [
        NbtSummaryValue::List(vec![
            NbtSummaryValue::List(vec![a.clone(), b.clone()]),
            c.clone(),
        ]),
        NbtSummaryValue::List(vec![
            entries(vec![("text", a), ("extra", NbtSummaryValue::List(vec![b]))]),
            c,
        ]),
    ] {
        assert!(component_codec_values_equal(&flat, &nested).unwrap());
    }
    let raw_key_payload = entries(vec![
        ("text", NbtSummaryValue::String("x".into())),
        (
            "click_event",
            entries(vec![
                ("action", NbtSummaryValue::String("custom".into())),
                ("id", NbtSummaryValue::String("minecraft:foo".into())),
                (
                    "payload",
                    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: vec![0xd800].into(),
                        value: NbtSummaryValue::Int(1),
                    }]),
                ),
            ]),
        ),
    ]);
    assert!(validate_component_nbt(&raw_key_payload, "component").is_ok());
    assert!(component_codec_values_equal(&raw_key_payload, &raw_key_payload).unwrap());
}

#[test]
fn custom_click_payload_map_keeps_raw_keys_and_map_equality() {
    let component = |payload: Vec<NbtSummaryEntry>| {
        entries(vec![
            ("text", NbtSummaryValue::String("x".into())),
            (
                "click_event",
                entries(vec![
                    ("action", NbtSummaryValue::String("custom".into())),
                    ("id", NbtSummaryValue::String("minecraft:foo".into())),
                    ("payload", NbtSummaryValue::Compound(payload)),
                ]),
            ),
        ])
    };
    let key = |units: Vec<u16>, value: i8| NbtSummaryEntry {
        name: units.into(),
        value: NbtSummaryValue::Byte(value),
    };
    let source = component(vec![
        key(vec![0xd800], 1),
        key(vec![0xd801], 2),
        key(vec![0xd800], 3),
    ]);
    let reversed = component(vec![key(vec![0xd801], 2), key(vec![0xd800], 3)]);
    assert!(component_codec_values_equal(&source, &reversed).unwrap());
    assert!(component_codec_values_equal(&reversed, &source).unwrap());
    assert!(!component_codec_values_equal(
        &source,
        &component(vec![key(vec![0xfffd], 3), key(vec![0xd801], 2)])
    )
    .unwrap());
    assert!(!component_codec_values_equal(
        &source,
        &component(vec![key(vec![0xd800], 3), key(vec![0xd802], 2)])
    )
    .unwrap());
    let invalid = entries(vec![
        ("text", NbtSummaryValue::String("x".into())),
        (
            "click_event",
            entries(vec![
                ("action", NbtSummaryValue::String("custom".into())),
                ("id", NbtSummaryValue::Int(1)),
                (
                    "payload",
                    NbtSummaryValue::Compound(vec![key(vec![0xd800], 3)]),
                ),
            ]),
        ),
    ]);
    assert!(component_codec_values_equal(&invalid, &invalid).is_err());
}

#[test]
fn styled_runs_decode_every_style_key_and_named_color() {
    // {text:"Styled", bold:1b, italic:0b, underlined:1b, strikethrough:1b,
    //  obfuscated:1b, color:"dark_purple"}
    let mut payload = vec![10];
    write_named_string(&mut payload, "text", "Styled");
    write_named_byte(&mut payload, "bold", 1);
    write_named_byte(&mut payload, "italic", 0);
    write_named_byte(&mut payload, "underlined", 1);
    write_named_byte(&mut payload, "strikethrough", 1);
    write_named_byte(&mut payload, "obfuscated", 1);
    write_named_string(&mut payload, "color", "dark_purple");
    write_named_string(&mut payload, "insertion", "insert me");
    payload.push(0);

    assert_eq!(
        decode_styled_component_summary(&payload).unwrap(),
        vec![StyledTextRun {
            text: "Styled".to_string(),
            style: ComponentStyle {
                bold: Some(true),
                italic: Some(false),
                underlined: Some(true),
                strikethrough: Some(true),
                obfuscated: Some(true),
                color: Some(0xAA_00_AA),
                color_name: Some(ChatFormatting::DarkPurple),
                shadow_color: None,
                click_event: None,
                hover_event: None,
                insertion: Some("insert me".to_string().into()),
                font: None,
            },
        }]
    );
}

#[test]
fn component_style_projection_accepts_normalized_numeric_booleans() {
    let style = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "bold".into(),
            value: NbtSummaryValue::Long(1),
        },
        NbtSummaryEntry {
            name: "italic".into(),
            value: NbtSummaryValue::Double(0.0f64.to_bits()),
        },
    ]);

    assert_eq!(
        component_style_from_nbt_value(&style),
        Some(ComponentStyle {
            bold: Some(true),
            italic: Some(false),
            ..ComponentStyle::default()
        })
    );
}

#[test]
fn styled_runs_decode_click_event_copy_to_clipboard() {
    // {text:"debug", underlined:1b,
    //  click_event:{action:"copy_to_clipboard", value:"debug value"}}
    let mut payload = vec![10];
    write_named_string(&mut payload, "text", "debug");
    write_named_byte(&mut payload, "underlined", 1);
    write_named_compound_start(&mut payload, "click_event");
    write_named_string(&mut payload, "action", "copy_to_clipboard");
    write_named_string(&mut payload, "value", "debug value");
    payload.push(0);
    payload.push(0);

    assert_eq!(
        decode_styled_component_summary(&payload).unwrap(),
        vec![StyledTextRun {
            text: "debug".to_string(),
            style: ComponentStyle {
                underlined: Some(true),
                click_event: Some(ComponentClickEvent::CopyToClipboard {
                    value: "debug value".to_string().into(),
                }),
                ..ComponentStyle::default()
            },
        }]
    );
}

#[test]
fn styled_runs_retain_hover_text_and_custom_font() {
    let mut payload = vec![10];
    write_named_string(&mut payload, "text", "hover me");
    write_named_string(&mut payload, "font", "example:compact");
    write_named_compound_start(&mut payload, "hover_event");
    write_named_string(&mut payload, "action", "show_text");
    write_named_compound_start(&mut payload, "value");
    write_named_string(&mut payload, "text", "details");
    write_named_byte(&mut payload, "italic", 1);
    payload.push(0);
    payload.push(0);
    payload.push(0);

    let runs = decode_styled_component_summary(&payload).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].style.font.as_deref(), Some("example:compact"));
    assert_eq!(
        runs[0].style.hover_event,
        Some(ComponentHoverEvent::ShowText {
            value: vec![StyledTextRun {
                text: "details".to_string(),
                style: ComponentStyle {
                    italic: Some(true),
                    ..ComponentStyle::default()
                },
            }],
            component: Some(NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".into(),
                    value: NbtSummaryValue::String("details".into()),
                },
                NbtSummaryEntry {
                    name: "italic".into(),
                    value: NbtSummaryValue::Byte(1),
                },
            ])),
        })
    );
}

#[test]
fn component_hover_uuid_formats_every_lenient_codec_shape() {
    let expected = uuid::Uuid::parse_str("00000001-0002-0003-0004-000000000005").unwrap();
    for value in [
        NbtSummaryValue::IntArray(vec![1, 0x0002_0003, 0x0004_0000, 5]),
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Byte(1),
            NbtSummaryValue::Int(0x0002_0003),
            NbtSummaryValue::Long(0x0004_0000),
            NbtSummaryValue::Short(5),
        ]),
        NbtSummaryValue::String("1-2-3-4-5".into()),
    ] {
        assert_eq!(component_hover_uuid(&value), Some(expected));
    }
    let saved = component_hover_uuid_nbt(expected);
    assert_eq!(
        saved,
        NbtSummaryValue::IntArray(vec![1, 0x0002_0003, 0x0004_0000, 5])
    );
    assert_eq!(crate::nbt_summary::codec_uuid(&saved), Some(expected));
    let signed = uuid::Uuid::parse_str("ffffffff-0000-0002-ffff-fffd00000004").unwrap();
    for value in [
        NbtSummaryValue::ByteArray(vec![-1, 2, -3, 4]),
        NbtSummaryValue::LongArray(vec![0x1_ffff_ffff, 2, -3, 4]),
    ] {
        assert_eq!(component_hover_uuid(&value), Some(signed));
    }
    assert_eq!(
        component_hover_uuid(&NbtSummaryValue::IntArray(vec![1, 2, 3])),
        None
    );
}

#[test]
fn component_wire_rejects_absent_root_and_server_disallowed_click_event() {
    assert!(decode_styled_component_summary(&[0]).is_err());

    let mut payload = vec![10];
    write_named_string(&mut payload, "text", "debug");
    write_named_compound_start(&mut payload, "click_event");
    write_named_string(&mut payload, "action", "open_file");
    write_named_string(&mut payload, "path", "screenshots/debug");
    payload.push(0);
    payload.push(0);
    let error = decode_styled_component_summary(&payload).unwrap_err();
    assert!(error.to_string().contains("not allowed from a server"));
}

#[test]
fn component_style_wire_is_distinct_from_component_contents() {
    let empty_style = [10, 0];
    let mut decoder = Decoder::new(&empty_style);
    let style = decode_rich_text_style_from_decoder(&mut decoder).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(style.style(), &ComponentStyle::default());
    assert_eq!(style.component(), &NbtSummaryValue::Compound(Vec::new()));

    let mut malformed_style = vec![10];
    write_named_string(&mut malformed_style, "bold", "true");
    malformed_style.push(0);
    let mut decoder = Decoder::new(&malformed_style);
    let error = decode_rich_text_style_from_decoder(&mut decoder).unwrap_err();
    assert!(error.to_string().contains("bold is not a boolean"));
}

#[test]
fn rich_text_style_retains_exact_nbt_and_rejects_serde_projection_mismatch() {
    let exact = NbtSummaryValue::Compound(vec![
        nbt_entry("bold", NbtSummaryValue::Byte(1)),
        nbt_entry("color", NbtSummaryValue::String("dark_purple".into())),
    ]);
    let style = RichTextStyle::try_from_style_nbt(exact.clone()).unwrap();
    assert_eq!(
        style.style(),
        &ComponentStyle {
            bold: Some(true),
            color: Some(0xAA_00_AA),
            ..ComponentStyle::default()
        }
    );
    assert_eq!(style.component(), &exact);

    let synthesized = RichTextStyle::try_from_style(style.style().clone()).unwrap();
    assert_eq!(synthesized.style(), style.style());

    let mut serialized = serde_json::to_value(&style).unwrap();
    serialized["style"]["bold"] = serde_json::Value::Bool(false);
    let error = serde_json::from_value::<RichTextStyle>(serialized).unwrap_err();
    assert!(error
        .to_string()
        .contains("projection does not match exact style NBT"));
}

#[test]
fn styled_runs_parse_hex_color_and_reject_invalid_colors() {
    let mut payload = vec![10];
    write_named_string(&mut payload, "text", "Hex");
    write_named_string(&mut payload, "color", "#1A2b3C");
    payload.push(0);
    assert_eq!(
        decode_styled_component_summary(&payload).unwrap()[0]
            .style
            .color,
        Some(0x1A_2B_3C)
    );

    // Out-of-range hex and unknown names fail vanilla's Style codec.
    for bad in ["#1FFFFFF", "#nothex", "purple", "DARK_PURPLE"] {
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "Bad");
        write_named_string(&mut payload, "color", bad);
        payload.push(0);
        assert!(
            decode_styled_component_summary(&payload).is_err(),
            "color {bad:?} should fail component decoding"
        );
    }
}

#[test]
fn a_named_colour_serializes_as_its_name_and_any_other_rgb_as_an_upper_case_literal() {
    // `TextColor.serialize` (TextColor.java:41-46) writes the name when the
    // colour carries one, and only otherwise the `String.format("#%06X")`
    // literal — so a `ChatFormatting`-built style is `red`, never `#FF5555`.
    let named = ComponentStyle::with_chat_formatting_color(ChatFormatting::Red).unwrap();
    assert_eq!(
        component_styled_literal_nbt("boom", &named).unwrap(),
        NbtSummaryValue::Compound(vec![
            nbt_entry("text", NbtSummaryValue::String("boom".into())),
            nbt_entry("color", NbtSummaryValue::String("red".into())),
        ])
    );

    let anonymous = ComponentStyle {
        color: Some(0x1A_2B_3C),
        ..ComponentStyle::default()
    };
    assert_eq!(
        component_styled_literal_nbt("boom", &anonymous).unwrap(),
        NbtSummaryValue::Compound(vec![
            nbt_entry("text", NbtSummaryValue::String("boom".into())),
            nbt_entry("color", NbtSummaryValue::String("#1A2B3C".into())),
        ])
    );

    // A `ChatFormatting` that is not a colour has no `TextColor`
    // (`TextColor.fromLegacyFormat`, TextColor.java:69-71).
    assert_eq!(
        ComponentStyle::with_chat_formatting_color(ChatFormatting::Bold),
        None
    );

    // The two `TextColor` fields may not disagree: the name decides the
    // wire form, so a value it does not resolve to is unrepresentable.
    assert!(component_styled_literal_nbt(
        "boom",
        &ComponentStyle {
            color: Some(0x1A_2B_3C),
            color_name: Some(ChatFormatting::Red),
            ..ComponentStyle::default()
        }
    )
    .is_err());
    assert!(component_styled_literal_nbt(
        "boom",
        &ComponentStyle {
            color_name: Some(ChatFormatting::Red),
            ..ComponentStyle::default()
        }
    )
    .is_err());
}

#[test]
fn a_colour_round_trips_through_the_name_it_was_spelled_with() {
    for (serialized, expected) in [
        (
            "red",
            ComponentStyle::with_chat_formatting_color(ChatFormatting::Red).unwrap(),
        ),
        (
            "dark_purple",
            ComponentStyle::with_chat_formatting_color(ChatFormatting::DarkPurple).unwrap(),
        ),
        (
            "#FF5555",
            ComponentStyle {
                color: Some(0xFF_55_55),
                ..ComponentStyle::default()
            },
        ),
    ] {
        let nbt = NbtSummaryValue::Compound(vec![nbt_entry(
            "color",
            NbtSummaryValue::String(serialized.to_string().into()),
        )]);
        let decoded = RichTextStyle::try_from_style_nbt(nbt.clone()).unwrap();
        assert_eq!(decoded.style().color_name, expected.color_name);
        assert_eq!(decoded.style().color, expected.color);
        // Re-encoding a decoded style reproduces the exact bytes, so a
        // hex-spelled colour is never rewritten as a name and back.
        assert_eq!(
            RichTextStyle::try_from_style(decoded.style().clone())
                .unwrap()
                .component(),
            &nbt
        );
    }

    // `applyTo` inherits the whole `TextColor` (Style.java:508), so an
    // inherited colour keeps the name its parent spelled it with.
    let gold = ComponentStyle::with_chat_formatting_color(ChatFormatting::Gold).unwrap();
    let inherited = ComponentStyle::default().apply_to(&gold);
    assert_eq!(inherited.color_name, Some(ChatFormatting::Gold));
    assert_eq!(inherited.color, ChatFormatting::Gold.color());

    // `TextColor.equals` (TextColor.java:51-58) compares the value alone,
    // so `Style.equals` cannot tell the two spellings apart either.
    assert_eq!(
        ComponentStyle::with_chat_formatting_color(ChatFormatting::Red).unwrap(),
        ComponentStyle {
            color: Some(0xFF_55_55),
            ..ComponentStyle::default()
        }
    );
}

#[test]
fn signed_zero_hex_colours_match_java_integer_parse_int() {
    for color in ["#-0", "#-000000", "#+0"] {
        assert_eq!(parse_text_color(color), Some((0, None)), "{color}");
    }
    assert_eq!(parse_text_color("#-1"), None);
}

#[test]
fn styled_runs_inherit_parent_style_into_extra_children() {
    // {text:"Kicked", bold:1b, color:"red", extra:[
    //    {text:" by"},                       // inherits bold + red
    //    {text:" server", bold:0b, italic:1b} // overrides bold, keeps red
    // ]}
    let mut payload = vec![10];
    write_named_string(&mut payload, "text", "Kicked");
    write_named_byte(&mut payload, "bold", 1);
    write_named_string(&mut payload, "color", "red");
    payload.push(9);
    write_mutf8(&mut payload, "extra");
    payload.push(10);
    payload.extend_from_slice(&2i32.to_be_bytes());
    write_named_string(&mut payload, "text", " by");
    payload.push(0);
    write_named_string(&mut payload, "text", " server");
    write_named_byte(&mut payload, "bold", 0);
    write_named_byte(&mut payload, "italic", 1);
    payload.push(0);
    payload.push(0);

    let bold_red = ComponentStyle {
        bold: Some(true),
        color: Some(0xFF_55_55),
        ..ComponentStyle::default()
    };
    assert_eq!(
        decode_styled_component_summary(&payload).unwrap(),
        vec![
            StyledTextRun {
                text: "Kicked by".to_string(),
                style: bold_red,
            },
            StyledTextRun {
                text: " server".to_string(),
                style: ComponentStyle {
                    bold: Some(false),
                    italic: Some(true),
                    color: Some(0xFF_55_55),
                    ..ComponentStyle::default()
                },
            },
        ]
    );

    // The plain-text API is a pure delegation: concatenated run text.
    assert_eq!(
        decode_component_summary(&payload).unwrap(),
        "Kicked by server"
    );
}

#[test]
fn apply_to_matches_vanilla_style_apply_to() {
    let child = ComponentStyle {
        bold: Some(false),
        color: Some(0x12_34_56),
        insertion: Some("child insertion".to_string().into()),
        font: Some("example:child".to_string()),
        ..ComponentStyle::default()
    };
    let parent = ComponentStyle {
        bold: Some(true),
        italic: Some(true),
        color: Some(0xFF_FF_FF),
        hover_event: Some(ComponentHoverEvent::ShowText {
            value: vec![StyledTextRun {
                text: "parent hover".to_string(),
                style: ComponentStyle::default(),
            }],
            component: None,
        }),
        insertion: Some("parent insertion".to_string().into()),
        font: Some("example:parent".to_string()),
        ..ComponentStyle::default()
    };
    assert_eq!(
        child.apply_to(&parent),
        ComponentStyle {
            bold: Some(false),
            italic: Some(true),
            color: Some(0x12_34_56),
            hover_event: parent.hover_event,
            insertion: Some("child insertion".to_string().into()),
            font: Some("example:child".to_string()),
            ..ComponentStyle::default()
        }
    );
}

#[test]
fn styled_runs_summary_text_falls_back_like_the_legacy_decoder() {
    assert_eq!(styled_runs_summary_text(&[]), "component nbt");
    assert_eq!(
        styled_runs_summary_text(&[StyledTextRun {
            text: "ab".to_string(),
            style: ComponentStyle::default(),
        }]),
        "ab"
    );
}

fn sign_side_nbt(out: &mut Vec<u8>, key: &str, lines: &[&str], color: Option<&str>, glowing: bool) {
    out.push(10);
    write_mutf8(out, key);
    write_sign_line_list(out, "messages", lines);
    if let Some(color) = color {
        write_named_string(out, "color", color);
    }
    if glowing {
        write_named_byte(out, "has_glowing_text", 1);
    }
    out.push(0);
}

fn write_sign_line_list(out: &mut Vec<u8>, key: &str, lines: &[&str]) {
    out.push(9);
    write_mutf8(out, key);
    out.push(8);
    out.extend_from_slice(&(lines.len() as i32).to_be_bytes());
    for line in lines {
        write_mutf8(out, line);
    }
}

#[test]
fn decodes_sign_block_entity_nbt_with_color_glowing_and_waxed() {
    let mut payload = vec![10];
    sign_side_nbt(
        &mut payload,
        "front_text",
        &["a", "b", "c", "d"],
        Some("red"),
        true,
    );
    sign_side_nbt(&mut payload, "back_text", &["", "", "", ""], None, false);
    write_named_byte(&mut payload, "is_waxed", 1);
    payload.push(0);

    let sign = decode_sign_block_entity_nbt(&payload).unwrap().unwrap();
    assert!(sign.is_waxed);
    let front = sign.front_text.unwrap();
    assert_eq!(front.color.as_deref(), Some("red"));
    assert!(front.has_glowing_text);
    assert_eq!(front.messages[0][0].text, "a");
    assert_eq!(front.messages[3][0].text, "d");
    let back = sign.back_text.unwrap();
    assert_eq!(back.color, None);
    assert!(!back.has_glowing_text);
    assert!(back.messages.iter().all(Vec::is_empty));
}

#[test]
fn sign_filtered_messages_decode_leniently_and_fall_back_when_malformed() {
    let mut payload = vec![10, 10];
    write_mutf8(&mut payload, "front_text");
    write_sign_line_list(&mut payload, "messages", &["raw", "b", "c", "d"]);
    write_sign_line_list(
        &mut payload,
        "filtered_messages",
        &["filtered", "B", "C", "D"],
    );
    payload.push(0);
    payload.push(0);

    let sign = decode_sign_block_entity_nbt(&payload).unwrap().unwrap();
    let front = sign.front_text.unwrap();
    assert_eq!(front.messages[0][0].text, "raw");
    assert_eq!(
        front.filtered_messages.as_ref().unwrap()[0][0].text,
        "filtered"
    );

    let mut malformed = vec![10, 10];
    write_mutf8(&mut malformed, "front_text");
    write_sign_line_list(&mut malformed, "messages", &["raw", "b", "c", "d"]);
    write_sign_line_list(
        &mut malformed,
        "filtered_messages",
        &["only", "three", "lines"],
    );
    malformed.push(0);
    malformed.push(0);

    let sign = decode_sign_block_entity_nbt(&malformed).unwrap().unwrap();
    let front = sign.front_text.unwrap();
    assert_eq!(front.messages[0][0].text, "raw");
    assert_eq!(front.filtered_messages, None);
}

#[test]
fn sign_message_components_keep_styled_runs() {
    // messages[0] = {text:"Hi", bold:1b, color:"#123456"} as a compound list element.
    let mut payload = vec![10];
    payload.push(10);
    write_mutf8(&mut payload, "front_text");
    payload.push(9);
    write_mutf8(&mut payload, "messages");
    payload.push(10);
    payload.extend_from_slice(&4i32.to_be_bytes());
    for index in 0..4 {
        if index == 0 {
            write_named_string(&mut payload, "text", "Hi");
            write_named_byte(&mut payload, "bold", 1);
            write_named_string(&mut payload, "color", "#123456");
        } else {
            write_named_string(&mut payload, "text", "");
        }
        payload.push(0);
    }
    payload.push(0);
    payload.push(0);

    let sign = decode_sign_block_entity_nbt(&payload).unwrap().unwrap();
    let front = sign.front_text.unwrap();
    assert_eq!(
        front.messages[0],
        vec![StyledTextRun {
            text: "Hi".to_string(),
            style: ComponentStyle {
                bold: Some(true),
                color: Some(0x12_34_56),
                ..ComponentStyle::default()
            },
        }]
    );
    assert!(sign.back_text.is_none());
}

#[test]
fn non_sign_block_entity_nbt_decodes_to_none() {
    // A vault-ish compound without front_text/back_text.
    let mut payload = vec![10];
    write_named_string(&mut payload, "id", "minecraft:vault");
    payload.push(0);
    assert_eq!(decode_sign_block_entity_nbt(&payload).unwrap(), None);
    // An empty (end-tag-only) network NBT.
    assert_eq!(decode_sign_block_entity_nbt(&[0]).unwrap(), None);
}

#[test]
fn sign_messages_list_must_be_exactly_four_lines() {
    let mut payload = vec![10];
    sign_side_nbt(&mut payload, "front_text", &["a", "b", "c"], None, false);
    payload.push(0);
    assert_eq!(decode_sign_block_entity_nbt(&payload).unwrap(), None);
}

fn nbt_string_root(value: &str) -> Vec<u8> {
    let mut payload = vec![8];
    write_mutf8(&mut payload, value);
    payload
}

#[test]
fn component_stack_budget_pins_every_recursive_decode_phase() {
    const STACK_SIZE_BYTES: usize = 2 * 1024 * 1024;
    const STACK_BUDGET_BYTES: usize = 3 * 1024 * 1024 / 2;

    std::thread::Builder::new()
        .stack_size(STACK_SIZE_BYTES)
        .spawn(|| {
            let accepted_depth = codec::MAX_COMPONENT_DEPTH;
            let accepted_wire = nested_component_list_wire(accepted_depth);
            let caller_marker = 0u8;
            crate::test_stack_budget::reset(&caller_marker);
            let rich =
                decode_rich_text_component_from_decoder(&mut Decoder::new(&accepted_wire)).unwrap();
            assert_eq!(rich.text(), "deep");
            assert_stack_budget("network decode and styled projection", STACK_BUDGET_BYTES);

            let component = rich.component();
            crate::test_stack_budget::reset(&caller_marker);
            visit_component_payloads(component, "component", |_, _| Ok(())).unwrap();
            assert_stack_budget("owner-payload traversal", STACK_BUDGET_BYTES);

            crate::test_stack_budget::reset(&caller_marker);
            assert!(canonical_component_codec_value(component).is_ok());
            assert_stack_budget("canonical JsonOps projection", STACK_BUDGET_BYTES);

            let rejected_wire = nested_component_list_wire(accepted_depth + 1);
            let error = decode_rich_text_component_from_decoder(&mut Decoder::new(&rejected_wire))
                .unwrap_err();
            assert_eq!(error.to_string(), "network nbt exceeded max depth");

            let rejected_value = nested_component_list_value(accepted_depth + 1);
            let error = validate_component_nbt(&rejected_value, "component").unwrap_err();
            assert!(
                error.ends_with(&format!(
                    "component nesting exceeds {}",
                    codec::MAX_COMPONENT_DEPTH
                )),
                "{error}"
            );

            assert_eq!(
                decode_component_summary(&nbt_string_root("recovered")).unwrap(),
                "recovered"
            );
        })
        .expect("spawn component stack-budget test thread")
        .join()
        .expect("component stack-budget test thread");
}

fn assert_stack_budget(phase: &str, budget: usize) {
    let used = crate::test_stack_budget::used_bytes();
    assert!(
        used <= budget,
        "depth-{} Component {phase} used {used} bytes of stack, over the {budget}-byte budget",
        codec::MAX_COMPONENT_DEPTH
    );
}

fn nested_component_list_value(depth: usize) -> NbtSummaryValue {
    let mut value = NbtSummaryValue::String("deep".into());
    for _ in 0..depth {
        value = NbtSummaryValue::List(vec![value]);
    }
    value
}

fn nested_component_list_wire(depth: usize) -> Vec<u8> {
    assert!(depth > 0);
    let mut payload = Vec::with_capacity(depth * 6 + 6);
    payload.push(9);
    for level in 0..depth {
        payload.push(if level + 1 == depth { 8 } else { 9 });
        payload.extend_from_slice(&1i32.to_be_bytes());
    }
    write_mutf8(&mut payload, "deep");
    payload
}

fn write_named_byte(out: &mut Vec<u8>, name: &str, value: u8) {
    out.push(1);
    write_mutf8(out, name);
    out.push(value);
}

fn write_named_string(out: &mut Vec<u8>, name: &str, value: &str) {
    out.push(8);
    write_mutf8(out, name);
    write_mutf8(out, value);
}

fn write_named_compound_start(out: &mut Vec<u8>, name: &str) {
    out.push(10);
    write_mutf8(out, name);
}

fn write_mutf8(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(bytes);
}

#[test]
fn rich_text_component_deduplicates_single_and_empty_run_text_storage() {
    assert_eq!(
        std::mem::size_of::<RichTextComponent>(),
        std::mem::size_of::<Option<Box<str>>>()
            + std::mem::size_of::<Vec<StyledTextRun>>()
            + std::mem::size_of::<NbtSummaryValue>()
    );

    let empty = RichTextComponent::plain("");
    assert!(empty.is_empty());
    assert_eq!(empty.text(), "component nbt");
    let empty_json = serde_json::to_value(&empty).unwrap();
    assert_eq!(empty_json["text"], "component nbt");
    assert_eq!(
        serde_json::from_value::<RichTextComponent>(empty_json).unwrap(),
        empty
    );

    let single = RichTextComponent::plain("hello");
    assert_eq!(single.text(), "hello");
    assert_eq!(single.text().as_ptr(), single.styled()[0].text.as_ptr());
    let single_json = serde_json::to_value(&single).unwrap();
    assert_eq!(single_json["text"], "hello");
    assert_eq!(
        serde_json::from_value::<RichTextComponent>(single_json).unwrap(),
        single
    );
}
