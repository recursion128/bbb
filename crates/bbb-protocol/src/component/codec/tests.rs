use super::*;
use crate::nbt_summary::{decode_nbt_summary, encode_nbt_summary_bounded};

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
fn accepts_score_string_fallback_without_resource_or_selector_restrictions() {
    for typed in [false, true] {
        let score = compound([
            ("name", string("@invalid selector")),
            ("objective", string("Bad objective !")),
        ]);
        validate_component_nbt(&score_component(score, typed), "score component").unwrap();
    }
}

#[test]
fn score_required_java_strings_reject_missing_and_non_string_values() {
    let raw = crate::JavaString::from(vec![0xd800]).to_nbt();
    for typed in [false, true] {
        for target in ["name", "objective"] {
            for invalid in [
                None,
                Some(NbtSummaryValue::Int(7)),
                Some(NbtSummaryValue::Byte(1)),
                Some(compound([])),
                Some(list([])),
            ] {
                let mut score = Vec::new();
                for field in ["name", "objective"] {
                    if field == target {
                        if let Some(invalid) = &invalid {
                            score.push((field, invalid.clone()));
                        }
                    } else {
                        score.push((field, raw.clone()));
                    }
                }
                let score = NbtSummaryValue::Compound(
                    score
                        .into_iter()
                        .map(|(name, value)| NbtSummaryEntry {
                            name: name.into(),
                            value,
                        })
                        .collect(),
                );
                assert!(
                    validate_component_nbt(&score_component(score, typed), "score component")
                        .is_err()
                );
            }
        }
    }
}

fn score_component(score: NbtSummaryValue, typed: bool) -> NbtSummaryValue {
    if typed {
        compound([("type", string("score")), ("score", score)])
    } else {
        compound([("score", score)])
    }
}

