use super::*;
use crate::control::NetCounters;
use bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X1_ID;
use bbb_protocol::{
    packets::{DialogHolder, OpenScreen as ProtocolOpenScreen, ShowDialog as ProtocolShowDialog},
    NbtSummaryEntry, NbtSummaryValue,
};
use std::time::Duration;
const DIALOG_REGISTRY: &str = "minecraft:dialog";
const DIALOG_WAITING_CLOSE_ACTIVE_TICKS: u32 = 101;
fn dialog_title(input: &ClientInputState) -> Option<&str> {
    match input.current_dialog()?.data.as_ref()? {
        DialogDataState::Notice { common, .. } => Some(&common.title.text),
        _ => None,
    }
}

fn show_notice(input: &mut ClientInputState, store: &mut WorldStore, title: &str) {
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::string("minecraft:notice"),
                },
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::string(title),
                },
            ]),
        },
    });
    input.apply_screen_requests(store, &mut NetCounters::default(), &None);
}

fn dialog_nbt(dialog_type: &str) -> Vec<u8> {
    let mut bytes = vec![10, 8, 0, 4, b't', b'y', b'p', b'e'];
    bytes.extend_from_slice(&(dialog_type.len() as u16).to_be_bytes());
    bytes.extend_from_slice(dialog_type.as_bytes());
    bytes.extend_from_slice(&[8, 0, 5, b't', b'i', b't', b'l', b'e', 0, 5]);
    bytes.extend_from_slice(b"Title");
    bytes.push(0);
    bytes
}

#[test]
fn dialog_warning_keeps_replaced_or_cleared_return_screen_hidden_until_back() {
    let mut input = ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let notice = |title: &str| ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string(title)),
            ]),
        },
    };
    let mut store = WorldStore::new();
    store.apply_show_dialog(notice("First"));
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(input.open_dialog_warning());
    assert!(input.dialog_warning_is_open());
    assert!(input.focus_dialog_warning_button(true));
    assert!(!input.open_dialog_warning());

    store.apply_show_dialog(notice("Second"));
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(input.dialog_warning_is_open());
    let Some(DialogDataState::Notice { common, .. }) = input
        .current_dialog()
        .and_then(|dialog| dialog.data.as_ref())
    else {
        panic!("expected replacement notice");
    };
    assert_eq!(common.title.text, "Second");
    assert_eq!(input.dialog_warning_focused_button(), Some(true));

    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(input.dialog_warning_is_open());
    assert!(input.current_dialog().is_none());
    assert!(input.dialog_screen_is_open());
    assert!(input.close_dialog_warning());
    assert!(!input.dialog_warning_is_open());
    assert!(!input.dialog_screen_is_open());
    assert!(!input.close_dialog_warning());

    show_notice(&mut input, &mut store, "Link parent");
    assert!(input.open_dialog_url("https://example.invalid/parent".into(), "none".into()));
    show_notice(&mut input, &mut store, "Child");
    assert!(input.open_dialog_warning());
    assert!(input.focus_dialog_warning_button(false));
    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(input.dialog_link_confirmation().is_some());
    assert!(input.dialog_warning_is_open());
    assert!(input.resolve_dialog_link_confirmation(false));
    assert_eq!(input.dialog_warning_focused_button(), Some(false));
    assert!(input.close_dialog_warning());
    assert_eq!(dialog_title(&input), Some("Link parent"));
}

#[test]
fn late_clear_dialog_keeps_the_screen_that_replaced_it() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 1 },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(store.open_local_inventory());
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(input.current_dialog(), None);

    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(store.local_inventory_is_open());

    let mut container_store = WorldStore::new();
    let mut container_input = ClientInputState::new(true);
    container_store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 2 },
    });
    container_input.apply_screen_requests(&mut container_store, &mut NetCounters::default(), &None);
    container_store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Container"),
    });
    container_input.apply_screen_requests(&mut container_store, &mut NetCounters::default(), &None);
    assert_eq!(container_input.current_dialog(), None);
    container_store.apply_clear_dialog();
    container_input.apply_screen_requests(&mut container_store, &mut NetCounters::default(), &None);
    assert_eq!(container_store.open_container_id(), Some(7));
}

