use super::*;
use bbb_protocol::{packets::ConfigurationClientbound, JavaString};

const REGISTRY: &str = "minecraft:trim_material";

#[derive(Clone, Copy, Debug)]
enum Source {
    Network,
    ServerJson,
    KnownPack,
}

impl Source {
    fn element_source(self) -> RegistryElementSource {
        match self {
            Self::Network => RegistryElementSource::NetworkNbt,
            Self::ServerJson => RegistryElementSource::ServerDatapackJson,
            Self::KnownPack => RegistryElementSource::ClientKnownPackJson,
        }
    }
}

#[derive(Clone)]
struct Input {
    nbt: NbtSummaryValue,
    json: &'static str,
}

fn string(units: &[u16], json: &'static str) -> Input {
    Input {
        nbt: JavaString::from(units.to_vec()).to_nbt(),
        json,
    }
}

fn corpus() -> Vec<(Input, Input)> {
    vec![
        (
            string(&[0xd800], r#""\ud800""#),
            string(&[0xdc00], r#""\udc00""#),
        ),
        (
            string(&[0xdc00], r#""\udc00""#),
            string(&[0xd800], r#""\ud800""#),
        ),
        (
            string(&[0x61, 0xd800, 0x62], r#""a\ud800b""#),
            string(&[0x63, 0xdc00, 0x64], r#""c\udc00d""#),
        ),
        (
            string(&[0xd83d, 0xde00], r#""\ud83d\ude00""#),
            string(&[0xd834, 0xdd1e], r#""\ud834\udd1e""#),
        ),
        (
            string(&[0xd83d, 0xde00], r#""😀""#),
            string(&[0xd834, 0xdd1e], r#""𝄞""#),
        ),
        (
            string(&[0x70, 0x6c, 0x61, 0x79, 0x65, 0x72], r#""player""#),
            string(&[0x73, 0x63, 0x6f, 0x72, 0x65], r#""score""#),
        ),
        (string(&[], r#""""#), string(&[], r#""""#)),
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

fn trim_nbt(name: &Input, objective: &Input) -> NbtSummaryValue {
    compound(vec![
        ("asset_name", NbtSummaryValue::String("quartz".into())),
        (
            "description",
            compound(vec![
                ("type", NbtSummaryValue::String("score".into())),
                (
                    "score",
                    compound(vec![
                        ("objective", objective.nbt.clone()),
                        ("ignored", NbtSummaryValue::Int(1)),
                        ("name", name.nbt.clone()),
                    ]),
                ),
                (
                    "extra",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::String("display".into()),
                        string(&[0xd800, 0x7c, 0xdc00], r#""\ud800|\udc00""#).nbt,
                    ]),
                ),
                ("color", NbtSummaryValue::String("red".into())),
                ("ignored", NbtSummaryValue::Int(1)),
            ]),
        ),
    ])
}

fn trim_json(name: &Input, objective: &Input) -> String {
    format!(
        r#"{{"asset_name":"quartz","description":{{"type":"score","score":{{"objective":{},"ignored":1,"name":{}}},"extra":["display","\ud800|\udc00"],"color":"red","ignored":1}}}}"#,
        objective.json, name.json,
    )
}

fn record_packet(store: &mut WorldStore, resource_id: &str, raw_data: Option<Vec<u8>>) {
    let (packet_id, bytes) = bbb_protocol::packets::encode_configuration_registry_data(
        REGISTRY,
        &[RegistryDataEntry {
            id: resource_id.into(),
            raw_data,
        }],
    )
    .unwrap();
    let ConfigurationClientbound::RegistryData(packet) =
        bbb_protocol::packets::decode_configuration_clientbound(packet_id, &bytes).unwrap()
    else {
        panic!("configuration packet must decode as RegistryData");
    };
    assert_eq!(packet.raw_payload_len, bytes.len());
    store.record_registry_data(packet);
}

fn known_pack_source(resource_id: &str, json: &str) -> bbb_client_assets::KnownPackDataSource {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-score-known-pack-{}-{nonce}",
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
    let (namespace, path) = resource_id.split_once(':').unwrap();
    write_test_file(
        &root
            .join("data")
            .join(namespace)
            .join("trim_material")
            .join(format!("{path}.json")),
        json,
    );
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

fn begin_candidate(
    store: &mut WorldStore,
    source: Source,
    resource_id: &str,
    name: &Input,
    objective: &Input,
) {
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(store, None);
    match source {
        Source::Network => record_packet(
            store,
            resource_id,
            Some(bbb_protocol::encode_nbt_summary(&trim_nbt(name, objective), 1 << 20).unwrap()),
        ),
        Source::ServerJson => {
            let data = bbb_registry_codec::decode_direct_registry_json(
                trim_json(name, objective).as_bytes(),
            )
            .unwrap();
            let mut entry = RegistryPacketEntry::stub(resource_id);
            entry.has_data = true;
            entry.element_data = Some(Arc::new(data));
            store.record_registry_entries(REGISTRY, 0, vec![entry]);
        }
        Source::KnownPack => {
            let data = known_pack_source(resource_id, &trim_json(name, objective));
            let core = core_known_pack();
            store.apply_select_known_packs_with_data(vec![core.clone()], vec![core], data);
            record_packet(store, resource_id, None);
        }
    }
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    let NbtSummaryValue::Compound(fields) = value else {
        panic!("expected compound containing {name}");
    };
    &fields
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

fn assert_published(
    store: &WorldStore,
    source: Source,
    resource_id: &str,
    name: &Input,
    objective: &Input,
) {
    let entry = store.registry_entry_by_holder_id(REGISTRY, 0).unwrap();
    assert_eq!(entry.id, resource_id);
    assert_eq!(
        entry.element_data().unwrap().source,
        source.element_source()
    );
    if matches!(source, Source::KnownPack) {
        assert!(!entry.has_data);
        assert!(entry.raw_data().is_none());
        assert_eq!(
            entry.known_pack_data(),
            Some(trim_json(name, objective).as_bytes())
        );
    }
    let material = store.synchronized_trim_material_presentation(0).unwrap();
    assert_eq!(material.asset_name, "quartz");
    assert_eq!(material.description, "display�|�");
    assert_eq!(
        material
            .description_styled
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        material.description
    );
    let description = material.description_nbt.as_ref().unwrap();
    let score = field(description, "score");
    for (field_name, expected) in [("name", name), ("objective", objective)] {
        assert_eq!(
            JavaString::from_nbt(field(score, field_name))
                .unwrap()
                .units(),
            JavaString::from_nbt(&expected.nbt).unwrap().units()
        );
    }
    assert_eq!(
        score,
        &compound(vec![
            ("name", name.nbt.clone()),
            ("objective", objective.nbt.clone())
        ])
    );
    assert_eq!(
        description,
        &compound(vec![
            ("score", score.clone()),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("display".into()),
                    string(&[0xd800, 0x7c, 0xdc00], r#""\ud800|\udc00""#).nbt
                ])
            ),
            ("color", NbtSummaryValue::String("red".into())),
        ])
    );
}

fn assert_publication(source: Source) {
    let mut store = WorldStore::new();
    for (index, (name, objective)) in corpus().into_iter().enumerate() {
        let previous_revision = store.registry_access_revision();
        let previous_material = store.synchronized_trim_material_presentation(0).cloned();
        let previous_id = store
            .registry_resource_id_by_holder_id(REGISTRY, 0)
            .map(str::to_owned);
        let id = format!("example:score_{index}");
        begin_candidate(&mut store, source, &id, &name, &objective);
        assert_eq!(store.registry_access_revision(), previous_revision);
        assert_eq!(
            store.synchronized_trim_material_presentation(0),
            previous_material.as_ref()
        );
        assert_eq!(
            store.registry_resource_id_by_holder_id(REGISTRY, 0),
            previous_id.as_deref()
        );
        store.finish_registry_configuration().unwrap();
        assert_ne!(store.registry_access_revision(), previous_revision);
        assert_published(&store, source, &id, &name, &objective);
    }
}

fn assert_invalid_preserves_and_recovers(source: Source, invalid_field: &str) {
    let mut store = WorldStore::new();
    let (name, objective) = &corpus()[0];
    begin_candidate(&mut store, source, "example:active", name, objective);
    store.finish_registry_configuration().unwrap();
    let active_revision = store.registry_access_revision();
    let active = store.synchronized_trim_material_presentation(0).cloned();
    for invalid in [
        Input {
            nbt: NbtSummaryValue::Int(7),
            json: "7",
        },
        Input {
            nbt: compound(vec![]),
            json: "{}",
        },
    ] {
        let (candidate_name, candidate_objective) = if invalid_field == "name" {
            (&invalid, objective)
        } else {
            (name, &invalid)
        };
        begin_candidate(
            &mut store,
            source,
            "example:invalid",
            candidate_name,
            candidate_objective,
        );
        let error = store.finish_registry_configuration().unwrap_err();
        assert_eq!(error.registry, REGISTRY);
        assert_eq!(error.validator, "TrimMaterial.DIRECT_CODEC");
        assert!(
            error
                .message
                .contains(&format!("description.score.{invalid_field}")),
            "{error:?}"
        );
        assert!(error.message.contains("expected a string"), "{error:?}");
        assert_eq!(
            store
                .apply_diagnostics()
                .last_apply_error
                .as_ref()
                .unwrap()
                .source,
            "registry_configuration_exact_codecs"
        );
        assert_eq!(store.registry_access_revision(), active_revision);
        assert_eq!(
            store.registry_resource_id_by_holder_id(REGISTRY, 0),
            Some("example:active")
        );
        assert_eq!(
            store.registry_holder_id_by_resource_id(REGISTRY, "example:invalid"),
            None
        );
        assert_eq!(
            store.synchronized_trim_material_presentation(0),
            active.as_ref()
        );
    }
    let (name, objective) = &corpus()[1];
    begin_candidate(&mut store, source, "example:recovered", name, objective);
    store.finish_registry_configuration().unwrap();
    assert_ne!(store.registry_access_revision(), active_revision);
    assert_published(&store, source, "example:recovered", name, objective);
}

macro_rules! source_tests {
    ($module:ident, $source:expr) => {
        mod $module {
            use super::*;

            #[test]
            fn raw_java_strings_publish_in_the_canonical_score_record() {
                assert_publication($source);
            }

            #[test]
            fn malformed_name_preserves_active_revision_and_recovers() {
                assert_invalid_preserves_and_recovers($source, "name");
            }

            #[test]
            fn malformed_objective_preserves_active_revision_and_recovers() {
                assert_invalid_preserves_and_recovers($source, "objective");
            }
        }
    };
}

source_tests!(network, Source::Network);
source_tests!(server_json, Source::ServerJson);
source_tests!(known_pack, Source::KnownPack);
