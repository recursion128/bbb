use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
    },
    packets::DataComponentPatchSummary,
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::{RegistryContentState, RegistryPacketEntry};

use super::super::{
    apply_item_patch_value, decode_supported_component_value, DialogDataComponentValueState,
};
use super::*;

#[test]
fn decodes_direct_provides_trim_material_with_assets_and_component() {
    let description = compound([
        ("text", string("Quartz")),
        ("italic", NbtSummaryValue::Byte(1)),
    ]);
    let decoded = decode_provides(
        &compound([
            ("asset_name", string("quartz")),
            (
                "override_armor_assets",
                compound([
                    ("iron", string("iron_darker")),
                    ("example:copper", string("nested/copper")),
                ]),
            ),
            ("description", description.clone()),
            ("ignored", NbtSummaryValue::Int(1)),
        ]),
        None,
    )
    .unwrap();
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        decoded.as_ref().unwrap(),
        &mut patch,
    ));
    let projected = patch.provides_trim_material_direct.as_ref().unwrap();
    assert_eq!(projected.asset_name, "quartz");
    assert_eq!(projected.description, "Quartz");
    assert_eq!(
        projected.description_styled.as_ref().unwrap()[0]
            .style
            .italic,
        Some(true)
    );
    let Some(DialogDataComponentValueState::ProvidesTrimMaterial {
        material: DialogTrimMaterialHolderState::Direct { material },
    }) = decoded
    else {
        panic!("expected direct trim material")
    };
    assert_eq!(material.asset_name, "quartz");
    assert_eq!(
        material.override_armor_assets,
        BTreeMap::from([
            ("example:copper".to_string(), "nested/copper".to_string()),
            ("minecraft:iron".to_string(), "iron_darker".to_string()),
        ])
    );
    assert_eq!(material.description.text, "Quartz");
    assert_eq!(material.description.component, description);
    assert_eq!(material.description.runs[0].style.italic, Some(true));
}

#[test]
fn decodes_context_free_direct_armor_trim_and_pattern_defaults() {
    let decoded = decode_trim(
        &compound([
            (
                "material",
                compound([
                    ("asset_name", string("resin")),
                    ("description", string("Resin")),
                ]),
            ),
            (
                "pattern",
                compound([
                    ("asset_id", string("sentry")),
                    ("description", string("Sentry")),
                    ("ignored", NbtSummaryValue::Byte(1)),
                ]),
            ),
            ("ignored", NbtSummaryValue::Int(1)),
        ]),
        None,
    )
    .unwrap();
    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        decoded.as_ref().unwrap(),
        &mut patch,
    ));
    assert_eq!(
        patch
            .armor_trim_material_direct
            .as_ref()
            .unwrap()
            .description,
        "Resin"
    );
    assert_eq!(
        patch.armor_trim_pattern_direct.as_ref().unwrap().asset_id,
        "minecraft:sentry"
    );
    let Some(DialogDataComponentValueState::Trim { material, pattern }) = decoded else {
        panic!("expected armor trim")
    };
    let DialogTrimMaterialHolderState::Direct { material } = material else {
        panic!("expected direct material")
    };
    assert_eq!(material.asset_name, "resin");
    assert!(material.override_armor_assets.is_empty());
    assert_eq!(material.description.text, "Resin");
    let DialogTrimPatternHolderState::Direct { pattern } = pattern else {
        panic!("expected direct pattern")
    };
    assert_eq!(pattern.asset_id, "minecraft:sentry");
    assert_eq!(pattern.description.text, "Sentry");
    assert!(!pattern.decal);
}

#[test]
fn decodes_mixed_named_and_direct_armor_trim_holders() {
    let mut registries = RegistrySet::default();
    let mut materials = RegistryContentState::new(TRIM_MATERIAL_REGISTRY);
    materials.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:quartz"),
    ];
    registries
        .contents
        .insert(TRIM_MATERIAL_REGISTRY.to_string(), materials);

    let decoded = decode_trim(
        &compound([
            ("material", string("quartz")),
            (
                "pattern",
                compound([
                    ("asset_id", string("example:spiral")),
                    (
                        "description",
                        compound([
                            ("text", string("Spiral")),
                            ("bold", NbtSummaryValue::Byte(1)),
                        ]),
                    ),
                    ("decal", NbtSummaryValue::Int(1)),
                ]),
            ),
        ]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Trim { material, pattern }) = decoded else {
        panic!("expected armor trim")
    };
    assert_eq!(
        material,
        DialogTrimMaterialHolderState::Named {
            registry_id: 1,
            resource_id: "minecraft:quartz".to_string(),
        }
    );
    let DialogTrimPatternHolderState::Direct { pattern } = pattern else {
        panic!("expected direct pattern")
    };
    assert_eq!(pattern.asset_id, "example:spiral");
    assert!(pattern.decal);
    assert_eq!(pattern.description.runs[0].style.bold, Some(true));
}