#[test]
fn dialog_list_entry_opens_registry_target_without_counting_a_server_packet() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    store.record_registry_entries(
        DIALOG_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::with_raw_data(
            "example:target",
            dialog_nbt("minecraft:notice"),
        )],
    );
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::string("minecraft:dialog_list"),
                },
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::string("Menu"),
                },
                NbtSummaryEntry {
                    name: "dialogs".into(),
                    value: NbtSummaryValue::string("example:target"),
                },
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(input.focus_dialog_navigation_target(
        &store,
        DialogFocusState::Action { index: 0 },
        false
    ));
    assert!(!input.show_dialog_list_entry(&store, 1));

    assert!(input.show_dialog_list_entry(&store, 0));
    let target = input.current_dialog().unwrap();
    assert_eq!(target.holder_kind, "reference");
    assert_eq!(target.registry_id, Some(0));
    assert_eq!(target.resource_id.as_deref(), Some("example:target"));
    assert!(matches!(
        target.data,
        Some(bbb_client_world::DialogDataState::Notice { ref common, .. }) if common.title.text == "Title"
    ));
    assert_eq!(store.counters().show_dialog_packets, 1);
    assert_eq!(input.dialog_focus(), None);
}

#[test]
fn dialog_screen_revision_tracks_concrete_screen_replacements() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    assert_eq!(input.dialog_screen_revision(), 0);
    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(input.dialog_screen_revision(), 0);

    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 1 },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(input.dialog_screen_revision(), 1);
    assert!(input.open_dialog_warning());
    assert_eq!(input.dialog_screen_revision(), 2);
    assert!(!input.open_dialog_warning());
    assert_eq!(input.dialog_screen_revision(), 2);
    assert!(input.close_dialog_warning());
    assert_eq!(input.dialog_screen_revision(), 3);
    assert!(!input.close_dialog_warning());
    assert_eq!(input.dialog_screen_revision(), 3);

    assert!(input.wait_for_dialog_response());
    assert_eq!(input.dialog_screen_revision(), 4);
    input.advance_dialog_waiting_ticks(DIALOG_WAITING_CLOSE_ACTIVE_TICKS);
    assert!(input.close_dialog_waiting_response());
    assert_eq!(input.dialog_screen_revision(), 5);
    assert!(!input.close_dialog_waiting_response());
    assert_eq!(input.dialog_screen_revision(), 5);
}

#[test]
fn dialog_open_url_prompt_owns_clear_and_cancel_lifecycle() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    show_notice(&mut input, &mut store, "Links");
    let revision = input.dialog_screen_revision();

    assert!(input.open_dialog_url("https://example.invalid/docs".into(), "none".to_string(),));
    assert_eq!(input.dialog_screen_revision(), revision + 1);
    assert_eq!(
        input.dialog_link_confirmation(),
        Some(&DialogLinkConfirmationState {
            focused_button: None,
            url: "https://example.invalid/docs".into(),
            after_action: "none".to_string(),
        })
    );
    assert!(input.focus_dialog_link_button(1));

    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(store.counters().clear_dialog_packets, 1);
    assert!(input.current_dialog().is_some());
    assert!(input.dialog_link_confirmation().is_some());
    assert_eq!(input.dialog_link_focused_button(), Some(1));
    assert_eq!(input.dialog_screen_revision(), revision + 1);

    assert!(input.resolve_dialog_link_confirmation(false));
    assert_eq!(input.dialog_screen_revision(), revision + 2);
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_link_confirmation(), None);
    assert!(input.take_dialog_open_url_requests().is_empty());
}

