use bbb_protocol::{
    packets::{
        DialogHolder, ServerLinkEntry, ServerLinkKnownType, ServerLinkType, ServerLinks, ShowDialog,
    },
    NbtSummaryEntry, NbtSummaryValue,
};

mod actions;
mod bodies;
mod inputs;
mod legacy_codes;
mod screens;
mod unattended;
mod utf16_hud;

use super::*;

#[derive(Default)]
struct MockDialogClipboard {
    text: Option<String>,
    rejects_write: bool,
}

impl DebugClipboard for MockDialogClipboard {
    fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
        if self.rejects_write {
            return false;
        }
        self.text = Some(text.to_string());
        true
    }

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        self.text.clone()
    }
}

fn custom_action_fields() -> Vec<NbtSummaryEntry> {
    vec![entry(
        "action",
        NbtSummaryValue::Compound(vec![
            entry("label", NbtSummaryValue::string("Send")),
            entry(
                "action",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::string("minecraft:custom")),
                    entry("id", NbtSummaryValue::string("example:test")),
                    entry("payload", NbtSummaryValue::Int(7)),
                ]),
            ),
        ]),
    )]
}

fn copy_action_fields(value: &str) -> Vec<NbtSummaryEntry> {
    vec![
        entry("pause", NbtSummaryValue::Byte(0)),
        entry("after_action", NbtSummaryValue::string("none")),
        entry(
            "action",
            NbtSummaryValue::Compound(vec![
                entry("label", NbtSummaryValue::string("Copy")),
                entry(
                    "action",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "type",
                            NbtSummaryValue::string("minecraft:copy_to_clipboard"),
                        ),
                        entry("value", NbtSummaryValue::string(value)),
                    ]),
                ),
            ]),
        ),
    ]
}

fn custom_button(
    label: &str,
    tooltip: Option<&str>,
    width: i32,
    id: &str,
    payload: Option<NbtSummaryValue>,
) -> NbtSummaryValue {
    let mut action = vec![
        entry("type", NbtSummaryValue::string("minecraft:custom")),
        entry("id", NbtSummaryValue::string(id)),
    ];
    if let Some(payload) = payload {
        action.push(entry("payload", payload));
    }
    let mut button = vec![
        entry("label", NbtSummaryValue::string(label)),
        entry("width", NbtSummaryValue::Int(width)),
        entry("action", NbtSummaryValue::Compound(action)),
    ];
    if let Some(tooltip) = tooltip {
        button.push(entry("tooltip", NbtSummaryValue::string(tooltip)));
    }
    NbtSummaryValue::Compound(button)
}

fn plain_message_body(contents: NbtSummaryValue, width: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("type", NbtSummaryValue::string("minecraft:plain_message")),
        entry("contents", contents),
        entry("width", NbtSummaryValue::Int(width)),
    ])
}

fn clickable_component(
    text: &str,
    action: &str,
    mut action_fields: Vec<NbtSummaryEntry>,
) -> NbtSummaryValue {
    let mut click_event = vec![entry("action", NbtSummaryValue::string(action))];
    click_event.append(&mut action_fields);
    NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string(text)),
        entry("click_event", NbtSummaryValue::Compound(click_event)),
    ])
}

fn boolean_input(
    key: &str,
    label: &str,
    initial: bool,
    on_true: &str,
    on_false: &str,
) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("key", NbtSummaryValue::string(key)),
        entry("type", NbtSummaryValue::string("minecraft:boolean")),
        entry("label", NbtSummaryValue::string(label)),
        entry("initial", NbtSummaryValue::Byte(i8::from(initial))),
        entry("on_true", NbtSummaryValue::string(on_true)),
        entry("on_false", NbtSummaryValue::string(on_false)),
    ])
}

#[allow(clippy::too_many_arguments)]
fn number_range_input(
    key: &str,
    label: &str,
    width: i32,
    start: f32,
    end: f32,
    initial: Option<f32>,
    step: Option<f32>,
) -> NbtSummaryValue {
    let mut fields = vec![
        entry("key", NbtSummaryValue::string(key)),
        entry("type", NbtSummaryValue::string("minecraft:number_range")),
        entry("width", NbtSummaryValue::Int(width)),
        entry("label", NbtSummaryValue::string(label)),
        entry("start", NbtSummaryValue::Float(start.to_bits())),
        entry("end", NbtSummaryValue::Float(end.to_bits())),
    ];
    if let Some(initial) = initial {
        fields.push(entry("initial", NbtSummaryValue::Float(initial.to_bits())));
    }
    if let Some(step) = step {
        fields.push(entry("step", NbtSummaryValue::Float(step.to_bits())));
    }
    NbtSummaryValue::Compound(fields)
}

fn single_option_input(
    key: &str,
    label: &str,
    width: i32,
    label_visible: bool,
    options: Vec<NbtSummaryValue>,
) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("key", NbtSummaryValue::string(key)),
        entry("type", NbtSummaryValue::string("minecraft:single_option")),
        entry("width", NbtSummaryValue::Int(width)),
        entry("options", NbtSummaryValue::List(options)),
        entry("label", NbtSummaryValue::string(label)),
        entry(
            "label_visible",
            NbtSummaryValue::Byte(i8::from(label_visible)),
        ),
    ])
}