#[test]
fn rejects_malformed_direct_trim_materials() {
    for (value, expected) in [
        (NbtSummaryValue::List(Vec::new()), "neither a resource id"),
        (
            compound([("description", string("Quartz"))]),
            "missing field asset_name",
        ),
        (
            compound([("asset_name", string("quartz"))]),
            "missing field description",
        ),
        (
            material_with_override(string("Bad:Path")),
            "equipment asset minecraft:iron resource path",
        ),
        (
            compound([
                ("asset_name", string("bad:path")),
                ("description", string("Quartz")),
            ]),
            "invalid",
        ),
        (
            compound([
                ("asset_name", string("quartz")),
                ("override_armor_assets", NbtSummaryValue::List(Vec::new())),
                ("description", string("Quartz")),
            ]),
            "is not a map",
        ),
        (
            compound([
                ("asset_name", string("quartz")),
                (
                    "override_armor_assets",
                    compound([("Bad", string("darker"))]),
                ),
                ("description", string("Quartz")),
            ]),
            "equipment asset Bad",
        ),
        (
            compound([
                ("asset_name", string("quartz")),
                (
                    "override_armor_assets",
                    compound([
                        ("iron", string("first")),
                        ("minecraft:iron", string("second")),
                    ]),
                ),
                ("description", string("Quartz")),
            ]),
            "duplicate",
        ),
        (
            compound([
                ("asset_name", string("quartz")),
                ("description", NbtSummaryValue::Int(1)),
            ]),
            "is not a component",
        ),
    ] {
        let error = decode_provides(&value, None).unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

#[test]
fn rejects_malformed_armor_trim_and_direct_patterns() {
    let material = || {
        compound([
            ("asset_name", string("quartz")),
            ("description", string("Quartz")),
        ])
    };
    for (pattern, expected) in [
        (
            compound([("description", string("Coast"))]),
            "missing field asset_id",
        ),
        (
            compound([("asset_id", string("coast"))]),
            "missing field description",
        ),
        (
            compound([
                ("asset_id", string("Bad")),
                ("description", string("Coast")),
            ]),
            "invalid",
        ),
        (
            compound([
                ("asset_id", string("coast")),
                ("description", NbtSummaryValue::Int(1)),
            ]),
            "is not a component",
        ),
        (
            compound([
                ("asset_id", string("coast")),
                ("description", string("Coast")),
                ("decal", string("true")),
            ]),
            "is not a boolean",
        ),
    ] {
        let error = decode_trim(
            &compound([("material", material()), ("pattern", pattern)]),
            None,
        )
        .unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }

    for (value, expected) in [
        (NbtSummaryValue::List(Vec::new()), "is not a compound"),
        (
            compound([("pattern", string("coast"))]),
            "missing field material",
        ),
        (
            compound([("material", material())]),
            "missing field pattern",
        ),
        (
            compound([
                ("material", material()),
                ("pattern", NbtSummaryValue::List(Vec::new())),
            ]),
            "neither a resource id",
        ),
    ] {
        let error = decode_trim(&value, None).unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} did not contain {expected:?}"
        );
    }
}

fn material_with_override(suffix: NbtSummaryValue) -> NbtSummaryValue {
    compound([
        ("asset_name", string("quartz")),
        ("override_armor_assets", compound([("iron", suffix)])),
        ("description", string("Quartz")),
    ])
}

fn decode_provides(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        "minecraft:provides_trim_material",
        value,
        registries,
    )
}

fn decode_trim(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        "minecraft:trim",
        value,
        registries,
    )
}

fn compound<const N: usize>(entries: [(&str, NbtSummaryValue); N]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| entry(name, value))
            .collect(),
    )
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}
