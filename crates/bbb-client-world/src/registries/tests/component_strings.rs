use super::*;
use bbb_protocol::{packets::ConfigurationClientbound, JavaString};
use bbb_registry_codec::registry_codec::decode_exact_synchronized_registry_element;

const TRIM: &str = "minecraft:trim_material";
const DIALOG: &str = "minecraft:dialog";

#[derive(Clone, Copy, Debug)]
enum Source {
    Network,
    ServerJson,
    KnownPack,
}
impl Source {
    fn kind(self) -> RegistryElementSource {
        match self {
            Self::Network => RegistryElementSource::NetworkNbt,
            Self::ServerJson => RegistryElementSource::ServerDatapackJson,
            Self::KnownPack => RegistryElementSource::ClientKnownPackJson,
        }
    }
}
struct Text {
    units: Vec<u16>,
    json: &'static str,
}
impl Text {
    fn value(&self, prefix: &str, suffix: &str) -> JavaString {
        prefix
            .encode_utf16()
            .chain(self.units.iter().copied())
            .chain(suffix.encode_utf16())
            .collect::<Vec<_>>()
            .into()
    }
    fn quoted(&self, prefix: &str, suffix: &str) -> String {
        format!("\"{prefix}{}{suffix}\"", &self.json[1..self.json.len() - 1])
    }
}
fn corpus() -> Vec<Text> {
    vec![
        Text {
            units: vec![0xd800],
            json: r#""\ud800""#,
        },
        Text {
            units: vec![0xdc00],
            json: r#""\udc00""#,
        },
        Text {
            units: vec![0xd83d, 0xde00],
            json: r#""\ud83d\ude00""#,
        },
        Text {
            units: vec![0x61, 0xd800, 0x62, 0xdc00],
            json: r#""a\ud800b\udc00""#,
        },
    ]
}
fn compound(fields: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        fields
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}
fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}
fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    let NbtSummaryValue::Compound(fields) = value else {
        panic!("expected map field {name}")
    };
    &fields
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

