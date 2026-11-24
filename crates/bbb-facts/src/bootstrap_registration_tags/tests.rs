use super::*;
use serde_json::{json, Value};

fn malformed(change: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(ARTIFACT).unwrap();
    change(&mut value);
    parse(&serde_json::to_string(&value).unwrap())
        .unwrap_err()
        .to_string()
}

#[test]
fn bootstrap_registration_tags_are_complete_and_cached() {
    let catalog = vanilla_bootstrap_registration_tags().unwrap();
    assert!(std::ptr::eq(
        catalog,
        vanilla_bootstrap_registration_tags().unwrap()
    ));
    assert_eq!(catalog.registries().len(), 95);
    assert_eq!(
        catalog
            .registries()
            .values()
            .map(BTreeSet::len)
            .sum::<usize>(),
        35
    );
    assert_eq!(
        catalog
            .registries()
            .values()
            .filter(|tags| tags.is_empty())
            .count(),
        92
    );
    assert!(catalog.contains("minecraft:item", "minecraft:wooden_tool_materials"));
    assert!(catalog.contains("minecraft:block", "minecraft:mineable/pickaxe"));
    assert!(catalog.contains("minecraft:entity_type", "minecraft:can_equip_saddle"));
    assert!(!catalog.contains("minecraft:item", "test:new"));
    assert!(!catalog.contains("test:registry", "minecraft:wooden_tool_materials"));
    assert!(catalog.registries().values().any(BTreeSet::is_empty));
    assert!(!catalog.registries().contains_key("minecraft:damage_type"));
}

#[test]
fn bootstrap_registration_tags_reject_bad_metadata_and_counts() {
    assert!(malformed(|v| v["vanilla_version"] = json!("26.0")).contains("version mismatch"));
    assert!(malformed(|v| v["source"] = json!("after reload")).contains("source mismatch"));
    assert!(malformed(|v| v["registry_count"] = json!(94)).contains("registry count"));
    assert!(malformed(|v| v["tag_count"] = json!(0)).contains("tag count"));
    assert!(malformed(|v| {
        let row = v["registries"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| !row["tags"].as_array().unwrap().is_empty())
            .unwrap();
        row["tags"].as_array_mut().unwrap().pop();
    })
    .contains("tag count"));
    assert!(malformed(|v| {
        v["registries"].as_array_mut().unwrap().pop();
    })
    .contains("registry count"));
    assert!(malformed(|v| v["unexpected"] = json!(true)).contains("unknown field"));
    assert!(
        malformed(|v| v["registries"][0]["unexpected"] = json!(true)).contains("unknown field")
    );
}

#[test]
fn bootstrap_registration_tags_reject_bad_registry_and_tag_identity_order() {
    assert!(
        malformed(|v| v["registries"][0]["resource_id"] = json!("test:unknown"))
            .contains("unknown registry")
    );
    assert!(
        malformed(|v| v["registries"].as_array_mut().unwrap().swap(0, 1))
            .contains("unordered registry")
    );
    assert!(malformed(|v| v["registries"][1] = v["registries"][0].clone()).contains("duplicate"));
    assert!(
        malformed(|v| v["registries"][0]["tags"] = json!(["Bad:Tag"]))
            .contains("invalid tag identifier")
    );
    assert!(
        malformed(|v| v["registries"][0]["tags"] = json!(["test:a", "test:a"]))
            .contains("duplicate")
    );
    assert!(
        malformed(|v| v["registries"][0]["tags"] = json!(["test:z", "test:a"]))
            .contains("unordered tag")
    );
}