#[test]
fn accepts_vanilla_advancement_tooltip_with_heterogeneous_extra_list() {
    // Advancement.decorateName appends a literal newline and a translated
    // description to the hover tooltip. Vanilla 26.1 ListTag represents that
    // mixed StringTag/CompoundTag list with its empty-name compound wrapper.
    let announcement = compound([
        ("translate", string("chat.type.advancement.task")),
        (
            "with",
            list([
                compound([("text", string("bbb-playablegate"))]),
                compound([
                    ("translate", string("advancements.nether.root.title")),
                    ("color", string("dark_purple")),
                    (
                        "hover_event",
                        compound([
                            ("action", string("show_text")),
                            (
                                "value",
                                compound([
                                    ("translate", string("advancements.nether.root.title")),
                                    (
                                        "extra",
                                        list([
                                            string("\n"),
                                            compound([(
                                                "translate",
                                                string("advancements.nether.root.description"),
                                            )]),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            ]),
        ),
    ]);
    let wire = encode_nbt_summary_bounded(Some(&announcement), 8 * 1024, 64).unwrap();
    let decoded = decode_nbt_summary(&wire).unwrap().unwrap();
    assert_eq!(decoded, announcement);
    validate_component_nbt(&decoded, "advancement announcement").unwrap();
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
fn validates_compiled_nbt_paths_before_data_source_dispatch() {
    validate_component_nbt(
        &compound([
            ("type", string("nbt")),
            ("nbt", string("Items[{id:'minecraft:diamond'}].components")),
            ("source", string("storage")),
            ("storage", string("example:data")),
        ]),
        "component",
    )
    .unwrap();

    for nbt_path in ["Items[", "Items[0] trailing", "Items{count=1}"] {
        let error = validate_component_nbt(
            &compound([
                ("type", string("nbt")),
                ("nbt", string(nbt_path)),
                ("source", string("storage")),
                ("storage", string("example:data")),
            ]),
            "component",
        )
        .unwrap_err();
        assert!(
            error.contains("field nbt is not a valid NBT path"),
            "{nbt_path:?} failed with unexpected error: {error}"
        );
    }
}

#[test]
fn validates_compiled_entity_selectors_in_content_and_data_sources() {
    for value in [
        compound([
            ("type", string("selector")),
            (
                "selector",
                string("@e[type=minecraft:pig,distance=..16,limit=2]"),
            ),
        ]),
        compound([
            ("type", string("nbt")),
            ("nbt", string("Pos")),
            ("source", string("entity")),
            ("entity", string("@n[nbt={Silent:true}]")),
        ]),
    ] {
        validate_component_nbt(&value, "component").unwrap();
    }

    for (value, expected) in [
        (
            compound([
                ("type", string("selector")),
                ("selector", string("@e[type=minecraft:missing]")),
            ]),
            "field selector is not a valid entity selector",
        ),
        (
            compound([
                ("type", string("nbt")),
                ("nbt", string("Pos")),
                ("source", string("entity")),
                ("entity", string("@e[limit=0]")),
            ]),
            "field entity is not a valid entity selector",
        ),
    ] {
        let error = validate_component_nbt(&value, "component").unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
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
    for value in [
        NbtSummaryValue::Int(1),
        NbtSummaryValue::List(Vec::new()),
        NbtSummaryValue::Compound(Vec::new()),
        compound([("type", NbtSummaryValue::Int(1))]),
        compound([("type", string("unknown"))]),
        compound([
            ("type", string("text")),
            ("translate", string("fallback.must.not.win")),
        ]),
        compound([
            ("text", string("value")),
            ("extra", NbtSummaryValue::List(Vec::new())),
        ]),
        compound([("text", string("value")), ("extra", string("not a list"))]),
    ] {
        assert!(validate_component_nbt(&value, "component").is_err());
    }
}

#[test]
fn rejects_malformed_simple_and_nested_content_fields() {
    for value in [
        compound([
            ("type", string("translatable")),
            ("translate", NbtSummaryValue::Int(1)),
        ]),
        compound([("translate", string("key")), ("with", string("not a list"))]),
        compound([
            ("type", string("translatable")),
            ("translate", string("key")),
            ("with", list([NbtSummaryValue::List(Vec::new())])),
        ]),
        compound([("type", string("score")), ("score", string("wrong"))]),
        compound([
            ("type", string("score")),
            ("score", compound([("name", string("player"))])),
        ]),
        compound([
            ("type", string("nbt")),
            ("nbt", string("Pos")),
            ("source", string("storage")),
            ("storage", string("Bad")),
        ]),
        compound([
            ("type", string("nbt")),
            ("nbt", string("Pos")),
            ("block", string("~ ~ ~")),
            ("interpret", NbtSummaryValue::Byte(1)),
            ("plain", NbtSummaryValue::Byte(1)),
        ]),
        compound([
            ("type", string("object")),
            ("object", string("atlas")),
            ("sprite", string("Bad")),
        ]),
    ] {
        assert!(validate_component_nbt(&value, "component").is_err());
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
                url: "https://example.invalid/path?q=1".into(),
            },
        ),
        (
            compound([
                ("action", string("run_command")),
                ("command", string("/say hello")),
            ]),
            ComponentClickEvent::RunCommand {
                command: "/say hello".into(),
            },
        ),
        (
            compound([
                ("action", string("suggest_command")),
                ("command", string("/give @s diamond")),
            ]),
            ComponentClickEvent::SuggestCommand {
                command: "/give @s diamond".to_string().into(),
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
                value: "copied".to_string().into(),
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
fn open_url_uses_vanilla_java_uri_lexical_rules() {
    let value = compound([
        ("text", string("scoped URI")),
        (
            "click_event",
            compound([
                ("action", string("open_url")),
                ("url", string("http://[fe80::1%eth0]/")),
            ]),
        ),
    ]);

    validate_component_nbt(&value, "component").unwrap();
    assert_eq!(
        super::decode_component_click_event(
            find_entry(
                match &value {
                    NbtSummaryValue::Compound(entries) => entries,
                    _ => unreachable!(),
                },
                "click_event"
            )
            .unwrap(),
            "click event",
        )
        .unwrap(),
        ComponentClickEvent::OpenUrl {
            url: "http://[fe80::1%eth0]/".into(),
        }
    );
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
fn component_payload_rejection_survives_fuzzy_content_and_lenient_separator() {
    let value = compound([
        ("type", string("nbt")),
        ("nbt", string("value")),
        ("storage", string("test:values")),
        (
            "separator",
            compound([
                ("translate", string("component.test")),
                ("keybind", string("key.jump")),
                (
                    "with",
                    list([compound([("player", string("abcdefghijklmnopq"))])]),
                ),
            ]),
        ),
    ]);

    validate_component_nbt(&value, "component").unwrap();
    assert!(visit_component_payloads(&value, "component", |payload, _| {
        if matches!(payload, ComponentPayload::PlayerProfile(_)) {
            return Err("invalid player profile".to_string());
        }
        Ok(())
    })
    .is_err());
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
            "protocol is not permitted",
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
        name: name.into(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string().into())
}

#[test]
fn untrusted_uri_identity_uses_jdk_structural_equality_without_rewriting() {
    let equal = [
        (
            "HTTP://Example.COM/a%2F?q=%Ab#%Cd",
            "http://example.com/a%2f?q=%aB#%cD",
        ),
        ("http://foo:", "http://FOO"),
        ("http://foo:00080", "http://Foo:80"),
        ("http://u%Ab@foo/a", "http://u%aB@FOO/a"),
        ("HTTP:a%2F?X#%Ab", "http:a%2f?X#%aB"),
        ("http://[FE80::A%Eth0]/", "http://[fe80::a%eth0]/"),
        ("http:///a", "http:/a"),
    ];
    for (left, right) in equal {
        let left: crate::JavaString = left.into();
        let right: crate::JavaString = right.into();
        assert_eq!(
            parse_untrusted_uri_units(&left, "uri").unwrap(),
            parse_untrusted_uri_units(&right, "uri").unwrap(),
            "{left:?} {right:?}"
        );
        assert_eq!(
            ComponentClickEvent::OpenUrl { url: left.clone() },
            ComponentClickEvent::OpenUrl { url: right }
        );
        assert_eq!(crate::JavaString::from_nbt(&left.to_nbt()), Some(left));
    }
    for (left, right) in [
        ("http://foo", "https://foo"),
        ("http://foo", "http://foo:80"),
        ("http://foo", "http://foo/"),
        ("http://foo/a", "http://foo/A"),
        ("http://foo/a%41", "http://foo/aA"),
        ("http://foo", "http://foo?"),
        ("http://foo", "http://foo#"),
        ("http://foo", "http://@foo"),
        ("http://User@foo", "http://user@foo"),
        ("http://FOO:bad/a", "http://foo:bad/a"),
        ("http://FOO:2147483648", "http://foo:2147483648"),
        ("http://FOO_bar", "http://foo_bar"),
        ("http://A.123", "http://a.123"),
        ("http://123", "http://0123"),
        ("http://[::1]", "http://[0:0:0:0:0:0:0:1]"),
        ("http://[::ffff:001.2.3.4]", "http://[::ffff:1.2.3.4]"),
        ("http:a", "http:/a"),
        ("http://foo/a/../b", "http://foo/b"),
        ("http://foo.", "http://foo"),
    ] {
        let left: crate::JavaString = left.into();
        let right: crate::JavaString = right.into();
        assert_ne!(
            parse_untrusted_uri_units(&left, "uri").unwrap(),
            parse_untrusted_uri_units(&right, "uri").unwrap(),
            "{left:?} {right:?}"
        );
        assert_ne!(
            ComponentClickEvent::OpenUrl { url: left },
            ComponentClickEvent::OpenUrl { url: right }
        );
    }
}

#[test]
fn untrusted_uri_authority_selects_jdk_server_or_registry_fields() {
    for value in [
        "http://123",
        "http://foo.",
        "http://foo:2147483647",
        "http://foo:",
        "http://001.002.003.004",
    ] {
        let identity = parse_untrusted_uri_units(&value.into(), "uri").unwrap();
        assert!(
            matches!(
                identity.kind,
                UntrustedUriKind::Hierarchical {
                    authority: UntrustedUriAuthority::Server { .. },
                    ..
                }
            ),
            "{value}"
        );
    }
    for value in [
        "http://a.123",
        "http://1.2.3.999",
        "http://foo_bar",
        "http://foo:bad",
        "http://foo:2147483648",
        "http://a@@foo",
        "http://%41",
    ] {
        let identity = parse_untrusted_uri_units(&value.into(), "uri").unwrap();
        assert!(
            matches!(
                identity.kind,
                UntrustedUriKind::Hierarchical {
                    authority: UntrustedUriAuthority::Registry(_),
                    ..
                }
            ),
            "{value}"
        );
    }
    for value in ["http:/", "http:///a", "http://?", "http://#"] {
        let identity = parse_untrusted_uri_units(&value.into(), "uri").unwrap();
        assert!(
            matches!(
                identity.kind,
                UntrustedUriKind::Hierarchical {
                    authority: UntrustedUriAuthority::Absent,
                    ..
                }
            ),
            "{value}"
        );
    }
}

#[test]
fn untrusted_uri_ipv6_embedded_ipv4_accepts_jdk_leading_zeroes_and_bounds() {
    for address in [
        "::",
        "::1",
        "1:2:3:4:5:6:7:8",
        "::ffff:001.002.003.004",
        "::ffff:000000000000255.2.3.4",
        "1:2:3:4:5:6:1.2.3.4",
        "::ffff:1.2.3.4%eth0",
        "::1%25Eth0",
    ] {
        let url = format!("http://[{address}]:0002147483647/a");
        assert!(validate_url(&url).is_ok(), "{url}");
        let identity = parse_untrusted_uri_units(&url.as_str().into(), "uri").unwrap();
        let UntrustedUriKind::Hierarchical {
            authority: UntrustedUriAuthority::Server { host, port, .. },
            ..
        } = identity.kind
        else {
            panic!("{url}")
        };
        assert_eq!(
            host,
            crate::JavaString::from(format!("[{address}]").to_ascii_lowercase())
        );
        assert_eq!(port, i32::MAX);
    }
    for address in [
        "",
        ":",
        "1:2:3:4:5:6:7",
        "1:2:3:4:5:6:7:8::",
        "1::2::3",
        "::ffff:256.2.3.4",
        "::ffff:000000000000256.2.3.4",
        "::ffff:1.2.3",
        "::ffff:1..2.3.4",
        "1.2.3.4::",
        "::1%",
        "::1%bad-zone",
    ] {
        let url = format!("http://[{address}]/");
        assert!(validate_url(&url).is_err(), "{url}");
    }
    for url in [
        "http://[::1]:2147483648",
        "http://[::1]:-1",
        "http://[::1]:bad",
        "http://[::1]x",
        "http://a@@[::1]",
    ] {
        assert!(validate_url(url).is_err(), "{url}");
    }
}

#[test]
fn decoded_session_mod_entity_type_does_not_leak_to_vanilla_or_other_sessions() {
    use crate::content_registry::{
        ContentRegistry, ContentRegistryBuilder, ModEntityRegistration, ModItemRegistration,
    };

    let mut builder = ContentRegistryBuilder::new();
    builder.register_entity(ModEntityRegistration {
        resource_id: "session_a_only:sky_drake".to_string(),
        ..Default::default()
    });
    builder.register_item(ModItemRegistration {
        resource_id: "session_a_only:sky_gem".to_string(),
        max_stack_size: 16,
        is_block_item: false,
    });
    let wire = builder.freeze_for_session().unwrap().encode_wire();
    let session_a = ContentRegistry::decode_wire(&wire).expect("session_a wire decode");

    let raw_entity_id: crate::JavaString = "session_a_only:sky_drake".into();
    assert_eq!(
        normalize_built_in_entity_type_id_with_registry(&raw_entity_id, &session_a),
        Ok("session_a_only:sky_drake".to_string())
    );
    assert_eq!(
        normalize_built_in_entity_type_id(&raw_entity_id),
        Err(BuiltInEntityIdError::UnknownEntityType)
    );
    assert_eq!(
        normalize_built_in_entity_type_id_with_registry(&raw_entity_id, ContentRegistry::vanilla()),
        Err(BuiltInEntityIdError::UnknownEntityType)
    );

    let hover_entity_component = compound([
        ("text", string("look")),
        (
            "hover_event",
            compound([
                ("action", string("show_entity")),
                ("id", string("session_a_only:sky_drake")),
                ("uuid", NbtSummaryValue::IntArray(vec![1, 2, 3, 4])),
            ]),
        ),
    ]);
    assert!(
        validate_component_nbt_with_registry(&hover_entity_component, "component", &session_a)
            .is_ok()
    );
    assert!(validate_component_nbt(&hover_entity_component, "component").is_err());

    let hover_item_component = compound([
        ("text", string("gem")),
        (
            "hover_event",
            compound([
                ("action", string("show_item")),
                ("id", string("session_a_only:sky_gem")),
                ("count", NbtSummaryValue::Int(2)),
            ]),
        ),
    ]);
    assert!(
        validate_component_nbt_with_registry(&hover_item_component, "component", &session_a)
            .is_ok()
    );
    assert!(validate_component_nbt(&hover_item_component, "component").is_err());
}
