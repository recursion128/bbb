use std::path::{Path, PathBuf};

use bbb_client_assets::ItemRegistryCatalog;
use bbb_protocol::data_component_types as component_id;

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
        .join(bbb_client_assets::MC_VERSION)
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
    for field in ["default_component_codec_values", "crafting_remainders"] {
        let mut values = serde_json::Map::new();
        for resource_id in catalog["resource_ids"].as_array().unwrap() {
            let resource_id = resource_id.as_str().unwrap();
            if field == "default_component_codec_values" {
                let mut components = [
                    component_id::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
                    component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                    component_id::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                    component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                ]
                .into_iter()
                .filter_map(|type_id| {
                    let key = type_id.to_string();
                    vanilla[field][resource_id]
                        .get(&key)
                        .map(|value| (key, value.clone()))
                })
                .collect::<serde_json::Map<_, _>>();
                components.insert(
                    component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID.to_string(),
                    serde_json::json!(64),
                );
                components.insert(
                    component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID.to_string(),
                    serde_json::json!({"translate": format!("item.minecraft.{}", &resource_id[10..])}),
                );
                values.insert(
                    resource_id.to_string(),
                    serde_json::Value::Object(components),
                );
            }
        }
        catalog[field] = serde_json::Value::Object(values);
    }
    for field in ["block_items", "spawn_egg_items"] {
        catalog[field] = serde_json::json!([]);
    }
    catalog
}

fn copy_profile(catalog: &mut serde_json::Value, target_item_id: &str, vanilla_item_id: &str) {
    let vanilla = serde_json::to_value(ItemRegistryCatalog::vanilla_26_1().unwrap()).unwrap();
    let target = format!("minecraft:{target_item_id}");
    let source = format!("minecraft:{vanilla_item_id}");
    for field in ["default_component_codec_values", "crafting_remainders"] {
        if field == "default_component_codec_values" {
            let mut values = [
                component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ]
            .into_iter()
            .filter_map(|type_id| {
                let key = type_id.to_string();
                vanilla[field][&source]
                    .get(&key)
                    .map(|value| (key, value.clone()))
            })
            .collect::<serde_json::Map<_, _>>();
            let name_type = component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID.to_string();
            values.insert(
                name_type.clone(),
                catalog[field][&target][&name_type].clone(),
            );
            catalog[field][&target] = serde_json::Value::Object(values);
        } else if let Some(value) = vanilla[field].get(&source) {
            catalog[field][&target] = value.clone();
        }
    }
    for field in ["block_items", "spawn_egg_items"] {
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

pub(crate) fn install_damageable_item_defaults(
    world: &mut bbb_client_world::WorldStore,
    item_id: i32,
    max_damage: i32,
) {
    world.set_default_item_component_codec_values(std::collections::BTreeMap::from([(
        item_id,
        std::collections::BTreeMap::from([
            (
                component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                serde_json::json!(1),
            ),
            (
                component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                serde_json::json!(max_damage),
            ),
            (
                component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                serde_json::json!(0),
            ),
        ]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
}

pub(crate) fn install_max_stack_size_default(
    world: &mut bbb_client_world::WorldStore,
    item_id: i32,
    max_stack_size: i32,
) {
    world.set_default_item_component_codec_values(std::collections::BTreeMap::from([(
        item_id,
        std::collections::BTreeMap::from([(
            component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            serde_json::json!(max_stack_size),
        )]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
}

pub(crate) fn install_equippable_item_default(
    world: &mut bbb_client_world::WorldStore,
    item_id: i32,
    slot: &str,
) {
    world.set_default_item_component_codec_values(std::collections::BTreeMap::from([(
        item_id,
        std::collections::BTreeMap::from([(
            component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            serde_json::json!({"slot": slot}),
        )]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
}