#[test]
fn chat_open_url_uses_link_options_without_active_dialog() {
    let mut input = ClientInputState::new(true);
    assert!(input.open_chat_url("https://example.invalid/chat".into()));
    assert_eq!(
        input.dialog_link_confirmation(),
        Some(&DialogLinkConfirmationState {
            focused_button: None,
            url: "https://example.invalid/chat".into(),
            after_action: "none".to_string(),
        })
    );
    assert!(input.resolve_dialog_link_confirmation(true));
    assert_eq!(
        input.take_dialog_open_url_requests(),
        ["https://example.invalid/chat"]
    );

    let mut disabled = ClientInputState::new(true);
    disabled.set_dialog_link_options(false, true);
    assert!(!disabled.open_chat_url("https://example.invalid/blocked".into()));
    assert!(disabled.dialog_link_confirmation().is_none());
}

#[test]
fn dialog_open_url_resolution_applies_deferred_after_action() {
    let mut close_store = WorldStore::new();
    let mut close_input = ClientInputState::new(true);
    show_notice(&mut close_input, &mut close_store, "Close");
    assert!(
        close_input.open_dialog_url("https://example.invalid/close".into(), "close".to_string(),)
    );
    assert!(close_input.resolve_dialog_link_confirmation(true));
    assert!(close_input.current_dialog().is_none());
    assert_eq!(
        close_input.take_dialog_open_url_requests(),
        ["https://example.invalid/close"]
    );
    assert_eq!(close_store.counters().show_dialog_packets, 1);
    assert_eq!(close_store.counters().clear_dialog_packets, 0);

    let mut wait_store = WorldStore::new();
    let mut wait_input = ClientInputState::new(true);
    show_notice(&mut wait_input, &mut wait_store, "Wait");
    assert!(wait_input.open_dialog_url(
        "https://example.invalid/wait".into(),
        "wait_for_response".to_string(),
    ));
    assert!(wait_input.resolve_dialog_link_confirmation(false));
    assert!(wait_input.current_dialog().is_none());
    assert_eq!(wait_input.dialog_waiting_ticks(), Some(0));
    assert!(wait_input.take_dialog_open_url_requests().is_empty());
}

#[test]
fn server_dialogs_preserve_nested_confirm_link_return_screens() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    show_notice(&mut input, &mut store, "First");
    assert!(input.open_dialog_url("https://example.invalid/first".into(), "none".to_string(),));
    assert!(input.focus_dialog_link_button(1));

    show_notice(&mut input, &mut store, "Second");
    assert_eq!(input.dialog_link_confirmation(), None);
    assert_eq!(dialog_title(&input), Some("Second"));
    assert!(input.open_dialog_url("https://example.invalid/second".into(), "none".to_string(),));
    assert!(input.focus_dialog_link_button(2));

    show_notice(&mut input, &mut store, "Third");
    assert_eq!(input.dialog_link_confirmation(), None);
    assert_eq!(dialog_title(&input), Some("Third"));
    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(dialog_title(&input), Some("Second"));
    assert_eq!(
        input.dialog_link_confirmation(),
        Some(&DialogLinkConfirmationState {
            focused_button: Some(2),
            url: "https://example.invalid/second".into(),
            after_action: "none".to_string(),
        })
    );
    assert_eq!(input.dialog_link_focused_button(), Some(2));
    assert!(input.resolve_dialog_link_confirmation(false));

    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(dialog_title(&input), Some("First"));
    assert_eq!(
        input.dialog_link_confirmation(),
        Some(&DialogLinkConfirmationState {
            focused_button: Some(1),
            url: "https://example.invalid/first".into(),
            after_action: "none".to_string(),
        })
    );
    assert_eq!(input.dialog_link_focused_button(), Some(1));
    assert!(input.resolve_dialog_link_confirmation(false));
    assert_eq!(dialog_title(&input), Some("First"));
    assert_eq!(store.counters().show_dialog_packets, 3);
    assert_eq!(store.counters().clear_dialog_packets, 2);
}

