use std::collections::BTreeMap;

use bbb_protocol::{
    decode_nbt_summary,
    packets::{ItemStackSummary, TrimMaterialSummary, TrimPatternSummary},
    styled_component_runs_from_nbt_value, styled_runs_summary_text, validate_component_nbt,
    NbtSummaryEntry, NbtSummaryValue, StyledTextRun,
};

use crate::WorldStore;

use super::{
    normalize_codec_identifier, RegistrySet, VanillaTrimMaterialIdentity,
    VanillaTrimPatternIdentity,
};

pub const TRIM_MATERIAL_REGISTRY: &str = "minecraft:trim_material";
pub const TRIM_PATTERN_REGISTRY: &str = "minecraft:trim_pattern";

/// Client-consumed fields from vanilla 26.1 `TrimMaterial.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrimMaterialPresentation {
    pub asset_name: String,
    pub override_armor_assets: BTreeMap<String, String>,
    pub description: String,
    pub description_styled: Vec<StyledTextRun>,
    /// Present for synchronized registry payloads and current direct wire values. The option keeps
    /// older serialized summaries readable without pretending their flattened text is the original
    /// Component tree.
    pub description_nbt: Option<NbtSummaryValue>,
}

/// Client-consumed fields from vanilla 26.1 `TrimPattern.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrimPatternPresentation {
    pub asset_id: String,
    pub description: String,
    pub description_styled: Vec<StyledTextRun>,
    /// See [`TrimMaterialPresentation::description_nbt`].
    pub description_nbt: Option<NbtSummaryValue>,
    pub decal: bool,
}

/// Exact synchronized trim registry field that failed its vanilla direct codec contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimRegistryProjectionError {
    pub registry: &'static str,
    pub resource_id: String,
    pub field: String,
    pub reason: &'static str,
}

impl std::fmt::Display for TrimRegistryProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {} {}: {}",
            self.registry, self.resource_id, self.field, self.reason
        )
    }
}

impl std::error::Error for TrimRegistryProjectionError {}

impl WorldStore {
    /// Projects one server-synchronized trim material through current-session holder order.
    ///
    /// `None` means there is no synchronized registry, the holder is unknown, or the selected
    /// entry is data-less. This surface never substitutes a vanilla bootstrap ordinal.
    pub fn synchronized_trim_material_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<TrimMaterialPresentation>, TrimRegistryProjectionError> {
        synchronized_presentation(self, TRIM_MATERIAL_REGISTRY, holder_id, decode_material)
    }

    /// Projects one server-synchronized trim pattern through current-session holder order.
    ///
    /// `None` has the same no-substitution meaning as
    /// [`WorldStore::synchronized_trim_material_presentation`].
    pub fn synchronized_trim_pattern_presentation(
        &self,
        holder_id: i32,
    ) -> Result<Option<TrimPatternPresentation>, TrimRegistryProjectionError> {
        synchronized_presentation(self, TRIM_PATTERN_REGISTRY, holder_id, decode_pattern)
    }
}

fn synchronized_presentation<T>(
    world: &WorldStore,
    registry: &'static str,
    holder_id: i32,
    decode: fn(&[u8]) -> Result<T, DecodeFailure>,
) -> Result<Option<T>, TrimRegistryProjectionError> {
    let Some(entry) = world.registry_entry_by_holder_id(registry, holder_id) else {
        return Ok(None);
    };
    let Some(raw_data) = entry.raw_data() else {
        return if entry.has_data {
            Err(TrimRegistryProjectionError {
                registry,
                resource_id: entry.id.clone(),
                field: "<root>".to_string(),
                reason: "payload bytes unavailable",
            })
        } else {
            Ok(None)
        };
    };
    decode(raw_data)
        .map(Some)
        .map_err(|failure| TrimRegistryProjectionError {
            registry,
            resource_id: entry.id.clone(),
            field: failure.field.to_string(),
            reason: failure.reason,
        })
}

/// Validates and normalizes a direct trim material decoded from an item component.
pub fn direct_trim_material_presentation(
    material: &TrimMaterialSummary,
) -> Option<TrimMaterialPresentation> {
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
    let (description, description_styled, description_nbt) = direct_description(
        &material.description,
        material.description_styled.as_deref(),
        material.description_nbt.as_deref(),
    )?;
    Some(TrimMaterialPresentation {
        asset_name,
        override_armor_assets,
        description,
        description_styled,
        description_nbt,
    })
}

