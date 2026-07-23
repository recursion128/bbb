use std::path::{Path, PathBuf};

use bbb_pack::ItemRegistryCatalog;

pub(crate) fn write_item_registry_fixture(root: &Path, item_ids: &[&str]) {
    write_item_registry_fixture_with_profiles(root, item_ids, &[]);
}

pub(crate) fn write_item_registry_fixture_with_profiles(
    root: &Path,
    item_ids: &[&str],
    profiles: &[(&str, &str)],
) {
    let mut catalog = fixture_value(item_ids);
    for (target, source) in profiles {
        copy_profile(&mut catalog, target, source);
    }
    let path = fixture_path(root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&catalog).unwrap()).unwrap();
}

fn fixture_path(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_pack::MC_VERSION)
        .join(".bbb")
        .join("item_registry_catalog.json")
}

fn fixture_value(item_ids: &[&str]) -> serde_json::Value {
    let vanilla = serde_json::to_value(ItemRegistryCatalog::vanilla_26_1().unwrap()).unwrap();
    let resource_ids = item_ids
        .iter()
        .map(|item_id| format!("minecraft:{item_id}"))
        .collect::<Vec<_>>();
    let mut catalog = serde_json::json!({
        "resource_ids": resource_ids,
        "protocol_ids": item_ids
            .iter()
            .enumerate()
            .map(|(id, item_id)| (format!("minecraft:{item_id}"), serde_json::json!(id as i32)))
            .collect::<serde_json::Map<_, _>>()
    });
    for field in [
        "max_damage",
        "max_stack_size",
        "default_item_name_translation_keys",
        "default_component_type_ids",
        "registry_dependent_component_initializers",
        "default_equipment_slots",
        "humanoid_armor_assets",
        "equippable_assets",
        "default_mount_body_armor_kinds",
        "mount_body_armor_assets",
        "default_attack_ranges",
        "default_minimum_attack_charge_bits",
        "default_swing_animation_durations",
        "default_kinetic_weapons",
        "default_attribute_modifiers",
        "default_use_effects",
        "default_consumables",
        "crafting_remainders",
        "mining_profiles",
    ] {
        let mut values = serde_json::Map::new();
        for resource_id in catalog["resource_ids"].as_array().unwrap() {
            let resource_id = resource_id.as_str().unwrap();
            if field == "max_stack_size" {
                values.insert(resource_id.to_string(), serde_json::json!(64));
            } else if field == "default_item_name_translation_keys" {
                values.insert(
                    resource_id.to_string(),
                    serde_json::json!(format!("item.minecraft.{}", &resource_id[10..])),
                );
            } else if field == "default_component_type_ids" {
                values.insert(
                    resource_id.to_string(),
                    vanilla[field]["minecraft:stick"].clone(),
                );
            } else if field == "registry_dependent_component_initializers" {
                if let Some(value) = vanilla[field].get(resource_id) {
                    values.insert(resource_id.to_string(), value.clone());
                }
            } else if field == "default_kinetic_weapons" {
                if let Some(value) = vanilla[field].get(resource_id) {
                    values.insert(resource_id.to_string(), value.clone());
                }
            }
        }
        catalog[field] = serde_json::Value::Object(values);
    }
    for field in [
        "block_items",
        "spawn_egg_items",
        "default_piercing_weapon_ids",
    ] {
        catalog[field] = serde_json::json!([]);
    }
    catalog
}

fn copy_profile(catalog: &mut serde_json::Value, target_item_id: &str, vanilla_item_id: &str) {
    let vanilla = serde_json::to_value(ItemRegistryCatalog::vanilla_26_1().unwrap()).unwrap();
    let target = format!("minecraft:{target_item_id}");
    let source = format!("minecraft:{vanilla_item_id}");
    for field in [
        "max_damage",
        "max_stack_size",
        "default_component_type_ids",
        "registry_dependent_component_initializers",
        "default_equipment_slots",
        "humanoid_armor_assets",
        "equippable_assets",
        "default_mount_body_armor_kinds",
        "mount_body_armor_assets",
        "default_attack_ranges",
        "default_minimum_attack_charge_bits",
        "default_swing_animation_durations",
        "default_kinetic_weapons",
        "default_attribute_modifiers",
        "default_use_effects",
        "default_consumables",
        "crafting_remainders",
    ] {
        if let Some(value) = vanilla[field].get(&source) {
            catalog[field][&target] = value.clone();
        }
    }
    for field in [
        "block_items",
        "spawn_egg_items",
        "default_piercing_weapon_ids",
    ] {
        if vanilla[field]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some(&source))
        {
            catalog[field]
                .as_array_mut()
                .unwrap()
                .push(serde_json::Value::String(target.clone()));
        }
    }
}