#[test]
fn waiting_screen_returns_to_suspended_confirm_link_screen() {
    let mut store = WorldStore::new();
    let mut input = ClientInputState::new(true);
    show_notice(&mut input, &mut store, "First");
    assert!(input.open_dialog_url("https://example.invalid/first".into(), "none".to_string(),));
    show_notice(&mut input, &mut store, "Second");

    assert!(input.wait_for_dialog_response());
    let projected =
        input.dialog_control_projection(serde_json::to_value(store.client_ui()).unwrap());
    assert_eq!(projected["current_dialog"], serde_json::Value::Null);
    assert_eq!(projected["dialog_waiting_ticks"], 0);
    assert!(projected.get("dialog_input_values").is_none());
    assert!(projected.get("dialog_link_confirmation").is_none());
    assert_eq!(input.dialog_waiting_ticks(), Some(0));
    assert_eq!(input.dialog_link_confirmation(), None);
    input.advance_dialog_waiting_ticks(DIALOG_WAITING_CLOSE_ACTIVE_TICKS);
    assert!(input.close_dialog_waiting_response());

    assert_eq!(dialog_title(&input), Some("First"));
    let projected =
        input.dialog_control_projection(serde_json::to_value(store.client_ui()).unwrap());
    assert_eq!(
        projected["dialog_link_confirmation"]["url"],
        "https://example.invalid/first"
    );
    assert!(projected["current_dialog"].is_object());
    assert!(projected.get("dialog_waiting_ticks").is_none());
    assert_eq!(
        input.dialog_link_confirmation(),
        Some(&DialogLinkConfirmationState {
            focused_button: None,
            url: "https://example.invalid/first".into(),
            after_action: "none".to_string(),
        })
    );
}

#[test]
fn dialog_control_projection_displays_raw_profile_and_text_without_mutation() {
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let text = |value: &str| NbtSummaryValue::String(value.into());
    let raw = bbb_protocol::JavaString::from(vec![0xd800, b'x' as u16]);
    let profile = NbtSummaryValue::Compound(vec![entry(
        "properties",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry("name", text("custom")),
            entry("value", raw.to_nbt()),
        ])]),
    )]);
    let item = NbtSummaryValue::Compound(vec![
        entry("id", text("minecraft:player_head")),
        entry(
            "components",
            NbtSummaryValue::Compound(vec![entry("minecraft:profile", profile)]),
        ),
    ]);
    let mut store = WorldStore::new();
    let mut input = ClientInputState::new(true);
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", text("minecraft:notice")),
                entry("title", text("Raw projection")),
                entry(
                    "body",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("type", text("item")),
                        entry("item", item),
                    ])]),
                ),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("key", text("raw")),
                        entry("type", text("text")),
                        entry("label", text("Raw input")),
                        entry("initial", raw.to_nbt()),
                    ])]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    let dialog = input.current_dialog().unwrap().clone();
    assert!(dialog.semantic_error.is_none(), "{dialog:?}");
    let projected = input.dialog_control_projection(serde_json::json!({}));
    assert_eq!(
        projected.pointer("/current_dialog/data/common/body_data/0/item/component_data/entries/0/value_data/profile/properties/0/value"),
        Some(&serde_json::json!("�x"))
    );
    assert_eq!(projected["dialog_input_values"][0]["value"], "�x");
    assert_eq!(input.current_dialog(), Some(&dialog));
    assert_eq!(
        input.current_dialog_input_values(),
        &[Some(DialogInputValueState::Text { value: raw.clone() })]
    );
    assert!(input.open_dialog_url(raw.clone(), "none".into()));
    let projected = input.dialog_control_projection(serde_json::json!({}));
    assert_eq!(
        projected["dialog_link_confirmation"],
        serde_json::json!({"url": "�x", "after_action": "none"})
    );
    assert_eq!(input.dialog_link_confirmation().unwrap().url, raw);
}

#[test]
fn dialog_link_options_match_vanilla_disabled_and_no_prompt_modes() {
    let mut store = WorldStore::new();
    let mut input = ClientInputState::new(true);
    show_notice(&mut input, &mut store, "Options");
    let revision = input.dialog_screen_revision();

    input.set_dialog_link_options(false, true);
    assert!(!input.open_dialog_url(
        "https://example.invalid/disabled".into(),
        "close".to_string(),
    ));
    assert_eq!(input.dialog_link_confirmation(), None);
    assert!(input.take_dialog_open_url_requests().is_empty());

    input.set_dialog_link_options(true, false);
    assert!(input.open_dialog_url("https://example.invalid/direct".into(), "close".to_string(),));
    assert_eq!(input.dialog_screen_revision(), revision);
    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_link_confirmation(), None);
    assert_eq!(
        input.take_dialog_open_url_requests(),
        ["https://example.invalid/direct"]
    );
    assert!(input.take_dialog_open_url_requests().is_empty());
}

