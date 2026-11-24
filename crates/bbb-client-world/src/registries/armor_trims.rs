use std::collections::BTreeMap;

use bbb_protocol::nbt_summary::{compound_field as field, integer_bool_value};
use bbb_protocol::{
    packets::{is_valid_resource_path, ItemStackSummary, TrimMaterialSummary, TrimPatternSummary},
    styled_component_runs_from_nbt_value, styled_runs_summary_text, validate_component_nbt,
    NbtSummaryValue, StyledTextRun,
};

use crate::WorldStore;

use super::{
    canonical_description, loader_order::loader_ordered_resource_id, normalize_codec_identifier,
    RegistrySet, VanillaTrimMaterialIdentity, VanillaTrimPatternIdentity,
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

impl WorldStore {
    /// Projects one server-synchronized trim material through current-session holder order.
    ///
    /// `None` means there is no synchronized registry, the holder is unknown, or the selected
    /// entry is data-less. This surface never substitutes a vanilla bootstrap ordinal.
    pub fn synchronized_trim_material_presentation(
        &self,
        holder_id: i32,
    ) -> Option<&TrimMaterialPresentation> {
        let entry = self.registry_entry_by_holder_id(TRIM_MATERIAL_REGISTRY, holder_id)?;
        self.registries.typed.trim_materials.get(&entry.id)
    }

    /// Projects one server-synchronized trim pattern through current-session holder order.
    ///
    /// `None` has the same no-substitution meaning as
    /// [`WorldStore::synchronized_trim_material_presentation`].
    pub fn synchronized_trim_pattern_presentation(
        &self,
        holder_id: i32,
    ) -> Option<&TrimPatternPresentation> {
        let entry = self.registry_entry_by_holder_id(TRIM_PATTERN_REGISTRY, holder_id)?;
        self.registries.typed.trim_patterns.get(&entry.id)
    }
}

impl TrimMaterialPresentation {
    fn asset_suffix_for_equipment(&self, equipment_asset: &str) -> &str {
        self.override_armor_assets
            .get(equipment_asset)
            .map_or(self.asset_name.as_str(), String::as_str)
    }
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
    let material;
    let material_suffix = if let Some(direct) = patch.armor_trim_material_direct.as_deref() {
        material = direct_trim_material_presentation(direct)?;
        material.asset_suffix_for_equipment(equipment_asset)
    } else {
        named_material_suffix(registries, patch.armor_trim_material_id?, equipment_asset)?
    };
    let pattern;
    let (asset_id, decal) = if let Some(direct) = patch.armor_trim_pattern_direct.as_deref() {
        pattern = direct_trim_pattern_presentation(direct)?;
        (pattern.asset_id.as_str(), pattern.decal)
    } else {
        named_pattern(registries, patch.armor_trim_pattern_id?)?
    };
    let (namespace, pattern_path) = asset_id.split_once(':')?;
    Some(ArmorTrimSprite {
        sprite_id: format!(
            "{namespace}:trims/entity/{}/{pattern_path}_{material_suffix}",
            layer.asset_id()
        ),
        decal,
    })
}

fn named_material_suffix<'a>(
    registries: &'a RegistrySet,
    holder_id: i32,
    equipment_asset: &str,
) -> Option<&'a str> {
    if let Some(contents) = registries.contents.get(TRIM_MATERIAL_REGISTRY) {
        let entry = contents.entry_by_holder_id(holder_id)?;
        if let Some(presentation) = registries.typed.trim_materials.get(&entry.id) {
            return Some(presentation.asset_suffix_for_equipment(equipment_asset));
        }
        return VanillaTrimMaterialIdentity::for_resource_id(&entry.id)
            .map(|material| material.asset_suffix_for_equipment(equipment_asset));
    }
    // `minecraft:trim_material` is loaded from data files
    // (RegistryDataLoader.java:100) and registered in sorted `Identifier` order
    // (ResourceManagerRegistryLoadTask.java:62), so an unsynchronized holder id
    // is a loader-order index rather than a `TrimMaterials` bootstrap ordinal.
    loader_ordered_resource_id(TRIM_MATERIAL_REGISTRY, holder_id)
        .and_then(VanillaTrimMaterialIdentity::for_resource_id)
        .map(|material| material.asset_suffix_for_equipment(equipment_asset))
}

fn named_pattern(registries: &RegistrySet, holder_id: i32) -> Option<(&str, bool)> {
    if let Some(contents) = registries.contents.get(TRIM_PATTERN_REGISTRY) {
        let entry = contents.entry_by_holder_id(holder_id)?;
        if let Some(presentation) = registries.typed.trim_patterns.get(&entry.id) {
            return Some((presentation.asset_id.as_str(), presentation.decal));
        }
        return VanillaTrimPatternIdentity::for_resource_id(&entry.id)
            .map(|pattern| (pattern.resource_id(), false));
    }
    // `minecraft:trim_pattern` is loaded from data files
    // (RegistryDataLoader.java:99) and registered in sorted `Identifier` order
    // (ResourceManagerRegistryLoadTask.java:62), so an unsynchronized holder id
    // is a loader-order index rather than a `TrimPatterns` bootstrap ordinal.
    loader_ordered_resource_id(TRIM_PATTERN_REGISTRY, holder_id)
        .and_then(VanillaTrimPatternIdentity::for_resource_id)
        .map(|pattern| (pattern.resource_id(), false))
}