fn trim_json(text: &Text) -> String {
    format!(
        r#"{{"asset_name":"quartz","description":{{"text":{},"extra":[{{"text":"url","click_event":{{"action":"open_url","url":{}}}}},{{"text":"copy","click_event":{{"action":"copy_to_clipboard","value":{}}}}},{{"text":"run","click_event":{{"action":"run_command","command":{}}}}},{{"text":"suggest","click_event":{{"action":"suggest_command","command":{}}}}}]}}}}"#,
        text.json,
        text.quoted("https://example.invalid/a", ""),
        text.json,
        text.quoted("/say ", ""),
        text.json
    )
}
fn description_nbt(text: &Text) -> NbtSummaryValue {
    let raw = text.value("", "").to_nbt();
    let extra = [
        (
            "url",
            "open_url",
            "url",
            text.value("https://example.invalid/a", "").to_nbt(),
        ),
        ("copy", "copy_to_clipboard", "value", raw.clone()),
        (
            "run",
            "run_command",
            "command",
            text.value("/say ", "").to_nbt(),
        ),
        ("suggest", "suggest_command", "command", raw.clone()),
    ]
    .into_iter()
    .map(|(label, action, name, value)| {
        compound(vec![
            ("text", string(label)),
            (
                "click_event",
                compound(vec![("action", string(action)), (name, value)]),
            ),
        ])
    })
    .collect();
    compound(vec![("text", raw), ("extra", NbtSummaryValue::List(extra))])
}
fn dialog_json(text: &Text) -> String {
    format!(
        r#"{{"type":"multi_action","pause":false,"columns":3,"title":{raw},"external_title":{raw},"body":{{"type":"plain_message","contents":{raw},"width":240}},"inputs":[{{"key":"boolean","type":"boolean","label":{raw},"initial":false,"on_true":{raw},"on_false":{raw}}},{{"key":"number","type":"number_range","label":{raw},"label_format":{raw},"start":0,"end":1}},{{"key":"option","type":"single_option","label":{raw},"options":[{raw},{{"id":{raw},"display":{raw}}}]}},{{"key":"text","type":"text","label":{raw},"initial":{raw}}}],"actions":[{{"label":{raw},"tooltip":{raw},"action":{{"type":"open_url","url":{url}}}}},{{"label":{raw},"action":{{"type":"copy_to_clipboard","value":{raw}}}}},{{"label":{raw},"action":{{"type":"run_command","command":{run}}}}},{{"label":{raw},"action":{{"type":"suggest_command","command":{raw}}}}},{{"label":{raw},"action":{{"type":"dynamic/run_command","template":{template}}}}},{{"label":{raw},"action":{{"type":"dynamic/custom","id":"example:submit","additions":{{"raw":{raw}}}}}}}]}}"#,
        raw = text.json,
        url = text.quoted("https://example.invalid/a", ""),
        run = text.quoted("/say ", ""),
        template = text.quoted("say ", " $(text)")
    )
}
fn dialog_nbt(text: &Text) -> NbtSummaryValue {
    let raw = text.value("", "").to_nbt();
    let input = |key: &str, fields: Vec<(&str, NbtSummaryValue)>, kind: &str| {
        let mut fields_with_key = vec![("key", string(key))];
        fields_with_key.extend(fields);
        fields_with_key.push(("type", string(&format!("minecraft:{kind}"))));
        compound(fields_with_key)
    };
    let inputs = vec![
        input(
            "boolean",
            vec![
                ("label", raw.clone()),
                ("on_true", raw.clone()),
                ("on_false", raw.clone()),
            ],
            "boolean",
        ),
        input(
            "number",
            vec![
                ("label", raw.clone()),
                ("label_format", raw.clone()),
                ("start", NbtSummaryValue::Float(0.0_f32.to_bits())),
                ("end", NbtSummaryValue::Float(1.0_f32.to_bits())),
            ],
            "number_range",
        ),
        input(
            "option",
            vec![
                (
                    "options",
                    NbtSummaryValue::List(vec![
                        compound(vec![("id", raw.clone())]),
                        compound(vec![("id", raw.clone()), ("display", raw.clone())]),
                    ]),
                ),
                ("label", raw.clone()),
            ],
            "single_option",
        ),
        input(
            "text",
            vec![("label", raw.clone()), ("initial", raw.clone())],
            "text",
        ),
    ];
    let button = |mut fields: Vec<(&str, NbtSummaryValue)>, kind: &str, tooltip: bool| {
        fields.push(("type", string(&format!("minecraft:{kind}"))));
        let mut fields_with_label = vec![("label", raw.clone())];
        if tooltip {
            fields_with_label.push(("tooltip", raw.clone()));
        }
        fields_with_label.push(("action", compound(fields)));
        compound(fields_with_label)
    };
    compound(vec![
        ("title", raw.clone()),
        ("external_title", raw.clone()),
        ("pause", NbtSummaryValue::Byte(0)),
        ("columns", NbtSummaryValue::Int(3)),
        (
            "body",
            compound(vec![
                ("contents", raw.clone()),
                ("width", NbtSummaryValue::Int(240)),
                ("type", string("minecraft:plain_message")),
            ]),
        ),
        ("inputs", NbtSummaryValue::List(inputs)),
        (
            "actions",
            NbtSummaryValue::List(vec![
                button(
                    vec![("url", text.value("https://example.invalid/a", "").to_nbt())],
                    "open_url",
                    true,
                ),
                button(vec![("value", raw.clone())], "copy_to_clipboard", false),
                button(
                    vec![("command", text.value("/say ", "").to_nbt())],
                    "run_command",
                    false,
                ),
                button(vec![("command", raw.clone())], "suggest_command", false),
                button(
                    vec![("template", text.value("say ", " $(text)").to_nbt())],
                    "dynamic/run_command",
                    false,
                ),
                button(
                    vec![
                        ("id", string("example:submit")),
                        ("additions", compound(vec![("raw", raw.clone())])),
                    ],
                    "dynamic/custom",
                    false,
                ),
            ]),
        ),
        ("type", string("minecraft:multi_action")),
    ])
}
fn record_packet(store: &mut WorldStore, registry: &str, id: &str, bytes: Option<Vec<u8>>) {
    let (packet_id, bytes) = bbb_protocol::packets::encode_configuration_registry_data(
        registry,
        &[RegistryDataEntry {
            id: id.into(),
            raw_data: bytes,
        }],
    )
    .unwrap();
    let ConfigurationClientbound::RegistryData(packet) =
        bbb_protocol::packets::decode_configuration_clientbound(packet_id, &bytes).unwrap()
    else {
        panic!("expected RegistryData")
    };
    assert_eq!(packet.raw_payload_len, bytes.len());
    store.record_registry_data(packet);
}
fn known_pack_source(id: &str, trim: &str, dialog: &str) -> bbb_client_assets::KnownPackDataSource {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-component-strings-{}-{nonce}",
        std::process::id()
    ));
    write_test_file(
        &root.join("version.json"),
        &format!(
            r#"{{"id":"26.1","protocol_version":{}}}"#,
            bbb_protocol::PROTOCOL_VERSION
        ),
    );
    for registry in bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        let (_, path) = registry.resource_id.split_once(':').unwrap();
        write_test_file(
            &root.join("data/minecraft").join(path).join("base.json"),
            "{}",
        );
    }
    let (namespace, path) = id.split_once(':').unwrap();
    for (registry, json) in [("trim_material", trim), ("dialog", dialog)] {
        write_test_file(
            &root
                .join("data")
                .join(namespace)
                .join(registry)
                .join(format!("{path}.json")),
            json,
        );
    }
    let roots = bbb_client_assets::PackRoots::from_paths(
        root.clone(),
        root.clone(),
        root.join("assets/minecraft"),
    );
    let loaded = bbb_client_assets::KnownPacksManager::load(&roots);
    fs::remove_dir_all(root).unwrap();
    let manager = loaded.unwrap();
    let core = core_known_pack();
    let selected = manager.select_offered(std::slice::from_ref(&core));
    assert_eq!(selected, [core]);
    manager.data_source_for(&selected).unwrap()
}
fn begin_candidate(store: &mut WorldStore, source: Source, id: &str, trim: &str, dialog: &str) {
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(store, None);
    if matches!(source, Source::KnownPack) {
        let data = known_pack_source(id, trim, dialog);
        let core = core_known_pack();
        store.apply_select_known_packs_with_data(vec![core.clone()], vec![core], data);
    }
    for (registry, json) in [(TRIM, trim), (DIALOG, dialog)] {
        match source {
            Source::Network => {
                let data =
                    bbb_registry_codec::decode_direct_registry_json(json.as_bytes()).unwrap();
                let nbt = data.source_order_nbt_projection().unwrap();
                record_packet(
                    store,
                    registry,
                    id,
                    Some(bbb_protocol::encode_nbt_summary(&nbt, 1 << 20).unwrap()),
                );
            }
            Source::ServerJson => {
                let data =
                    bbb_registry_codec::decode_direct_registry_json(json.as_bytes()).unwrap();
                let mut entry = RegistryPacketEntry::stub(id);
                entry.has_data = true;
                entry.element_data = Some(Arc::new(data));
                store.record_registry_entries(registry, 0, vec![entry]);
            }
            Source::KnownPack => record_packet(store, registry, id, None),
        }
    }
}
fn published_nbt(
    store: &WorldStore,
    registry: &str,
    source: Source,
    id: &str,
    json: &str,
) -> NbtSummaryValue {
    let entry = store.registry_entry_by_holder_id(registry, 0).unwrap();
    assert_eq!(entry.id, id);
    let data = entry.element_data().unwrap();
    assert_eq!(data.source, source.kind());
    if matches!(source, Source::KnownPack) {
        assert!(!entry.has_data);
        assert!(entry.raw_data().is_none());
        assert_eq!(entry.known_pack_data(), Some(json.as_bytes()));
    }
    decode_exact_synchronized_registry_element(registry, data)
        .unwrap()
        .encode_network(&store.frozen_registry_access())
        .unwrap()
}
fn assert_published(store: &mut WorldStore, source: Source, id: &str, text: &Text) {
    let trim = published_nbt(store, TRIM, source, id, &trim_json(text));
    let description = description_nbt(text);
    assert_eq!(
        trim,
        compound(vec![
            ("asset_name", string("quartz")),
            ("description", description.clone())
        ])
    );
    let material = store.synchronized_trim_material_presentation(0).unwrap();
    assert_eq!(material.description_nbt, Some(description.clone()));
    assert_eq!(
        material.description,
        format!(
            "{}urlcopyrunsuggest",
            text.value("", "").to_display_string()
        )
    );
    assert_eq!(
        material
            .description_styled
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        material.description
    );
    let dialog = published_nbt(store, DIALOG, source, id, &dialog_json(text));
    assert_eq!(dialog, dialog_nbt(text));
    let raw = text.value("", "");
    store.apply_show_dialog(bbb_protocol::packets::ShowDialog {
        dialog: bbb_protocol::packets::DialogHolder::Reference { registry_id: 0 },
    });
    let current = store
        .screen_requests()
        .iter()
        .rev()
        .find_map(|request| match request {
            crate::ScreenRequest::ShowDialog(dialog) => Some(dialog),
            _ => None,
        })
        .unwrap();
    assert_eq!(current.semantic_error, None);
    let crate::DialogDataState::MultiAction {
        common,
        actions,
        columns,
        ..
    } = current.data.as_ref().unwrap()
    else {
        panic!("published dialog must open as a typed form")
    };
    assert!(!common.pause);
    assert_eq!(*columns, 3);
    assert!(matches!(
        &common.body_data[0],
        crate::DialogBodyState::PlainMessage { message } if message.width == 240
    ));
    use crate::DialogInputControlState;
    assert!(
        matches!(&common.input_data[0].control, DialogInputControlState::Boolean { initial: false, on_true, on_false, .. } if on_true == &raw && on_false == &raw)
    );
    assert!(
        matches!(&common.input_data[1].control, DialogInputControlState::NumberRange { label_format, .. } if label_format == &raw)
    );
    assert!(
        matches!(&common.input_data[2].control, DialogInputControlState::SingleOption { options, .. } if options.iter().all(|option| option.id == raw))
    );
    assert!(
        matches!(&common.input_data[3].control, DialogInputControlState::Text { initial, .. } if initial == &raw)
    );
    assert_eq!(actions.len(), 6);
    assert!(actions.iter().all(|action| action.action_data.is_some()));
    assert_eq!(JavaString::from_nbt(field(&dialog, "title")).unwrap(), raw);
    if String::from_utf16(raw.units()).is_err() {
        assert_ne!(raw, JavaString::from(raw.to_display_string()));
    } else {
        assert_eq!(raw, JavaString::from(raw.to_display_string()));
    }
}
fn assert_publication(source: Source) {
    let mut store = WorldStore::new();
    for (index, text) in corpus().into_iter().enumerate() {
        let previous_revision = store.registry_access_revision();
        let previous_material = store.synchronized_trim_material_presentation(0).cloned();
        let previous_holder = store
            .registry_resource_id_by_holder_id(DIALOG, 0)
            .map(str::to_owned);
        let id = format!("example:strings_{index}");
        begin_candidate(
            &mut store,
            source,
            &id,
            &trim_json(&text),
            &dialog_json(&text),
        );
        assert_eq!(store.registry_access_revision(), previous_revision);
        assert_eq!(
            store.synchronized_trim_material_presentation(0),
            previous_material.as_ref()
        );
        assert_eq!(
            store.registry_resource_id_by_holder_id(DIALOG, 0),
            previous_holder.as_deref()
        );
        store.finish_registry_configuration().unwrap();
        assert_ne!(store.registry_access_revision(), previous_revision);
        assert_published(&mut store, source, &id, &text);
    }
}