#[test]
fn dialog_boolean_input_values_initialize_toggle_and_clear_with_screen() {
    let mut input = ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Inputs")),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Compound(vec![
                            entry("key", NbtSummaryValue::string("enabled")),
                            entry("type", NbtSummaryValue::string("boolean")),
                            entry("label", NbtSummaryValue::string("Enabled")),
                            entry("initial", NbtSummaryValue::Byte(1)),
                        ]),
                        NbtSummaryValue::Compound(vec![
                            entry("key", NbtSummaryValue::string("name")),
                            entry("type", NbtSummaryValue::string("text")),
                            entry("label", NbtSummaryValue::string("Name")),
                        ]),
                    ]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert_eq!(
        input.current_dialog_input_values(),
        [
            Some(DialogInputValueState::Boolean { selected: true }),
            Some(DialogInputValueState::Text {
                value: String::new().into(),
            }),
        ]
    );
    assert!(input.toggle_dialog_boolean_input(0));
    assert!(!input.toggle_dialog_boolean_input(1));
    assert_eq!(
        input.current_dialog_input_values(),
        [
            Some(DialogInputValueState::Boolean { selected: false }),
            Some(DialogInputValueState::Text {
                value: String::new().into(),
            }),
        ]
    );

    assert!(input.close_dialog());
    assert!(input.current_dialog_input_values().is_empty());
}

#[test]
fn dialog_body_scroll_state_clamps_drags_and_resets_with_lifecycle() {
    let mut input = ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut store = WorldStore::new();
    show_notice(&mut input, &mut store, "Initial");
    assert!(input.set_dialog_body_scroll_amount(15.0, 10.0));
    assert_eq!(input.dialog_body_scroll_amount(), 10.0);
    input.begin_dialog_body_scroll_drag(40);
    assert_eq!(input.update_dialog_body_scroll_drag_y(47), Some(7));
    assert!(input.end_dialog_body_scroll_drag());
    assert!(!input.end_dialog_body_scroll_drag());
    assert!(input.set_dialog_body_scroll_amount(f64::NAN, 10.0));
    assert_eq!(input.dialog_body_scroll_amount(), 0.0);

    input.set_dialog_body_scroll_amount(8.0, 10.0);
    input.begin_dialog_body_scroll_drag(12);
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Scroll")),
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(input.dialog_body_scroll_amount(), 0.0);
    assert_eq!(input.dialog_body_scroll_drag_y(), None);

    input.set_dialog_body_scroll_amount(6.0, 10.0);
    input.begin_dialog_body_scroll_drag(20);
    assert!(input.close_dialog());
    assert_eq!(input.dialog_body_scroll_amount(), 0.0);
    assert_eq!(input.dialog_body_scroll_drag_y(), None);
}

#[test]
fn dialog_item_body_focus_switches_with_inputs_and_clears_with_lifecycle() {
    let mut input = ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Focus")),
                entry(
                    "body",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::string("minecraft:item")),
                        entry(
                            "item",
                            NbtSummaryValue::Compound(vec![entry(
                                "id",
                                NbtSummaryValue::string("minecraft:stick"),
                            )]),
                        ),
                    ]),
                ),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("key", NbtSummaryValue::string("enabled")),
                        entry("type", NbtSummaryValue::string("minecraft:boolean")),
                        entry("label", NbtSummaryValue::string("Enabled")),
                    ])]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert!(input.focus_dialog_body_item(0));
    assert_eq!(input.dialog_focused_body(), Some(0));
    assert_eq!(input.dialog_focused_input(), None);
    assert!(!input.focus_dialog_body_item(1));
    assert_eq!(input.dialog_focused_body(), Some(0));

    assert!(input.focus_dialog_mouse_input(0));
    assert_eq!(input.dialog_focused_body(), None);
    assert_eq!(input.dialog_focused_input(), Some(0));
    assert!(input.focus_dialog_body_item(0));
    assert_eq!(input.dialog_focused_body(), Some(0));
    assert_eq!(input.dialog_focused_input(), None);

    assert!(input.close_dialog());
    assert_eq!(input.dialog_focused_body(), None);
    assert_eq!(input.dialog_focused_input(), None);
}