fn text_input(
    key: &str,
    label: &str,
    width: i32,
    label_visible: bool,
    initial: impl Into<bbb_protocol::JavaString>,
    max_length: i32,
) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("key", NbtSummaryValue::string(key)),
        entry("type", NbtSummaryValue::string("minecraft:text")),
        entry("width", NbtSummaryValue::Int(width)),
        entry("label", NbtSummaryValue::string(label)),
        entry(
            "label_visible",
            NbtSummaryValue::Byte(i8::from(label_visible)),
        ),
        entry("initial", initial.into().to_nbt()),
        entry("max_length", NbtSummaryValue::Int(max_length)),
    ])
}

#[allow(clippy::too_many_arguments)]
fn multiline_text_input(
    key: &str,
    label: &str,
    width: i32,
    label_visible: bool,
    initial: impl Into<bbb_protocol::JavaString>,
    max_length: i32,
    max_lines: Option<i32>,
    height: Option<i32>,
) -> NbtSummaryValue {
    let mut multiline = Vec::new();
    if let Some(max_lines) = max_lines {
        multiline.push(entry("max_lines", NbtSummaryValue::Int(max_lines)));
    }
    if let Some(height) = height {
        multiline.push(entry("height", NbtSummaryValue::Int(height)));
    }
    let NbtSummaryValue::Compound(mut fields) =
        text_input(key, label, width, label_visible, initial, max_length)
    else {
        unreachable!("text input helper always returns a compound")
    };
    fields.push(entry("multiline", NbtSummaryValue::Compound(multiline)));
    NbtSummaryValue::Compound(fields)
}

fn world_with_notice(
    input: &mut crate::input::ClientInputState,
    fields: Vec<NbtSummaryEntry>,
) -> WorldStore {
    let mut world = WorldStore::new();
    apply_notice(input, &mut world, fields);
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world
}

fn apply_notice(
    input: &mut crate::input::ClientInputState,
    world: &mut WorldStore,
    mut fields: Vec<NbtSummaryEntry>,
) {
    let mut dialog = vec![
        entry("type", NbtSummaryValue::string("minecraft:notice")),
        entry("title", NbtSummaryValue::string("Notice")),
    ];
    dialog.append(&mut fields);
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(dialog),
        },
    });

    input.apply_screen_requests(world, &mut NetCounters::default(), &None);
}

fn world_with_confirmation(
    input: &mut crate::input::ClientInputState,
    mut fields: Vec<NbtSummaryEntry>,
) -> WorldStore {
    let mut dialog = vec![
        entry("type", NbtSummaryValue::string("minecraft:confirmation")),
        entry("title", NbtSummaryValue::string("Confirm")),
    ];
    dialog.append(&mut fields);
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(dialog),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world
}

fn world_with_multi_action(
    input: &mut crate::input::ClientInputState,
    mut fields: Vec<NbtSummaryEntry>,
) -> WorldStore {
    let mut dialog = vec![
        entry("type", NbtSummaryValue::string("minecraft:multi_action")),
        entry("title", NbtSummaryValue::string("Actions")),
    ];
    dialog.append(&mut fields);
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(dialog),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world
}

fn world_with_dialog_list(
    input: &mut crate::input::ClientInputState,
    mut fields: Vec<NbtSummaryEntry>,
    context_free: bool,
) -> WorldStore {
    let mut dialog = vec![
        entry("type", NbtSummaryValue::string("minecraft:dialog_list")),
        entry("title", NbtSummaryValue::string("Menu")),
    ];
    dialog.append(&mut fields);
    let dialog = NbtSummaryValue::Compound(dialog);
    let mut world = WorldStore::new();
    world.apply_show_dialog(ShowDialog {
        dialog: if context_free {
            DialogHolder::ContextFree { dialog }
        } else {
            DialogHolder::Direct { dialog }
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world
}

fn world_with_server_links_dialog(
    input: &mut crate::input::ClientInputState,
    links: Vec<ServerLinkEntry>,
    mut fields: Vec<NbtSummaryEntry>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_server_links(ServerLinks { links });
    let mut dialog = vec![
        entry("type", NbtSummaryValue::string("minecraft:server_links")),
        entry("title", NbtSummaryValue::string("Links")),
    ];
    dialog.append(&mut fields);
    world.apply_show_dialog(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: NbtSummaryValue::Compound(dialog),
        },
    });
    input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
    world
}

fn notice_dialog(title: &str, external_title: Option<&str>) -> NbtSummaryValue {
    let mut dialog = vec![
        entry("type", NbtSummaryValue::string("minecraft:notice")),
        entry("title", NbtSummaryValue::string(title)),
    ];
    if let Some(external_title) = external_title {
        dialog.push(entry(
            "external_title",
            NbtSummaryValue::string(external_title),
        ));
    }
    NbtSummaryValue::Compound(dialog)
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}