/// Validates and normalizes a direct trim pattern decoded from an item component.
pub fn direct_trim_pattern_presentation(
    pattern: &TrimPatternSummary,
) -> Option<TrimPatternPresentation> {
    let (description, description_styled, description_nbt) = direct_description(
        &pattern.description,
        pattern.description_styled.as_deref(),
        pattern.description_nbt.as_deref(),
    )?;
    Some(TrimPatternPresentation {
        asset_id: normalize_codec_identifier(&pattern.asset_id)?,
        description,
        description_styled,
        description_nbt,
        decal: pattern.decal,
    })
}

fn direct_description(
    description: &str,
    description_styled: Option<&[StyledTextRun]>,
    description_nbt: Option<&NbtSummaryValue>,
) -> Option<(String, Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    if let Some(description_nbt) = description_nbt {
        validate_component_nbt(description_nbt, "direct trim description").ok()?;
        let description_styled = styled_component_runs_from_nbt_value(description_nbt);
        let description = styled_runs_summary_text(&description_styled);
        return Some((
            description,
            description_styled,
            Some(description_nbt.clone()),
        ));
    }
    let description_styled = match description_styled {
        Some(runs) if !runs.is_empty() => runs.to_vec(),
        _ => vec![StyledTextRun {
            text: description.to_string(),
            style: Default::default(),
        }],
    };
    Some((description.to_string(), description_styled, None))
}

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
struct TrimMaterialAssets {
    asset_name: String,
    override_armor_assets: BTreeMap<String, String>,
}

