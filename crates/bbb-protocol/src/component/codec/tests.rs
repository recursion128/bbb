use super::*;

#[test]
fn accepts_component_root_alternatives_and_simple_contents() {
    for value in [
        string("literal"),
        list([string("first"), compound([("text", string("second"))])]),
        compound([("text", string("plain"))]),
        compound([
            ("type", string("text")),
            ("text", string("typed")),
            ("unknown", NbtSummaryValue::Int(1)),
        ]),
        compound([
            ("translate", string("chat.type.text")),
            ("fallback", NbtSummaryValue::Int(1)),
            (
                "with",
                list([
                    NbtSummaryValue::Int(42),
                    compound([("text", string("sender"))]),
                ]),
            ),
        ]),
        compound([("keybind", string("key.jump"))]),
        compound([(
            "score",
            compound([("name", string("@s")), ("objective", string("points"))]),
        )]),
    ] {
        validate_component_nbt(&value, "component").unwrap();
    }
}

#[test]
fn accepts_structural_complex_contents_extra_and_scalar_style() {
    let value = compound([
        ("selector", string("@a")),
        ("separator", string(", ")),
        (
            "extra",
            list([
                compound([
                    ("nbt", string("Pos[0]")),
                    ("source", string("storage")),
                    ("storage", string("example:data")),
                    ("interpret", string("leniently ignored")),
                ]),
                compound([
                    ("type", string("object")),
                    ("object", string("atlas")),
                    ("sprite", string("item/diamond")),
                    ("fallback", string("diamond")),
                ]),
            ]),
        ),
        ("color", string("dark_blue")),
        (
            "shadow_color",
            list([
                NbtSummaryValue::Float(0.0f32.to_bits()),
                NbtSummaryValue::Int(1),
                NbtSummaryValue::Double(0.5f64.to_bits()),
                NbtSummaryValue::Byte(1),
            ]),
        ),
        ("bold", NbtSummaryValue::Int(1)),
        ("insertion", string("insert")),
        ("font", string("uniform")),
    ]);
    validate_component_nbt(&value, "component").unwrap();
}

#[test]
fn accepts_vanilla_block_data_source_coordinate_forms() {
    for coordinates in [
        "0 64 -12",
        "-2147483648 0 2147483647",
        "~ ~ ~",
        "~0.5 ~1 ~-5",
        "^ ^ ^",
        "^1 ^ ^-5",
        "^.5 ^-0.25 ^1.",
        " 1 2",
        "1  2",
    ] {
        validate_component_nbt(
            &compound([
                ("type", string("nbt")),
                ("nbt", string("Pos")),
                ("source", string("block")),
                ("block", string(coordinates)),
            ]),
            "component",
        )
        .unwrap_or_else(|error| panic!("{coordinates:?} was rejected: {error}"));
    }

    // Legacy fuzzy dispatch checks entity before block, so a malformed extra
    // block field cannot override a valid entity data source.
    validate_component_nbt(
        &compound([
            ("type", string("nbt")),
            ("nbt", string("Pos")),
            ("entity", string("@e")),
            ("block", string("not coordinates")),
        ]),
        "component",
    )
    .unwrap();
}

#[test]
fn rejects_non_brigadier_block_data_source_coordinates() {
    for coordinates in [
        "",
        "0 0",
        "0 0 0 ",
        "0 0 0 extra",
        "0 0 0 1",
        "1.0 2 3",
        "2147483648 0 0",
        "+1 0 0",
        "~+1 ~ ~",
        "~1e2 ~ ~",
        "^1 ~2 ^3",
        "^1  ^3",
        "^NaN ^ ^",
    ] {
        let error = validate_component_nbt(
            &compound([
                ("type", string("nbt")),
                ("nbt", string("Pos")),
                ("source", string("block")),
                ("block", string(coordinates)),
            ]),
            "component",
        )
        .unwrap_err();
        assert!(
            error.contains("field block is not valid block coordinates"),
            "{coordinates:?} failed with unexpected error: {error}"
        );
    }
}

