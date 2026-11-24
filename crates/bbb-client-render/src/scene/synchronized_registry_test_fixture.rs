//! Minimal synchronized-registry payloads shared by the scene and runtime tests.

use bbb_protocol::packets::{RegistryData, RegistryDataEntry};

pub fn minimal_non_empty_synchronized_registry_data() -> Vec<RegistryData> {
    bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES
        .iter()
        .filter(|data_type| data_type.validator == Some("RegistryValidator.nonEmpty ()"))
        .map(|data_type| RegistryData {
            registry: data_type.resource_id.to_string(),
            raw_payload_len: 0,
            entries: vec![RegistryDataEntry {
                id: "example:minimal".to_string(),
                raw_data: minimal_sound_variant_nbt(data_type.resource_id)
                    .or_else(|| minimal_animal_variant_nbt(data_type.resource_id)),
            }],
        })
        .collect()
}

fn minimal_animal_variant_nbt(registry: &str) -> Option<Vec<u8>> {
    let write_utf = |out: &mut Vec<u8>, value: &str| {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    };
    let write_string = |out: &mut Vec<u8>, name: &str, value: &str| {
        out.push(8);
        write_utf(out, name);
        write_utf(out, value);
    };
    let mut data = vec![10];
    match registry {
        "minecraft:wolf_variant" => {
            for (name, prefix) in [("assets", "adult"), ("baby_assets", "baby")] {
                data.push(10);
                write_utf(&mut data, name);
                for state in ["wild", "tame", "angry"] {
                    write_string(&mut data, state, &format!("example:{prefix}_{state}"));
                }
                data.push(0);
            }
        }
        "minecraft:pig_variant"
        | "minecraft:cat_variant"
        | "minecraft:cow_variant"
        | "minecraft:chicken_variant" => {
            write_string(&mut data, "asset_id", "example:adult");
            write_string(&mut data, "baby_asset_id", "example:baby");
        }
        "minecraft:frog_variant" | "minecraft:zombie_nautilus_variant" => {
            write_string(&mut data, "asset_id", "example:variant");
        }
        "minecraft:painting_variant" => {
            for name in ["width", "height"] {
                data.push(3);
                write_utf(&mut data, name);
                data.extend_from_slice(&1_i32.to_be_bytes());
            }
            write_string(&mut data, "asset_id", "example:painting");
        }
        _ => return None,
    }
    data.push(0);
    Some(data)
}

fn minimal_sound_variant_nbt(registry: &str) -> Option<Vec<u8>> {
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
    let write_utf = |out: &mut Vec<u8>, value: &str| {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    };
    let write_sound_set = |out: &mut Vec<u8>| {
        for field in fields {
            out.push(10);
            write_utf(out, field);
            out.push(8);
            write_utf(out, "sound_id");
            write_utf(out, "minecraft:entity.cat.ambient");
            out.push(0);
        }
    };
    let mut data = vec![10];
    if registry == "minecraft:cow_sound_variant" {
        write_sound_set(&mut data);
    } else {
        data.push(10);
        write_utf(&mut data, "adult_sounds");
        write_sound_set(&mut data);
        data.push(0);
        data.push(10);
        write_utf(&mut data, "baby_sounds");
        write_sound_set(&mut data);
        data.push(0);
    }
    data.push(0);
    Some(data)
}