impl From<TrimMaterialPresentation> for TrimMaterialAssets {
    fn from(material: TrimMaterialPresentation) -> Self {
        Self {
            asset_name: material.asset_name,
            override_armor_assets: material.override_armor_assets,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrimPatternAsset {
    asset_id: String,
    decal: bool,
}

impl From<TrimPatternPresentation> for TrimPatternAsset {
    fn from(pattern: TrimPatternPresentation) -> Self {
        Self {
            asset_id: pattern.asset_id,
            decal: pattern.decal,
        }
    }
}

/// Vanilla `EquipmentClientInfo.LayerType` values currently consumed by native armor-trim
/// projection. Keeping the serialized layer id here prevents callers from reconstructing atlas
/// paths ad hoc as non-humanoid equipment renderers are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArmorTrimLayer {
    CamelHuskSaddle,
    CamelSaddle,
    DonkeySaddle,
    HorseSaddle,
    HappyGhastBody,
    Humanoid,
    HumanoidLeggings,
    HorseBody,
    LlamaBody,
    MuleSaddle,
    NautilusBody,
    NautilusSaddle,
    PigSaddle,
    SkeletonHorseSaddle,
    StriderSaddle,
    Wings,
    WolfBody,
    ZombieHorseSaddle,
}

impl ArmorTrimLayer {
    const fn asset_id(self) -> &'static str {
        match self {
            Self::CamelHuskSaddle => "camel_husk_saddle",
            Self::CamelSaddle => "camel_saddle",
            Self::DonkeySaddle => "donkey_saddle",
            Self::HorseSaddle => "horse_saddle",
            Self::HappyGhastBody => "happy_ghast_body",
            Self::Humanoid => "humanoid",
            Self::HumanoidLeggings => "humanoid_leggings",
            Self::HorseBody => "horse_body",
            Self::LlamaBody => "llama_body",
            Self::MuleSaddle => "mule_saddle",
            Self::NautilusBody => "nautilus_body",
            Self::NautilusSaddle => "nautilus_saddle",
            Self::PigSaddle => "pig_saddle",
            Self::SkeletonHorseSaddle => "skeleton_horse_saddle",
            Self::StriderSaddle => "strider_saddle",
            Self::Wings => "wings",
            Self::WolfBody => "wolf_body",
            Self::ZombieHorseSaddle => "zombie_horse_saddle",
        }
    }
}

/// Resolves vanilla `ArmorTrim.layerAssetId(layerType.trimAssetPrefix(), equipmentAsset)` for one
/// equipment layer. Named holders use synchronized payloads when present. A known vanilla
/// data-less entry may use generated metadata by resource identity; only a session with no
/// synchronized trim registry may use the source-generated vanilla bootstrap order.
pub(crate) fn armor_trim_sprite(
    registries: &RegistrySet,
    item: &ItemStackSummary,
    equipment_asset: &str,
    layer: ArmorTrimLayer,
) -> Option<ArmorTrimSprite> {
    let patch = &item.component_patch;
    let material = if let Some(material) = patch.armor_trim_material_direct.as_deref() {
        direct_trim_material_presentation(material)?.into()
    } else {
        named_material(registries, patch.armor_trim_material_id?)?
    };
    let pattern = if let Some(pattern) = patch.armor_trim_pattern_direct.as_deref() {
        direct_trim_pattern_presentation(pattern)?.into()
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

fn named_material(registries: &RegistrySet, holder_id: i32) -> Option<TrimMaterialAssets> {
    if let Some(contents) = registries.contents.get(TRIM_MATERIAL_REGISTRY) {
        let entry = contents.entry_by_holder_id(holder_id)?;
        return match entry.raw_data() {
            Some(raw_data) => decode_material(raw_data).ok().map(Into::into),
            None => {
                VanillaTrimMaterialIdentity::for_resource_id(&entry.id).map(static_material_assets)
            }
        };
    }
    VanillaTrimMaterialIdentity::for_vanilla_registry_id(holder_id).map(static_material_assets)
}

fn static_material_assets(material: VanillaTrimMaterialIdentity) -> TrimMaterialAssets {
    TrimMaterialAssets {
        asset_name: material.asset_name().to_string(),
        override_armor_assets: material_override_map(material),
    }
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

fn named_pattern(registries: &RegistrySet, holder_id: i32) -> Option<TrimPatternAsset> {
    if let Some(contents) = registries.contents.get(TRIM_PATTERN_REGISTRY) {
        let entry = contents.entry_by_holder_id(holder_id)?;
        return match entry.raw_data() {
            Some(raw_data) => decode_pattern(raw_data).ok().map(Into::into),
            None => {
                VanillaTrimPatternIdentity::for_resource_id(&entry.id).map(static_pattern_asset)
            }
        };
    }
    VanillaTrimPatternIdentity::for_vanilla_registry_id(holder_id).map(static_pattern_asset)
}

fn static_pattern_asset(pattern: VanillaTrimPatternIdentity) -> TrimPatternAsset {
    TrimPatternAsset {
        asset_id: pattern.resource_id().to_string(),
        decal: false,
    }
}

fn decode_material(raw_data: &[u8]) -> Result<TrimMaterialPresentation, DecodeFailure> {
    let entries = root_compound(raw_data)?;
    let asset_name = required_resource_path(&entries, "asset_name")?;
    let mut override_armor_assets = BTreeMap::new();
    if let Some(value) = field(&entries, "override_armor_assets") {
        let NbtSummaryValue::Compound(overrides) = value else {
            return Err(failure("override_armor_assets", "expected compound"));
        };
        for entry in overrides {
            let Some(asset) = normalize_codec_identifier(&entry.name) else {
                return Err(failure(
                    "override_armor_assets",
                    "invalid equipment asset identifier",
                ));
            };
            let NbtSummaryValue::String(suffix) = &entry.value else {
                return Err(failure(
                    "override_armor_assets",
                    "expected resource-path string values",
                ));
            };
            let Some(suffix) = resource_path(suffix) else {
                return Err(failure(
                    "override_armor_assets",
                    "invalid resource-path value",
                ));
            };
            override_armor_assets.insert(asset, suffix.to_string());
        }
    }
    let (description, description_styled, description_nbt) = required_description(&entries)?;
    Ok(TrimMaterialPresentation {
        asset_name,
        override_armor_assets,
        description,
        description_styled,
        description_nbt: Some(description_nbt),
    })
}

fn decode_pattern(raw_data: &[u8]) -> Result<TrimPatternPresentation, DecodeFailure> {
    let entries = root_compound(raw_data)?;
    let asset_id = required_string(&entries, "asset_id")?;
    let asset_id = normalize_codec_identifier(asset_id)
        .ok_or_else(|| failure("asset_id", "invalid identifier"))?;
    let (description, description_styled, description_nbt) = required_description(&entries)?;
    let decal = match field(&entries, "decal") {
        None => false,
        Some(NbtSummaryValue::Byte(value)) => *value != 0,
        Some(_) => return Err(failure("decal", "expected boolean byte")),
    };
    Ok(TrimPatternPresentation {
        asset_id,
        description,
        description_styled,
        description_nbt: Some(description_nbt),
        decal,
    })
}

fn root_compound(raw_data: &[u8]) -> Result<Vec<NbtSummaryEntry>, DecodeFailure> {
    let root = decode_nbt_summary(raw_data)
        .map_err(|_| failure("<root>", "invalid NBT"))?
        .ok_or_else(|| failure("<root>", "missing root compound"))?;
    let NbtSummaryValue::Compound(entries) = root else {
        return Err(failure("<root>", "expected compound"));
    };
    Ok(entries)
}

fn required_description(
    entries: &[NbtSummaryEntry],
) -> Result<(String, Vec<StyledTextRun>, NbtSummaryValue), DecodeFailure> {
    let description_nbt = field(entries, "description")
        .ok_or_else(|| failure("description", "missing required field"))?
        .clone();
    validate_component_nbt(&description_nbt, "trim description")
        .map_err(|_| failure("description", "invalid Component"))?;
    let description_styled = styled_component_runs_from_nbt_value(&description_nbt);
    let description = styled_runs_summary_text(&description_styled);
    Ok((description, description_styled, description_nbt))
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn required_string<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &'static str,
) -> Result<&'a str, DecodeFailure> {
    match field(entries, name) {
        Some(NbtSummaryValue::String(value)) => Ok(value),
        Some(_) => Err(failure(name, "expected string")),
        None => Err(failure(name, "missing required field")),
    }
}

fn required_resource_path(
    entries: &[NbtSummaryEntry],
    name: &'static str,
) -> Result<String, DecodeFailure> {
    resource_path(required_string(entries, name)?)
        .map(str::to_string)
        .ok_or_else(|| failure(name, "invalid resource path"))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecodeFailure {
    field: &'static str,
    reason: &'static str,
}

const fn failure(field: &'static str, reason: &'static str) -> DecodeFailure {
    DecodeFailure { field, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn synchronized_presentations_follow_server_order_and_keep_component_trees() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            TRIM_MATERIAL_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:opal",
                    material_nbt("opal", "Opal", Some(("minecraft:iron", "opal_darker"))),
                ),
                RegistryPacketEntry::with_raw_data(
                    "minecraft:iron",
                    material_nbt("iron", "Iron", None),
                ),
            ],
        );
        world.record_registry_entries(
            TRIM_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:spiral",
                pattern_nbt("example:spiral", "Spiral", true),
            )],
        );