#[derive(Clone, Copy)]
enum Invalid {
    UrlScheme,
    UrlSpace,
    UriProjectionTrap,
    CommandControl,
    DescriptionType,
    DialogTitleType,
    ButtonLabelType,
    TemplateVariable,
    TemplateRawVariable,
    InputKey,
    TextInitialType,
    TextInitialLength,
}
fn invalid_candidate(text: &Text, invalid: Invalid) -> (String, String, &'static str) {
    let mut trim = trim_json(text);
    let mut dialog = dialog_json(text);
    let registry = match invalid {
        Invalid::UrlScheme => {
            trim = trim.replace(
                &text.quoted("https://example.invalid/a", ""),
                "\"file:///tmp/a\"",
            );
            TRIM
        }
        Invalid::UrlSpace => {
            trim = trim.replace(
                &text.quoted("https://example.invalid/a", ""),
                "\"https://example.invalid/a b\"",
            );
            TRIM
        }
        Invalid::UriProjectionTrap => {
            trim = trim.replace(
                &text.quoted("https://example.invalid/a", ""),
                r#""https://example.invalid/a\ud800[tail""#,
            );
            TRIM
        }
        Invalid::CommandControl => {
            trim = trim.replace(&text.quoted("/say ", ""), r#""/say \u0001""#);
            TRIM
        }
        Invalid::DescriptionType => {
            trim = r#"{"asset_name":"quartz","description":7}"#.into();
            TRIM
        }
        Invalid::DialogTitleType => {
            dialog = dialog.replacen(&format!("\"title\":{}", text.json), "\"title\":7", 1);
            DIALOG
        }
        Invalid::ButtonLabelType => {
            dialog = dialog.replacen(
                &format!("\"label\":{},\"tooltip\"", text.json),
                "\"label\":7,\"tooltip\"",
                1,
            );
            DIALOG
        }
        Invalid::TemplateVariable => {
            dialog = dialog.replace(&text.quoted("say ", " $(text)"), "\"say $(bad-name)\"");
            DIALOG
        }
        Invalid::TemplateRawVariable => {
            dialog = dialog.replace(&text.quoted("say ", " $(text)"), r#""say $(\ud800)""#);
            DIALOG
        }
        Invalid::InputKey => {
            dialog = dialog.replacen("\"key\":\"boolean\"", r#""key":"\ud800""#, 1);
            DIALOG
        }
        Invalid::TextInitialType => {
            dialog = dialog.replacen(&format!("\"initial\":{}", text.json), "\"initial\":7", 1);
            DIALOG
        }
        Invalid::TextInitialLength => {
            dialog = dialog.replacen(
                &format!("\"initial\":{}", text.json),
                r#""initial":"\ud800\udc00","max_length":1"#,
                1,
            );
            DIALOG
        }
    };
    (trim, dialog, registry)
}
fn assert_invalid_preserves_and_recovers(source: Source) {
    let mut store = WorldStore::new();
    let texts = corpus();
    let text = &texts[0];
    begin_candidate(
        &mut store,
        source,
        "example:active",
        &trim_json(text),
        &dialog_json(text),
    );
    store.finish_registry_configuration().unwrap();
    assert_published(&mut store, source, "example:active", text);
    let active_revision = store.registry_access_revision();
    let active = store.synchronized_trim_material_presentation(0).cloned();
    let active_dialog = published_nbt(&store, DIALOG, source, "example:active", &dialog_json(text));
    for invalid in [
        Invalid::UrlScheme,
        Invalid::UrlSpace,
        Invalid::UriProjectionTrap,
        Invalid::CommandControl,
        Invalid::DescriptionType,
        Invalid::DialogTitleType,
        Invalid::ButtonLabelType,
        Invalid::TemplateVariable,
        Invalid::TemplateRawVariable,
        Invalid::InputKey,
        Invalid::TextInitialType,
        Invalid::TextInitialLength,
    ] {
        let (trim, dialog, registry) = invalid_candidate(text, invalid);
        begin_candidate(&mut store, source, "example:rejected", &trim, &dialog);
        let error = store.finish_registry_configuration().unwrap_err();
        assert_eq!(error.registry, registry);
        assert_eq!(store.registry_access_revision(), active_revision);
        assert_eq!(
            store.synchronized_trim_material_presentation(0),
            active.as_ref()
        );
        for registry in [TRIM, DIALOG] {
            assert_eq!(
                store.registry_resource_id_by_holder_id(registry, 0),
                Some("example:active")
            );
            assert_eq!(
                store.registry_holder_id_by_resource_id(registry, "example:rejected"),
                None
            );
        }
        assert_eq!(
            published_nbt(&store, DIALOG, source, "example:active", &dialog_json(text)),
            active_dialog
        );
        assert_eq!(
            store
                .apply_diagnostics()
                .last_apply_error
                .as_ref()
                .unwrap()
                .source,
            "registry_configuration_exact_codecs"
        );
    }
    let text = &texts[1];
    begin_candidate(
        &mut store,
        source,
        "example:recovered",
        &trim_json(text),
        &dialog_json(text),
    );
    store.finish_registry_configuration().unwrap();
    assert_ne!(store.registry_access_revision(), active_revision);
    assert_published(&mut store, source, "example:recovered", text);
}
macro_rules! source_tests {
    ($module:ident, $source:expr) => {
        mod $module {
            use super::*;
            #[test]
            fn raw_component_and_dialog_strings_publish_without_display_projection() {
                assert_publication($source);
            }
            #[test]
            fn restricted_strings_preserve_active_generation_and_recover() {
                assert_invalid_preserves_and_recovers($source);
            }
        }
    };
}
source_tests!(network, Source::Network);
source_tests!(server_json, Source::ServerJson);
source_tests!(known_pack, Source::KnownPack);