#[test]
fn dialog_number_range_value_initializes_and_drag_state_tracks_dialog_lifecycle() {
    let mut input = ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Inputs")),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("key", NbtSummaryValue::string("speed")),
                        entry("type", NbtSummaryValue::string("number_range")),
                        entry("label", NbtSummaryValue::string("Speed")),
                        entry("start", NbtSummaryValue::Float(10.0f32.to_bits())),
                        entry("end", NbtSummaryValue::Float(0.0f32.to_bits())),
                        entry("initial", NbtSummaryValue::Float(7.5f32.to_bits())),
                    ])]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::NumberRange {
            slider_value_bits: 0.25f64.to_bits(),
        })]
    );
    assert!(input.begin_dialog_number_range_drag(0));
    assert_eq!(input.dialog_dragging_number_range_input(), Some(0));
    assert!(input.focus_dialog_mouse_input(0));
    assert_eq!(input.dialog_focused_input(), Some(0));
    assert!(input.dialog_number_range_keyboard_can_change(0));
    assert!(input.toggle_dialog_number_range_keyboard_change(0));
    assert!(!input.dialog_number_range_keyboard_can_change(0));
    assert!(input.set_dialog_number_range_slider_value(0, 2.0));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::NumberRange {
            slider_value_bits: 1.0f64.to_bits(),
        })]
    );

    assert!(input.close_dialog());
    assert_eq!(input.dialog_dragging_number_range_input(), None);
    assert_eq!(input.dialog_focused_input(), None);
    assert!(input.current_dialog_input_values().is_empty());
}

#[test]
fn dialog_single_option_uses_marked_initial_and_updates_selected_index() {
    let mut input = ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Inputs")),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("key", NbtSummaryValue::string("mode")),
                        entry("type", NbtSummaryValue::string("single_option")),
                        entry("label", NbtSummaryValue::string("Mode")),
                        entry(
                            "options",
                            NbtSummaryValue::List(vec![
                                NbtSummaryValue::string("first"),
                                NbtSummaryValue::Compound(vec![
                                    entry("id", NbtSummaryValue::string("second")),
                                    entry("initial", NbtSummaryValue::Byte(1)),
                                ]),
                            ]),
                        ),
                    ])]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 1,
        })]
    );
    assert!(input.set_dialog_single_option_index(0, 0));
    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::SingleOption {
            selected_index: 0,
        })]
    );
}

#[test]
fn dialog_single_line_text_tracks_utf16_bounded_value_selection_and_drag() {
    let mut input = ClientInputState::new(true);
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:notice")),
                entry("title", NbtSummaryValue::string("Inputs")),
                entry(
                    "inputs",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("key", NbtSummaryValue::string("name")),
                        entry("type", NbtSummaryValue::string("text")),
                        entry("label", NbtSummaryValue::string("Name")),
                        entry("initial", NbtSummaryValue::string("A😀")),
                        entry("max_length", NbtSummaryValue::Int(4)),
                    ])]),
                ),
            ]),
        },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert_eq!(
        input.current_dialog_input_values(),
        [Some(DialogInputValueState::Text {
            value: "A😀".into(),
        })]
    );
    assert!(input.focus_dialog_text_input(0, 1, false));
    assert_eq!(input.dialog_focused_input(), Some(0));
    assert!(input.focus_dialog_text_input(0, 2, true));
    assert_eq!(
        input.dialog_text_edit_state(),
        Some(DialogTextEditState {
            input_index: 0,
            cursor: 2,
            selection: 1,
            scroll_amount: 0,
            ..DialogTextEditState::default()
        })
    );
    assert!(input.begin_dialog_text_input_drag(0));
    assert_eq!(input.dialog_dragging_text_input(), Some(0));
    assert!(!input.set_dialog_text_input_value_and_selection(0, "A😀BC".to_string(), 4, 4,));
    assert!(input.set_dialog_text_input_value_and_selection(0, "A😀B".to_string(), 3, 3,));
    assert_eq!(input.end_dialog_text_input_drag(), Some(0));

    assert!(input.close_dialog());
    assert_eq!(input.dialog_text_edit_state(), None);
    assert_eq!(input.dialog_dragging_text_input(), None);
    assert!(input.current_dialog_input_values().is_empty());
}

