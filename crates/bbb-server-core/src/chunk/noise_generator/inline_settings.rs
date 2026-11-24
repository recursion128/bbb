//! Saved inline `NoiseGeneratorSettings` holder compilation.

use super::*;
use bbb_registry_codec::RegistryJsonValue as Json;
use bbb_server_datapack::dynamic_registry::RawRegistryElement;
use bbb_server_datapack::worldgen::{noise::NoiseGeneratorSettings, WorldgenElement};

const INLINE_NOISE_SETTINGS_ID: &str = "bbb:inline_noise_settings";

pub(super) fn resolve_noise_settings(
    startup: Arc<DatapackStartup>,
    value: &NbtSummaryValue,
    path: &str,
) -> SaveResult<(Arc<DatapackStartup>, String)> {
    match value {
        NbtSummaryValue::String(_) => Ok((startup, expect_resource_id(value, path)?)),
        NbtSummaryValue::Compound(_) => install_inline_noise_settings(startup, value, path),
        _ => Err(invalid_schema(
            path,
            "must be a named holder or inline noise-settings compound",
        )),
    }
}

fn install_inline_noise_settings(
    startup: Arc<DatapackStartup>,
    value: &NbtSummaryValue,
    path: &str,
) -> SaveResult<(Arc<DatapackStartup>, String)> {
    // `RegistryFileCodec.java:49-72`; `NoiseGeneratorSettings.java:32-48`.
    let mut node_budget = MAX_INLINE_NBT_JSON_NODES;
    let source = inline_codec_nbt_to_json(value, path, 0, &mut node_budget)?;
    let typed = NoiseGeneratorSettings::decode(&source)
        .map_err(|error| invalid_schema(path, error.to_string()))?;
    let mut startup = (*startup).clone();
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .ok_or_else(|| invalid_schema(path, "noise-settings registry is absent"))?;
    let mut settings_id = INLINE_NOISE_SETTINGS_ID.to_string();
    let mut suffix = 0_u32;
    while registry
        .elements
        .iter()
        .any(|element| element.id == settings_id)
    {
        suffix = suffix
            .checked_add(1)
            .ok_or_else(|| invalid_schema(path, "inline settings id space is exhausted"))?;
        settings_id = format!("{INLINE_NOISE_SETTINGS_ID}_{suffix}");
    }
    let source_json = serde_json::to_vec(&source)
        .map_err(|error| invalid_schema(path, format!("cannot encode inline settings: {error}")))?;
    registry.elements.push(RawRegistryElement {
        id: settings_id.clone(),
        source_pack: "level.dat:inline".to_string(),
        source_json: source_json.into(),
        source: None,
        codec_value: None,
    });
    startup.registries.worldgen.insert(
        settings_id.clone(),
        WorldgenElement::NoiseSettings(Arc::new(typed)),
    );
    Ok((Arc::new(startup), settings_id))
}

const MAX_INLINE_NBT_JSON_NODES: usize = 65_536;

fn consume_nodes(budget: &mut usize, count: usize, path: &str) -> SaveResult<()> {
    match budget.checked_sub(count) {
        Some(remaining) => {
            *budget = remaining;
            Ok(())
        }
        None => Err(invalid_schema(
            path,
            "inline settings exceed the NBT node budget",
        )),
    }
}

fn inline_codec_nbt_to_json(
    value: &NbtSummaryValue,
    path: &str,
    depth: usize,
    budget: &mut usize,
) -> SaveResult<Json> {
    if depth > 64 {
        return Err(invalid_schema(
            path,
            "inline settings exceed the NBT depth limit",
        ));
    }
    consume_nodes(budget, 1, path)?;
    let number = |value: f64| {
        serde_json::Number::from_f64(value)
            .map(|value| Json::Number(value.to_string()))
            .ok_or_else(|| invalid_schema(path, "inline settings contain a non-finite number"))
    };
    match value {
        NbtSummaryValue::Byte(0) => Ok(Json::Bool(false)),
        NbtSummaryValue::Byte(1) => Ok(Json::Bool(true)),
        NbtSummaryValue::Byte(value) => Err(invalid_schema(
            path,
            format!("non-canonical inline boolean byte {value}"),
        )),
        NbtSummaryValue::Short(value) => Ok(Json::Number(value.to_string())),
        NbtSummaryValue::Int(value) => Ok(Json::Number(value.to_string())),
        NbtSummaryValue::Long(value) => Ok(Json::Number(value.to_string())),
        NbtSummaryValue::Float(bits) => number(f64::from(f32::from_bits(*bits))),
        NbtSummaryValue::Double(bits) => number(f64::from_bits(*bits)),
        NbtSummaryValue::ByteArray(values) => {
            consume_nodes(budget, values.len(), path)?;
            Ok(Json::Array(
                values
                    .iter()
                    .map(|value| Json::Number(value.to_string()))
                    .collect(),
            ))
        }
        NbtSummaryValue::String(value) => Ok(Json::String(value.clone())),
        NbtSummaryValue::List(values) => {
            if values.len() > *budget {
                return Err(invalid_schema(
                    path,
                    "inline settings exceed the NBT node budget",
                ));
            }
            let mut out = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                out.push(inline_codec_nbt_to_json(
                    value,
                    &format!("{path}[{index}]"),
                    depth + 1,
                    budget,
                )?);
            }
            Ok(Json::Array(out))
        }
        NbtSummaryValue::Compound(entries) => {
            if entries.len() > *budget {
                return Err(invalid_schema(
                    path,
                    "inline settings exceed the NBT node budget",
                ));
            }
            let mut names = BTreeSet::new();
            let mut result = Vec::with_capacity(entries.len());
            for entry in entries {
                let entry_path = format!("{path}.{}", entry.name.to_display_string());
                if !names.insert(&entry.name) {
                    return Err(invalid_schema(
                        &entry_path,
                        "duplicate inline settings field",
                    ));
                }
                result.push((
                    entry.name.clone(),
                    inline_codec_nbt_to_json(&entry.value, &entry_path, depth + 1, budget)?,
                ));
            }
            Ok(Json::Object(result))
        }
        NbtSummaryValue::IntArray(values) => {
            consume_nodes(budget, values.len(), path)?;
            Ok(Json::Array(
                values
                    .iter()
                    .map(|value| Json::Number(value.to_string()))
                    .collect(),
            ))
        }
        NbtSummaryValue::LongArray(values) => {
            consume_nodes(budget, values.len(), path)?;
            Ok(Json::Array(
                values
                    .iter()
                    .map(|value| Json::Number(value.to_string()))
                    .collect(),
            ))
        }
    }
}
