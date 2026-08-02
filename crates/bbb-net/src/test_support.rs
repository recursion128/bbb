use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn known_packs_manager(label: &str) -> bbb_pack::KnownPacksManager {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bbb-net-known-packs-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    write_file(
        &root.join("version.json"),
        &format!(
            r#"{{"id":"26.1","protocol_version":{}}}"#,
            bbb_protocol::PROTOCOL_VERSION
        ),
    );
    for registry in bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        let (_, registry_path) = registry.resource_id.split_once(':').unwrap();
        let contents =
            minimal_registry_json(registry.resource_id).unwrap_or_else(|| "{}".to_string());
        write_file(
            &root
                .join("data/minecraft")
                .join(registry_path)
                .join("base.json"),
            &contents,
        );
    }
    let roots =
        bbb_pack::PackRoots::from_paths(root.clone(), root.clone(), root.join("assets/minecraft"));
    let manager = bbb_pack::KnownPacksManager::load(&roots).unwrap();
    fs::remove_dir_all(root).unwrap();
    manager
}

fn minimal_registry_json(registry: &str) -> Option<String> {
    match registry {
        "minecraft:damage_type" => {
            return Some(r#"{"message_id":"base","scaling":"never","exhaustion":0.0}"#.to_string());
        }
        "minecraft:chicken_variant"
        | "minecraft:cow_variant"
        | "minecraft:pig_variant"
        | "minecraft:cat_variant" => {
            return Some(
                r#"{"asset_id":"example:adult","baby_asset_id":"example:baby"}"#.to_string(),
            );
        }
        "minecraft:frog_variant" | "minecraft:zombie_nautilus_variant" => {
            return Some(r#"{"asset_id":"example:variant"}"#.to_string());
        }
        "minecraft:wolf_variant" => {
            return Some(
                r#"{"assets":{"wild":"example:wild","tame":"example:tame","angry":"example:angry"},"baby_assets":{"wild":"example:baby_wild","tame":"example:baby_tame","angry":"example:baby_angry"}}"#.to_string(),
            );
        }
        "minecraft:painting_variant" => {
            return Some(r#"{"width":1,"height":1,"asset_id":"example:painting"}"#.to_string());
        }
        _ => {}
    }
    let fields = match registry {
        "minecraft:cat_sound_variant" => [
            "ambient_sound",
            "stray_ambient_sound",
            "hiss_sound",
            "hurt_sound",
            "death_sound",
            "eat_sound",
            "beg_for_food_sound",
            "purr_sound",
            "purreow_sound",
        ]
        .as_slice(),
        "minecraft:chicken_sound_variant" | "minecraft:cow_sound_variant" => {
            ["ambient_sound", "hurt_sound", "death_sound", "step_sound"].as_slice()
        }
        "minecraft:pig_sound_variant" => [
            "ambient_sound",
            "hurt_sound",
            "death_sound",
            "step_sound",
            "eat_sound",
        ]
        .as_slice(),
        "minecraft:wolf_sound_variant" => [
            "ambient_sound",
            "death_sound",
            "growl_sound",
            "hurt_sound",
            "pant_sound",
            "whine_sound",
            "step_sound",
        ]
        .as_slice(),
        _ => return None,
    };
    let sound_set = fields
        .iter()
        .map(|field| format!(r#""{field}":{{"sound_id":"minecraft:entity.cat.ambient"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    Some(if registry == "minecraft:cow_sound_variant" {
        format!("{{{sound_set}}}")
    } else {
        format!(r#"{{"adult_sounds":{{{sound_set}}},"baby_sounds":{{{sound_set}}}}}"#)
    })
}

fn write_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}