        let material = world
            .synchronized_trim_material_presentation(0)
            .unwrap()
            .unwrap();
        assert_eq!(material.asset_name, "opal");
        assert_eq!(
            material.override_armor_assets["minecraft:iron"],
            "opal_darker"
        );
        assert_eq!(material.description, "Opal");
        assert_eq!(material.description_styled[0].style.color, Some(0x12_34_56));
        assert!(matches!(
            material.description_nbt,
            Some(NbtSummaryValue::Compound(_))
        ));
        assert_eq!(
            world
                .synchronized_trim_material_presentation(1)
                .unwrap()
                .unwrap()
                .asset_name,
            "iron"
        );

        let pattern = world
            .synchronized_trim_pattern_presentation(0)
            .unwrap()
            .unwrap();
        assert_eq!(pattern.asset_id, "example:spiral");
        assert_eq!(pattern.description, "Spiral");
        assert!(pattern.decal);
    }

    #[test]
    fn direct_presentations_normalize_assets_and_trust_the_retained_component_tree() {
        let description_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Tree description".to_string()),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("#123456".to_string()),
            },
        ]);
        let material = direct_trim_material_presentation(&TrimMaterialSummary {
            asset_name: "opal".to_string(),
            override_armor_assets: BTreeMap::from([(
                "iron".to_string(),
                "opal_darker".to_string(),
            )]),
            description: "stale flattened text".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(description_nbt.clone())),
        })
        .unwrap();
        assert_eq!(
            material.override_armor_assets["minecraft:iron"],
            "opal_darker"
        );
        assert_eq!(material.description, "Tree description");
        assert_eq!(material.description_styled[0].style.color, Some(0x12_34_56));
        assert_eq!(material.description_nbt, Some(description_nbt.clone()));

        let pattern = direct_trim_pattern_presentation(&TrimPatternSummary {
            asset_id: "spiral".to_string(),
            description: "stale flattened text".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(description_nbt)),
            decal: true,
        })
        .unwrap();
        assert_eq!(pattern.asset_id, "minecraft:spiral");
        assert_eq!(pattern.description, "Tree description");
        assert!(pattern.decal);

        let malformed = TrimPatternSummary {
            asset_id: "example:broken".to_string(),
            description: "Broken".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(NbtSummaryValue::Int(1))),
            decal: false,
        };
        assert_eq!(direct_trim_pattern_presentation(&malformed), None);
    }

    #[test]
    fn malformed_presentations_report_registry_resource_and_codec_field() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            TRIM_MATERIAL_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:broken_material",
                material_nbt("bad suffix", "Broken", None),
            )],
        );
        world.record_registry_entries(
            TRIM_PATTERN_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:broken_pattern",
                    pattern_nbt_with_invalid_decal(),
                ),
                RegistryPacketEntry::with_data_len("example:missing_payload", 12),
            ],
        );

        assert_eq!(
            world
                .synchronized_trim_material_presentation(0)
                .unwrap_err(),
            TrimRegistryProjectionError {
                registry: TRIM_MATERIAL_REGISTRY,
                resource_id: "example:broken_material".to_string(),
                field: "asset_name".to_string(),
                reason: "invalid resource path",
            }
        );
        assert_eq!(
            world.synchronized_trim_pattern_presentation(0).unwrap_err(),
            TrimRegistryProjectionError {
                registry: TRIM_PATTERN_REGISTRY,
                resource_id: "example:broken_pattern".to_string(),
                field: "decal".to_string(),
                reason: "expected boolean byte",
            }
        );
        assert_eq!(
            world.synchronized_trim_pattern_presentation(1).unwrap_err(),
            TrimRegistryProjectionError {
                registry: TRIM_PATTERN_REGISTRY,
                resource_id: "example:missing_payload".to_string(),
                field: "<root>".to_string(),
                reason: "payload bytes unavailable",
            }
        );
    }

    #[test]
    fn synchronized_queries_never_substitute_vanilla_ordinals_for_stubs_or_unknown_ids() {
        let mut world = WorldStore::new();
        assert_eq!(world.synchronized_trim_material_presentation(1), Ok(None));
        assert_eq!(world.synchronized_trim_pattern_presentation(0), Ok(None));

        world.record_registry_entries(
            TRIM_MATERIAL_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("minecraft:iron")],
        );
        world.record_registry_entries(
            TRIM_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("example:custom")],
        );
        assert_eq!(world.synchronized_trim_material_presentation(0), Ok(None));
        assert_eq!(world.synchronized_trim_material_presentation(1), Ok(None));
        assert_eq!(world.synchronized_trim_pattern_presentation(0), Ok(None));
    }

    #[test]
    fn renderer_stub_fallback_uses_resource_identity_not_holder_ordinal() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            TRIM_MATERIAL_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("minecraft:iron")],
        );
        world.record_registry_entries(
            TRIM_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("minecraft:spire")],
        );

        assert_eq!(
            named_material(&world.registries, 0).unwrap().asset_name,
            "iron"
        );
        assert_eq!(
            named_pattern(&world.registries, 0).unwrap().asset_id,
            "minecraft:spire"
        );
        assert_eq!(named_material(&world.registries, 1), None);
    }

    fn material_nbt(
        asset_name: &str,
        description: &str,
        material_override: Option<(&str, &str)>,
    ) -> Vec<u8> {
        let mut out = vec![10];
        write_string(&mut out, "asset_name", asset_name);
        if let Some((asset, suffix)) = material_override {
            out.push(10);
            write_nbt_string(&mut out, "override_armor_assets");
            write_string(&mut out, asset, suffix);
            out.push(0);
        }
        write_description(&mut out, description);
        out.push(0);
        out
    }

    fn pattern_nbt(asset_id: &str, description: &str, decal: bool) -> Vec<u8> {
        let mut out = vec![10];
        write_string(&mut out, "asset_id", asset_id);
        write_description(&mut out, description);
        out.push(1);
        write_nbt_string(&mut out, "decal");
        out.push(u8::from(decal));
        out.push(0);
        out
    }

    fn pattern_nbt_with_invalid_decal() -> Vec<u8> {
        let mut out = vec![10];
        write_string(&mut out, "asset_id", "example:broken");
        write_description(&mut out, "Broken");
        write_string(&mut out, "decal", "true");
        out.push(0);
        out
    }

    fn write_description(out: &mut Vec<u8>, text: &str) {
        out.push(10);
        write_nbt_string(out, "description");
        write_string(out, "text", text);
        write_string(out, "color", "#123456");
        out.push(0);
    }

    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
