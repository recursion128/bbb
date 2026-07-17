use std::collections::BTreeMap;

use bbb_protocol::{
    decode_nbt_summary,
    packets::{ItemStackSummary, TrimMaterialSummary, TrimPatternSummary},
    NbtSummaryEntry, NbtSummaryValue,
};

use super::{
    normalize_codec_identifier, RegistrySet, VanillaTrimMaterialIdentity,
    VanillaTrimPatternIdentity,
};

pub const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
pub const TRIM_PATTERN_REGISTRY: &str = "minecraft:trim_pattern";

/// The resolved armor-trim atlas sprite projected from one worn item's `trim` component.
///
/// The world boundary keeps the resource id rather than a renderer handle: startup packs own the
/// atlas contents, and native resolves this id only after the matching generated sprite uploads.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArmorTrimSprite {
    pub sprite_id: String,
    pub decal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrimMaterialPresentation {
    asset_name: String,
    override_armor_assets: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrimPatternPresentation {
    asset_id: String,
    decal: bool,
}

/// Vanilla `EquipmentClientInfo.LayerType` values currently consumed by native armor-trim
/// projection. Keeping the serialized layer id here prevents callers from reconstructing atlas
/// paths ad hoc as non-humanoid equipment renderers are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArmorTrimLayer {
    Humanoid,
    HumanoidLeggings,
    HorseBody,
    LlamaBody,
    NautilusBody,
    PigSaddle,
    StriderSaddle,
    Wings,
    WolfBody,
}

impl ArmorTrimLayer {
    const fn asset_id(self) -> &'static str {
        match self {
            Self::Humanoid => "humanoid",
            Self::HumanoidLeggings => "humanoid_leggings",
            Self::HorseBody => "horse_body",
            Self::LlamaBody => "llama_body",
            Self::NautilusBody => "nautilus_body",
            Self::PigSaddle => "pig_saddle",
            Self::StriderSaddle => "strider_saddle",
            Self::Wings => "wings",
            Self::WolfBody => "wolf_body",
        }
    }
}

/// Resolves vanilla `ArmorTrim.layerAssetId(layerType.trimAssetPrefix(), equipmentAsset)` for one
/// equipment layer. Named holders use the synchronized registry payload when present; only a
/// session with no synchronized trim registry uses the source-generated vanilla bootstrap fallback.
pub(crate) fn armor_trim_sprite(
    registries: &RegistrySet,
    item: &ItemStackSummary,
    equipment_asset: &str,
    layer: ArmorTrimLayer,
) -> Option<ArmorTrimSprite> {
    let patch = &item.component_patch;
    let material = if let Some(material) = patch.armor_trim_material_direct.as_deref() {
        direct_material(material)?
    } else {
        named_material(registries, patch.armor_trim_material_id?)?
    };
    let pattern = if let Some(pattern) = patch.armor_trim_pattern_direct.as_deref() {
        direct_pattern(pattern)?
    } else {
        named_pattern(registries, patch.armor_trim_pattern_id?)?
    };
    let material_suffix = material
        .override_armor_assets
        .get(equipment_asset)
        .map_or(material.asset_name.as_str(), String::as_str);
    let (namespace, pattern_path) = pattern.asset_id.split_once(':')?;
    Some(ArmorTrimSprite {
        sprite_id: format!(
            "{namespace}:trims/entity/{}/{pattern_path}_{material_suffix}",
            layer.asset_id()
        ),
        decal: pattern.decal,
    })
}

fn direct_material(material: &TrimMaterialSummary) -> Option<TrimMaterialPresentation> {
    let asset_name = resource_path(&material.asset_name)?.to_string();
    let override_armor_assets = material
        .override_armor_assets
        .iter()
        .map(|(asset, suffix)| {
            Some((
                normalize_codec_identifier(asset)?,
                resource_path(suffix)?.to_string(),
            ))
        })
        .collect::<Option<_>>()?;
    Some(TrimMaterialPresentation {
        asset_name,
        override_armor_assets,
    })
}