#[test]
fn local_dialog_close_counts_no_packet() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 3 },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert!(input.close_dialog());
    assert_eq!(store.counters().clear_dialog_packets, 0);
    assert!(!input.close_dialog());
    assert_eq!(store.counters().clear_dialog_packets, 0);
}

#[test]
fn waiting_dialog_ignores_clear_packet() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 3 },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert!(input.wait_for_dialog_response());
    assert_eq!(input.current_dialog(), None);
    assert_eq!(input.dialog_waiting_ticks(), Some(0));
    store.apply_clear_dialog();
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert_eq!(input.dialog_waiting_ticks(), Some(0));
    assert_eq!(store.counters().clear_dialog_packets, 1);

    input.advance_dialog_waiting_ticks(100);
    assert!(!input.dialog_waiting_can_close());
    assert!(!input.close_dialog_waiting_response());
    input.advance_dialog_waiting_ticks(1);
    assert!(input.dialog_waiting_can_close());
    assert!(input.close_dialog_waiting_response());
    assert!(!input.dialog_screen_is_open());
}

#[test]
fn show_dialog_replaces_waiting_screen() {
    let mut input = ClientInputState::new(true);
    let mut store = WorldStore::new();
    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 3 },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);
    assert!(input.wait_for_dialog_response());

    store.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Reference { registry_id: 4 },
    });
    input.apply_screen_requests(&mut store, &mut NetCounters::default(), &None);

    assert!(input.current_dialog().is_some());
    assert_eq!(input.dialog_waiting_ticks(), None);
    assert!(input.close_dialog());
    assert!(!input.dialog_screen_is_open());
}

#[test]
fn container_close_distinguishes_server_screen_close_from_local_container_close() {
    for server_packet in [true, false] {
        let mut world = WorldStore::new();
        let mut input = ClientInputState::new(true);
        world.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
            title: bbb_protocol::RichTextComponent::plain("Container"),
        });
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        show_notice(&mut input, &mut world, "Notice");
        if server_packet {
            assert!(world
                .apply_container_close(bbb_protocol::packets::ContainerClose { container_id: 7 }));
        } else {
            assert!(world.close_local_container(7));
        }
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        assert!(world.open_container_id().is_none());
        if server_packet {
            assert!(input.current_screen().is_none());
            assert!(!input.close_dialog());
        } else {
            assert_eq!(dialog_title(&input), Some("Notice"));
            assert!(input.close_dialog());
            assert!(input.current_screen().is_none());
        }
    }
}

