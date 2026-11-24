//! Persistent projections for `StringRepresentable` enum components
//! (`minecraft:dye` and animal color/variant enums) using catalog ordinals.

use bbb_facts::builtin_enum_component_values::{
    builtin_enum_component_value_for_name, builtin_enum_component_value_name,
    normalize_builtin_enum_component_value_id,
};
use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
};
use bbb_facts::tropical_fish_patterns::{
    vanilla_tropical_fish_pattern, vanilla_tropical_fish_pattern_for_name,
};
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_protocol::NbtSummaryValue;

use super::error::{invalid_persistent_component, Result};
use super::nbt_support::nbt_string;

/// The components `DataComponentPatchSummary::builtin_enum_component_ids`
/// carries, i.e. everything `catalog/decode.rs`'s `decode_builtin_enum` owns
/// except `minecraft:base_color`, which the projection already encodes, and
/// `minecraft:dye`, which has a field of its own.
pub(super) const BUILTIN_ENUM_COMPONENT_TYPE_IDS: &[i32] = &[
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
    bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
];

pub(super) fn is_builtin_enum_component(type_id: i32) -> bool {
    BUILTIN_ENUM_COMPONENT_TYPE_IDS.contains(&type_id)
}

/// The three enum components whose id lives in a summary field of its own
/// rather than the shared `builtin_enum_component_ids` map: `decode_dye`,
/// `decode_tropical_fish_base_color` and
/// `decode_tropical_fish_pattern_color` each own a field, and
/// `validate_encodable_data_component_patch_summary` rebuilds exactly those.
fn dedicated_enum_field(type_id: i32, patch: &DataComponentPatchSummary) -> Option<Option<i32>> {
    match type_id {
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID => Some(patch.dye_color_id),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => {
            Some(patch.tropical_fish_base_color_id)
        }
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => {
            Some(patch.tropical_fish_pattern_color_id)
        }
        _ => None,
    }
}

fn set_dedicated_enum_field(type_id: i32, patch: &mut DataComponentPatchSummary, id: i32) {
    match type_id {
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID => patch.dye_color_id = Some(id),
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => {
            patch.tropical_fish_base_color_id = Some(id)
        }
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => {
            patch.tropical_fish_pattern_color_id = Some(id)
        }
        _ => unreachable!("only the dedicated enum fields reach this setter"),
    }
}

/// The enum value id one summary carries, or `None` when the component names
/// none.
fn builtin_enum_value_id(type_id: i32, patch: &DataComponentPatchSummary) -> Option<i32> {
    match dedicated_enum_field(type_id, patch) {
        Some(value) => value,
        None => patch.builtin_enum_component_ids.get(&type_id).copied(),
    }
}

pub(super) fn persistent_builtin_enum_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
) -> Result<NbtSummaryValue> {
    if type_id == VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID {
        let packed_id = patch.tropical_fish_pattern_id.ok_or(
            super::error::ItemStackPersistentProjectionError::UnsupportedComponent {
                type_id,
                resource_id,
            },
        )?;
        // `vanilla_tropical_fish_pattern` falls back to the first entry for an
        // unknown id, so the same canonicality check the network encoder makes
        // is what refuses one (`catalog/decode.rs`'s
        // `encode_tropical_fish_pattern`).
        let pattern = vanilla_tropical_fish_pattern(packed_id);
        if pattern.packed_id != packed_id {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                format!("unsupported tropical_fish_pattern packed id {packed_id}"),
            ));
        }
        return Ok(NbtSummaryValue::String(pattern.serialized_name.into()));
    }
    let value_id = builtin_enum_value_id(type_id, patch).ok_or(
        super::error::ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        },
    )?;
    // The network encoder refuses an id its `ByIdMap` cannot spell
    // (`catalog/decode.rs`'s `encode_builtin_enum`), so the save form cannot be
    // reached from one either.
    if normalize_builtin_enum_component_value_id(type_id, value_id) != Some(value_id) {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("unknown enum value id {value_id}"),
        ));
    }
    let name = builtin_enum_component_value_name(type_id, value_id).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            format!("unknown enum value id {value_id}"),
        )
    })?;
    Ok(NbtSummaryValue::String(name.into()))
}

pub(super) fn project_builtin_enum(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let name = nbt_string(value).ok_or_else(|| {
        invalid_persistent_component(type_id, resource_id, "enum value is not a name".to_string())
    })?;
    if type_id == VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID {
        let pattern = name
            .with_utf8(vanilla_tropical_fish_pattern_for_name)
            .flatten()
            .ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("unknown enum value {name}"),
                )
            })?;
        patch.tropical_fish_pattern_id = Some(pattern.packed_id);
        return Ok(());
    }
    let entry = name
        .with_utf8(|name| builtin_enum_component_value_for_name(type_id, name))
        .flatten()
        .ok_or_else(|| {
            invalid_persistent_component(type_id, resource_id, format!("unknown enum value {name}"))
        })?;
    if dedicated_enum_field(type_id, patch).is_some() {
        set_dedicated_enum_field(type_id, patch, entry.id);
    } else {
        patch.builtin_enum_component_ids.insert(type_id, entry.id);
    }
    Ok(())
}