pub(super) fn decode_validated_material(
    value: NbtSummaryValue,
) -> Result<TrimMaterialPresentation, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        unreachable!("canonical trim material");
    };
    let Some(NbtSummaryValue::String(asset_name)) = field(&entries, "asset_name") else {
        unreachable!("canonical trim asset name");
    };
    if asset_name.is_empty() {
        return Err("asset_name: invalid resource path".to_string());
    }
    let mut override_armor_assets = BTreeMap::new();
    if let Some(value) = field(&entries, "override_armor_assets") {
        let NbtSummaryValue::Compound(overrides) = value else {
            unreachable!("canonical trim asset overrides");
        };
        for entry in overrides {
            let NbtSummaryValue::String(suffix) = &entry.value else {
                unreachable!("canonical trim asset suffix");
            };
            if suffix.is_empty() {
                return Err("override_armor_assets: invalid resource-path value".to_string());
            }
            override_armor_assets.insert(
                entry
                    .name
                    .try_to_string()
                    .expect("canonical equipment asset"),
                suffix.try_to_string().expect("canonical trim asset suffix"),
            );
        }
    }
    let asset_name = asset_name
        .try_to_string()
        .expect("canonical trim asset name");
    let (description, description_styled, description_nbt) = canonical_description(entries);
    Ok(TrimMaterialPresentation {
        asset_name,
        override_armor_assets,
        description,
        description_styled,
        description_nbt: Some(description_nbt),
    })
}

pub(super) fn decode_validated_pattern(value: NbtSummaryValue) -> TrimPatternPresentation {
    let NbtSummaryValue::Compound(entries) = value else {
        unreachable!("canonical trim pattern");
    };
    let Some(NbtSummaryValue::String(asset_id)) = field(&entries, "asset_id") else {
        unreachable!("canonical trim asset id");
    };
    let decal = integer_bool_value(field(&entries, "decal").expect("canonical trim decal"))
        .expect("canonical trim boolean");
    let asset_id = asset_id.try_to_string().expect("canonical trim asset id");
    let (description, description_styled, description_nbt) = canonical_description(entries);
    TrimPatternPresentation {
        asset_id,
        description,
        description_styled,
        description_nbt: Some(description_nbt),
        decal,
    }
}

fn resource_path(value: &str) -> Option<&str> {
    (!value.is_empty() && is_valid_resource_path(value)).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;
    use bbb_protocol::NbtSummaryEntry;

    #[test]
    fn synchronized_presentations_follow_server_order_and_keep_component_trees() {
        let mut world = WorldStore::new();
        world.begin_registry_configuration();
        super::super::tests::record_minimal_non_empty_synchronized_registries(&mut world, None);
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

        world.finish_registry_configuration().unwrap();

        let material = world.synchronized_trim_material_presentation(0).unwrap();
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
                .asset_name,
            "iron"
        );

        let pattern = world.synchronized_trim_pattern_presentation(0).unwrap();
        assert_eq!(pattern.asset_id, "example:spiral");
        assert_eq!(pattern.description, "Spiral");
        assert!(pattern.decal);
    }

    #[test]
    fn direct_presentations_normalize_assets_and_trust_the_retained_component_tree() {
        let description_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::string("Tree description"),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("#123456"),
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
    fn malformed_material_is_rejected_and_invalid_optional_pattern_decal_defaults() {
        let mut world = WorldStore::new();
        for (asset, overrides) in [
            ("bad suffix", None),
            ("", None),
            ("opal", Some(("minecraft:iron", ""))),
        ] {
            world.begin_registry_configuration();
            super::super::tests::record_minimal_non_empty_synchronized_registries(&mut world, None);
            world.record_registry_entries(
                TRIM_MATERIAL_REGISTRY,
                0,
                vec![RegistryPacketEntry::with_raw_data(
                    "example:broken_material",
                    material_nbt(asset, "Broken", overrides),
                )],
            );
            assert!(world.finish_registry_configuration().is_err());
        }

        world.begin_registry_configuration();
        super::super::tests::record_minimal_non_empty_synchronized_registries(&mut world, None);
        world.record_registry_entries(
            TRIM_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:broken_pattern",
                pattern_nbt_with_invalid_decal(),
            )],
        );
        world.finish_registry_configuration().unwrap();
        assert!(
            !world
                .synchronized_trim_pattern_presentation(0)
                .unwrap()
                .decal
        );
    }

    #[test]
    fn synchronized_queries_never_substitute_vanilla_ordinals_for_stubs_or_unknown_ids() {
        let mut world = WorldStore::new();
        assert_eq!(world.synchronized_trim_material_presentation(1), None);
        assert_eq!(world.synchronized_trim_pattern_presentation(0), None);

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
        assert_eq!(world.synchronized_trim_material_presentation(0), None);
        assert_eq!(world.synchronized_trim_material_presentation(1), None);
        assert_eq!(world.synchronized_trim_pattern_presentation(0), None);
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
            named_material_suffix(&world.registries, 0, "minecraft:diamond").unwrap(),
            "iron"
        );
        assert_eq!(
            named_pattern(&world.registries, 0).unwrap().0,
            "minecraft:spire"
        );
        assert_eq!(
            named_material_suffix(&world.registries, 1, "minecraft:diamond"),
            None
        );
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