fn open_test_dialog(input: &mut ClientInputState) {
    let mut world = WorldStore::new();
    world.apply_show_dialog(bbb_protocol::packets::ShowDialog {
        dialog: bbb_protocol::packets::DialogHolder::Reference { registry_id: 0 },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
}

#[test]
fn dialog_double_click_timing_requires_same_screen_button_and_strict_threshold() {
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    open_test_dialog(&mut input);
    assert!(!input.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start,
    ));
    assert!(input.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start + DIALOG_DOUBLE_CLICK_THRESHOLD - Duration::from_millis(1),
    ));

    let mut exact = ClientInputState::new(true);
    open_test_dialog(&mut exact);
    assert!(!exact.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start,
    ));
    assert!(!exact.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start + DIALOG_DOUBLE_CLICK_THRESHOLD,
    ));

    let mut identity = ClientInputState::new(true);
    open_test_dialog(&mut identity);
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Pressed,
        7,
        start,
    ));
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Left,
        ElementState::Released,
        7,
        start + Duration::from_millis(50),
    ));
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Right,
        ElementState::Pressed,
        7,
        start + Duration::from_millis(100),
    ));
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Right,
        ElementState::Pressed,
        8,
        start + Duration::from_millis(150),
    ));
    identity.clear_pressed();
    assert!(!identity.dialog_mouse_click_is_double_at(
        MouseButton::Right,
        ElementState::Pressed,
        8,
        start + Duration::from_millis(200),
    ));
}

#[test]
fn dialog_ime_preedit_maps_utf8_cursor_ranges_and_blinks_at_vanilla_period() {
    let start = Instant::now();
    let mut input = ClientInputState::new(true);
    open_test_dialog(&mut input);

    assert!(input.handle_dialog_ime_preedit_at("a\u{0007}中b", Some((2, 5)), 7, start,));
    assert_eq!(
        input.dialog_ime_preedit_hud_state_at(start + Duration::from_millis(299)),
        Some(DialogImePreeditHudState {
            text: "a中b".to_string(),
            caret: Some(2),
            focused_start: 1,
            focused_end: 2,
            screen_revision: 7,
            caret_visible: true,
        })
    );
    assert!(
        !input
            .dialog_ime_preedit_hud_state_at(start + Duration::from_millis(300))
            .unwrap()
            .caret_visible
    );
    assert!(
        input
            .dialog_ime_preedit_hud_state_at(start + Duration::from_millis(600))
            .unwrap()
            .caret_visible
    );

    assert!(input.handle_dialog_ime_preedit_at("compose", Some((3, 3)), 8, start));
    let point_cursor = input.dialog_ime_preedit_hud_state_at(start).unwrap();
    assert_eq!(point_cursor.caret, Some(3));
    assert_eq!(
        (point_cursor.focused_start, point_cursor.focused_end),
        (0, 7)
    );

    assert!(input.handle_dialog_ime_preedit_at("compose", None, 8, start));
    let hidden_cursor = input.dialog_ime_preedit_hud_state_at(start).unwrap();
    assert_eq!(hidden_cursor.caret, None);
    assert_eq!(
        (hidden_cursor.focused_start, hidden_cursor.focused_end),
        (0, 7)
    );

    // Vanilla `KeyMapping.releaseAll` leaves the composition alone; only the
    // text-input focus handoff (or a newer IME event) clears it.
    input.clear_pressed();
    assert!(input.dialog_ime_preedit_hud_state_at(start).is_some());
    input.clear_dialog_ime_preedit();
    assert_eq!(input.dialog_ime_preedit_hud_state_at(start), None);
    assert!(input.handle_dialog_ime_preedit_at("", None, 8, start));
    assert_eq!(input.dialog_ime_preedit_hud_state_at(start), None);
}

#[test]
fn show_dialog_covering_advancements_screen_queues_closed_screen_packet() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    let commands = Some(tx);
    let mut world = WorldStore::new();
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();

    input.open_advancements_screen(&mut counters, &mut world, &commands, false);
    assert!(input.advancements_screen_is_open());

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::string("minecraft:notice"),
                },
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::string("Notice"),
                },
            ]),
        },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(input.dialog_screen_is_open());
    assert_eq!(
        rx.try_recv().unwrap(),
        bbb_client_net::NetCommand::SeenAdvancements(
            bbb_protocol::packets::SeenAdvancements::ClosedScreen
        )
    );
    assert!(rx.try_recv().is_err());

    world.apply_show_dialog(ProtocolShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::string("minecraft:notice"),
                },
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::string("Replacement"),
                },
            ]),
        },
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert!(rx.try_recv().is_err());

    input.close_screens(&mut counters, &commands);
    assert!(rx.try_recv().is_err());
}