fn direct_pattern(pattern: &TrimPatternSummary) -> Option<TrimPatternPresentation> {
    Some(TrimPatternPresentation {
        asset_id: normalize_codec_identifier(&pattern.asset_id)?,
        decal: pattern.decal,
    })
}

fn named_material(registries: &RegistrySet, holder_id: i32) -> Option<TrimMaterialPresentation> {
    if let Some(contents) = registries.contents.get(TRIM_MATERIAL_REGISTRY) {
        return decode_material(contents.entry_by_holder_id(holder_id)?.raw_data()?);
    }
    let material = VanillaTrimMaterialIdentity::for_vanilla_registry_id(holder_id)?;
    Some(TrimMaterialPresentation {
        asset_name: material.asset_name().to_string(),
        override_armor_assets: material_override_map(material),
    })
}

fn material_override_map(material: VanillaTrimMaterialIdentity) -> BTreeMap<String, String> {
    // The generated catalog owns the exact suffix lookup. Probe the finite set of armor equipment
    // assets that current trim projections can receive, retaining only real overrides.
    [
        "minecraft:leather",
        "minecraft:copper",
        "minecraft:chainmail",
        "minecraft:iron",
        "minecraft:gold",
        "minecraft:diamond",
        "minecraft:turtle_scute",
        "minecraft:netherite",
        "minecraft:armadillo_scute",
    ]
    .into_iter()
    .filter_map(|asset| {
        let suffix = material.asset_suffix_for_equipment(asset);
        (suffix != material.asset_name()).then(|| (asset.to_string(), suffix.to_string()))
    })
    .collect()
}

fn named_pattern(registries: &RegistrySet, holder_id: i32) -> Option<TrimPatternPresentation> {
    if let Some(contents) = registries.contents.get(TRIM_PATTERN_REGISTRY) {
        return decode_pattern(contents.entry_by_holder_id(holder_id)?.raw_data()?);
    }
    let pattern = VanillaTrimPatternIdentity::for_vanilla_registry_id(holder_id)?;
    Some(TrimPatternPresentation {
        asset_id: pattern.resource_id().to_string(),
        decal: false,
    })
}

fn decode_material(raw: &[u8]) -> Option<TrimMaterialPresentation> {
    let NbtSummaryValue::Compound(entries) = decode_nbt_summary(raw).ok()?? else {
        return None;
    };
    let asset_name = resource_path(string_field(&entries, "asset_name")?)?.to_string();
    let mut override_armor_assets = BTreeMap::new();
    if let Some(value) = field(&entries, "override_armor_assets") {
        let NbtSummaryValue::Compound(overrides) = value else {
            return None;
        };
        for entry in overrides {
            let asset = normalize_codec_identifier(&entry.name)?;
            let NbtSummaryValue::String(suffix) = &entry.value else {
                return None;
            };
            override_armor_assets.insert(asset, resource_path(suffix)?.to_string());
        }
    }
    Some(TrimMaterialPresentation {
        asset_name,
        override_armor_assets,
    })
}

fn decode_pattern(raw: &[u8]) -> Option<TrimPatternPresentation> {
    let NbtSummaryValue::Compound(entries) = decode_nbt_summary(raw).ok()?? else {
        return None;
    };
    let asset_id = normalize_codec_identifier(string_field(&entries, "asset_id")?)?;
    let decal = match field(&entries, "decal") {
        None => false,
        Some(NbtSummaryValue::Byte(value)) => *value != 0,
        Some(_) => return None,
    };
    Some(TrimPatternPresentation { asset_id, decal })
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn string_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a str> {
    let NbtSummaryValue::String(value) = field(entries, name)? else {
        return None;
    };
    Some(value)
}

fn resource_path(value: &str) -> Option<&str> {
    (!value.is_empty()
        && value.chars().all(|ch| {
            ch == '_'
                || ch == '-'
                || ch == '/'
                || ch == '.'
                || ch.is_ascii_lowercase()
                || ch.is_ascii_digit()
        }))
    .then_some(value)
}