#[test]
fn rejects_invalid_roots_lists_dispatch_and_extra() {
    for (value, expected) in [
        (NbtSummaryValue::Int(1), "is not a component"),
        (NbtSummaryValue::List(Vec::new()), "component list is empty"),
        (NbtSummaryValue::Compound(Vec::new()), "no matching"),
        (
            compound([("type", NbtSummaryValue::Int(1))]),
            "type is not a string",
        ),
        (
            compound([("type", string("unknown"))]),
            "unknown component content type",
        ),
        (
            compound([
                ("type", string("text")),
                ("translate", string("fallback.must.not.win")),
            ]),
            "missing field text",
        ),
        (
            compound([
                ("text", string("value")),
                ("extra", NbtSummaryValue::List(Vec::new())),
            ]),
            "field extra is empty",
        ),
        (
            compound([("text", string("value")), ("extra", string("not a list"))]),
            "extra is not a list",
        ),
    ] {
        let error = validate_component_nbt(&value, "component").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

#[test]
fn rejects_malformed_simple_and_nested_content_fields() {
    for (value, expected) in [
        (
            compound([
                ("type", string("translatable")),
                ("translate", NbtSummaryValue::Int(1)),
            ]),
            "translate is not a string",
        ),
        (
            compound([("translate", string("key")), ("with", string("not a list"))]),
            "no matching",
        ),
        (
            compound([
                ("type", string("translatable")),
                ("translate", string("key")),
                ("with", list([NbtSummaryValue::List(Vec::new())])),
            ]),
            "component list is empty",
        ),
        (
            compound([("type", string("score")), ("score", string("wrong"))]),
            "score is not a compound",
        ),
        (
            compound([
                ("type", string("score")),
                ("score", compound([("name", string("player"))])),
            ]),
            "missing field objective",
        ),
        (
            compound([
                ("type", string("nbt")),
                ("nbt", string("Pos")),
                ("source", string("storage")),
                ("storage", string("Bad")),
            ]),
            "not a valid identifier",
        ),
        (
            compound([
                ("type", string("nbt")),
                ("nbt", string("Pos")),
                ("block", string("~ ~ ~")),
                ("interpret", NbtSummaryValue::Byte(1)),
                ("plain", NbtSummaryValue::Byte(1)),
            ]),
            "cannot both be enabled",
        ),
        (
            compound([
                ("type", string("object")),
                ("object", string("atlas")),
                ("sprite", string("Bad")),
            ]),
            "not a valid identifier",
        ),
    ] {
        let error = validate_component_nbt(&value, "component").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

#[test]
fn rejects_malformed_scalar_style_fields() {
    for (field, value, expected) in [
        ("color", string("#1234567"), "color is invalid"),
        ("bold", string("true"), "bold is not a boolean"),
        (
            "shadow_color",
            list([
                NbtSummaryValue::Int(1),
                NbtSummaryValue::Int(2),
                NbtSummaryValue::Int(3),
            ]),
            "four-number vector4f",
        ),
        (
            "insertion",
            NbtSummaryValue::Int(1),
            "insertion is not a string",
        ),
        ("font", string("Bad"), "font is not a valid identifier"),
    ] {
        let value =
            NbtSummaryValue::Compound(vec![entry("text", string("styled")), entry(field, value)]);
        let error = validate_component_nbt(&value, "component").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

#[test]
fn accepts_all_server_click_actions_and_projects_payloads() {
    let cases = [
        (
            compound([
                ("action", string("open_url")),
                ("url", string("https://example.invalid/path?q=1")),
            ]),
            ComponentClickEvent::OpenUrl {
                url: "https://example.invalid/path?q=1".to_string(),
            },
        ),
        (
            compound([
                ("action", string("run_command")),
                ("command", string("/say hello")),
            ]),
            ComponentClickEvent::RunCommand {
                command: "/say hello".to_string(),
            },
        ),
        (
            compound([
                ("action", string("suggest_command")),
                ("command", string("/give @s diamond")),
            ]),
            ComponentClickEvent::SuggestCommand {
                command: "/give @s diamond".to_string(),
            },
        ),
        (
            compound([
                ("action", string("show_dialog")),
                ("dialog", string("notice")),
            ]),
            ComponentClickEvent::ShowDialog {
                dialog: string("notice"),
            },
        ),
        (
            compound([
                ("action", string("change_page")),
                ("page", NbtSummaryValue::Long(2)),
            ]),
            ComponentClickEvent::ChangePage { page: 2 },
        ),
        (
            compound([
                ("action", string("copy_to_clipboard")),
                ("value", string("copied")),
            ]),
            ComponentClickEvent::CopyToClipboard {
                value: "copied".to_string(),
            },
        ),
        (
            compound([
                ("action", string("custom")),
                ("id", string("example:inspect")),
                ("payload", NbtSummaryValue::Int(7)),
            ]),
            ComponentClickEvent::Custom {
                id: "example:inspect".to_string(),
                payload: Some(NbtSummaryValue::Int(7)),
            },
        ),
    ];

    for (click_event, expected) in cases {
        assert_eq!(
            decode_component_click_event(&click_event, "click event").unwrap(),
            expected
        );
        validate_component_nbt(
            &compound([("text", string("event")), ("click_event", click_event)]),
            "component",
        )
        .unwrap();
    }
}

#[test]
fn accepts_hover_text_item_and_entity_event_shapes() {
    for hover_event in [
        compound([
            ("action", string("show_text")),
            (
                "value",
                compound([("translate", string("item.minecraft.diamond"))]),
            ),
        ]),
        compound([
            ("action", string("show_item")),
            ("id", string("diamond")),
            ("count", NbtSummaryValue::Double(99.0f64.to_bits())),
            (
                "components",
                compound([
                    ("custom_name", compound([("text", string("Shiny"))])),
                    ("damage", NbtSummaryValue::Int(1)),
                    ("!damage", NbtSummaryValue::Compound(Vec::new())),
                ]),
            ),
        ]),
        compound([
            ("action", string("show_entity")),
            ("id", string("minecraft:pig")),
            ("uuid", string("12345678-1234-5678-9abc-def012345678")),
            ("name", compound([("text", string("Pig"))])),
        ]),
        compound([
            ("action", string("show_entity")),
            ("id", string("cow")),
            ("uuid", NbtSummaryValue::IntArray(vec![1, 2, 3, 4])),
        ]),
        compound([
            ("action", string("show_entity")),
            ("id", string("sheep")),
            ("uuid", string("1-2-3-4-5")),
        ]),
        compound([
            ("action", string("show_entity")),
            ("id", string("chicken")),
            ("uuid", string("123456789-2-3-4-5")),
        ]),
        compound([
            ("action", string("show_entity")),
            ("id", string("rabbit")),
            ("uuid", NbtSummaryValue::LongArray(vec![1, 2, 3, 4])),
        ]),
    ] {
        validate_component_nbt(
            &compound([("text", string("hover")), ("hover_event", hover_event)]),
            "component",
        )
        .unwrap();
    }
}

#[test]
fn visits_owner_sensitive_payloads_in_recursive_component_positions() {
    let item_components = compound([("damage", NbtSummaryValue::Int(1))]);
    let dialog = compound([
        ("type", string("notice")),
        ("title", string("Nested dialog")),
    ]);
    let item_component = compound([
        ("text", string("nested item")),
        (
            "hover_event",
            compound([
                ("action", string("show_item")),
                ("id", string("diamond")),
                ("components", item_components.clone()),
            ]),
        ),
    ]);
    let dialog_component = compound([
        ("text", string("nested dialog")),
        (
            "click_event",
            compound([
                ("action", string("show_dialog")),
                ("dialog", dialog.clone()),
            ]),
        ),
    ]);
    let value = compound([
        ("translate", string("component.test")),
        (
            "with",
            list([compound([
                ("text", string("item")),
                (
                    "hover_event",
                    compound([
                        ("action", string("show_item")),
                        ("id", string("diamond")),
                        ("components", item_components.clone()),
                    ]),
                ),
            ])]),
        ),
        (
            "extra",
            list([
                dialog_component.clone(),
                compound([
                    ("text", string("entity")),
                    (
                        "hover_event",
                        compound([
                            ("action", string("show_entity")),
                            ("id", string("pig")),
                            ("uuid", string("1-2-3-4-5")),
                            ("name", dialog_component),
                        ]),
                    ),
                ]),
            ]),
        ),
        (
            "hover_event",
            compound([("action", string("show_text")), ("value", item_component)]),
        ),
    ]);

    let mut visited = Vec::new();
    visit_component_payloads(&value, "component", |payload, what| {
        visited.push((
            match payload {
                ComponentPayload::PlayerProfile(value) => ("profile", value.clone()),
                ComponentPayload::ShowDialog(value) => ("dialog", value.clone()),
                ComponentPayload::ShowItemComponents(value) => ("item", value.clone()),
            },
            what.to_string(),
        ));
        Ok(())
    })
    .unwrap();

    assert_eq!(visited.len(), 4);
    assert_eq!(visited[0].0, ("item", item_components));
    assert!(visited[0]
        .1
        .contains("with[0] field hover_event field components"));
    assert_eq!(visited[1].0, ("dialog", dialog));
    assert!(visited[1]
        .1
        .contains("extra[0] field click_event field dialog"));
    assert!(visited[2]
        .1
        .contains("extra[1] field hover_event field name field click_event field dialog"));
    assert!(visited[3]
        .1
        .contains("hover_event field value field hover_event field components"));
}

#[test]
fn component_payload_visitor_follows_typed_and_fuzzy_player_object_dispatch() {
    let typed_profile = string("Steve");
    let fuzzy_profile = compound([("name", string("Alex"))]);
    let value = list([
        compound([
            ("type", string("object")),
            ("object", string("player")),
            ("player", typed_profile.clone()),
        ]),
        compound([("player", fuzzy_profile.clone())]),
        compound([
            ("sprite", string("item/diamond")),
            ("player", string("atlas branch ignores this extra field")),
        ]),
    ]);

    let mut visited = Vec::new();
    visit_component_payloads(&value, "component", |payload, what| {
        if let ComponentPayload::PlayerProfile(profile) = payload {
            visited.push((profile.clone(), what.to_string()));
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(
        visited,
        [
            (typed_profile, "component[0] field player".to_string()),
            (fuzzy_profile, "component[1] field player".to_string()),
        ]
    );
}

#[test]
fn rejects_disallowed_or_malformed_click_events() {
    let cases = [
        (string("not a compound"), "not a compound"),
        (
            compound([("url", string("https://example.invalid"))]),
            "missing field action",
        ),
        (
            compound([("action", string("open_file")), ("path", string("debug"))]),
            "not allowed from a server",
        ),
        (
            compound([
                ("action", string("open_url")),
                ("url", string("file:///tmp/debug")),
            ]),
            "unsupported scheme",
        ),
        (
            compound([
                ("action", string("run_command")),
                ("command", string("/say §bad")),
            ]),
            "disallowed chat character",
        ),
        (
            compound([
                ("action", string("show_dialog")),
                ("dialog", NbtSummaryValue::Int(1)),
            ]),
            "neither a resource id nor a direct value",
        ),
        (
            compound([
                ("action", string("change_page")),
                ("page", NbtSummaryValue::Int(0)),
            ]),
            "must be positive",
        ),
        (
            compound([("action", string("custom")), ("id", string("Bad"))]),
            "not a valid identifier",
        ),
        (
            compound([("action", string("unknown"))]),
            "unknown click action",
        ),
    ];

    for (click_event, expected) in cases {
        let error = validate_component_nbt(
            &compound([("text", string("event")), ("click_event", click_event)]),
            "component",
        )
        .unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

#[test]
fn rejects_malformed_hover_event_payloads() {
    let cases = [
        (string("not a compound"), "not a compound"),
        (
            compound([("action", string("unknown"))]),
            "unknown hover action",
        ),
        (
            compound([
                ("action", string("show_text")),
                ("value", NbtSummaryValue::List(Vec::new())),
            ]),
            "component list is empty",
        ),
        (
            compound([("action", string("show_item")), ("id", string("air"))]),
            "not a non-empty built-in item",
        ),
        (
            compound([
                ("action", string("show_item")),
                ("id", string("diamond")),
                ("count", NbtSummaryValue::Int(100)),
            ]),
            "outside 1..=99",
        ),
        (
            compound([
                ("action", string("show_item")),
                ("id", string("diamond")),
                ("components", string("not a map")),
            ]),
            "components is not a compound",
        ),
        (
            compound([
                ("action", string("show_item")),
                ("id", string("diamond")),
                ("components", compound([("missing", compound([]))])),
            ]),
            "unknown component minecraft:missing",
        ),
        (
            compound([
                ("action", string("show_item")),
                ("id", string("diamond")),
                (
                    "components",
                    compound([("creative_slot_lock", compound([]))]),
                ),
            ]),
            "is not persistent",
        ),
        (
            compound([
                ("action", string("show_item")),
                ("id", string("diamond")),
                (
                    "components",
                    compound([("!damage", NbtSummaryValue::Int(0))]),
                ),
            ]),
            "removed component minecraft:damage payload is not a compound",
        ),
        (
            compound([
                ("action", string("show_item")),
                ("id", string("diamond")),
                (
                    "components",
                    NbtSummaryValue::Compound(vec![
                        entry("damage", NbtSummaryValue::Int(1)),
                        entry("minecraft:damage", NbtSummaryValue::Int(2)),
                    ]),
                ),
            ]),
            "duplicate component patch key minecraft:damage",
        ),
        (
            compound([
                ("action", string("show_entity")),
                ("id", string("example:pig")),
                ("uuid", NbtSummaryValue::IntArray(vec![1, 2, 3, 4])),
            ]),
            "not a built-in entity type",
        ),
        (
            compound([
                ("action", string("show_entity")),
                ("id", string("pig")),
                ("uuid", string("not-a-uuid")),
            ]),
            "not a lenient UUID",
        ),
        (
            compound([
                ("action", string("show_entity")),
                ("id", string("pig")),
                ("uuid", string("12345678123456789abcdef012345678")),
            ]),
            "not a lenient UUID",
        ),
        (
            compound([
                ("action", string("show_entity")),
                ("id", string("pig")),
                ("uuid", NbtSummaryValue::IntArray(vec![1, 2, 3, 4])),
                ("name", NbtSummaryValue::List(Vec::new())),
            ]),
            "component list is empty",
        ),
    ];

    for (hover_event, expected) in cases {
        let error = validate_component_nbt(
            &compound([("text", string("hover")), ("hover_event", hover_event)]),
            "component",
        )
        .unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

fn compound<const N: usize>(entries: [(&str, NbtSummaryValue); N]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| entry(name, value))
            .collect(),
    )
}

fn list<const N: usize>(values: [NbtSummaryValue; N]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.into_iter().collect())
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}
