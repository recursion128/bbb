use super::*;

#[derive(Clone, Copy, Debug)]
enum Source {
    KnownPack,
    Network,
}

fn trim_entry(source: Source, resource_id: &str, value: serde_json::Value) -> RegistryPacketEntry {
    let json = serde_json::to_vec(&value).unwrap();
    match source {
        Source::KnownPack => RegistryPacketEntry::with_known_pack_data_for_test(resource_id, json),
        Source::Network => {
            let data = bbb_registry_codec::decode_client_known_pack_registry_json(&json).unwrap();
            RegistryPacketEntry::with_raw_data(
                resource_id,
                bbb_protocol::encode_nbt_summary(
                    &data.source_order_nbt_projection().unwrap(),
                    1 << 20,
                )
                .unwrap(),
            )
        }
    }
}

fn begin_trim_candidate(store: &mut WorldStore, entry: RegistryPacketEntry) {
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(store, None);
    store.record_registry_entries("minecraft:trim_material", 0, vec![entry]);
}

fn trim_with_separator(separator: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "asset_name": "quartz",
        "description": {
            "type": "nbt",
            "nbt": "foo",
            "source": "entity",
            "entity": "@s",
            "separator": separator,
        },
    })
}

fn description_has_separator(store: &WorldStore) -> bool {
    let material = store.synchronized_trim_material_presentation(0).unwrap();
    let Some(NbtSummaryValue::Compound(fields)) = &material.description_nbt else {
        panic!("NBT description must retain its canonical component map");
    };
    fields.iter().any(|entry| entry.name == "separator")
}

fn assert_node_budget_rejects_candidate(source: Source) {
    let mut store = WorldStore::new();
    begin_trim_candidate(
        &mut store,
        trim_entry(
            source,
            "example:active",
            serde_json::json!({"asset_name": "gold", "description": "Active"}),
        ),
    );
    store.finish_registry_configuration().unwrap();
    let active_revision = store.registry_access_revision();
    let active_material = store
        .synchronized_trim_material_presentation(0)
        .unwrap()
        .clone();
    assert_eq!(active_material.asset_name, "gold");
    assert_eq!(active_material.description, "Active");

    // Use boolean `with` primitives so the network NBT form stays well within
    // Vanilla's 2 MiB `NbtAccounter` wire quota (`RegistrySynchronization.java:92-98`)
    // and reaches `TrimMaterial.DIRECT_CODEC`'s 65,536-node component budget.
    let inner = serde_json::json!({
        "translate": "k",
        "with": vec![serde_json::json!(true); 4096],
    });
    let separator = serde_json::Value::Array(vec![inner; 17]);
    begin_trim_candidate(
        &mut store,
        trim_entry(
            source,
            "example:over_budget",
            trim_with_separator(separator),
        ),
    );

    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.registry, "minecraft:trim_material");
    assert_eq!(error.validator, "TrimMaterial.DIRECT_CODEC");
    assert!(
        error.message.contains("node count exceeds 65536"),
        "{error:?}"
    );
    assert!(error.message.contains("description.separator"), "{error:?}");
    assert_eq!(store.registry_access_revision(), active_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:trim_material", 0),
        Some("example:active")
    );
    assert_eq!(
        store.synchronized_trim_material_presentation(0),
        Some(&active_material)
    );

    begin_trim_candidate(
        &mut store,
        trim_entry(
            source,
            "example:recovered",
            trim_with_separator(serde_json::json!(",")),
        ),
    );
    store.finish_registry_configuration().unwrap();
    assert_ne!(store.registry_access_revision(), active_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:trim_material", 0),
        Some("example:recovered")
    );
    assert!(description_has_separator(&store));
}

#[test]
fn codec_terminal_known_pack_component_node_budget_preserves_active_revision() {
    assert_node_budget_rejects_candidate(Source::KnownPack);
}

#[test]
fn codec_terminal_network_component_node_budget_preserves_active_revision() {
    assert_node_budget_rejects_candidate(Source::Network);
}
