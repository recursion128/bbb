//! Persistent projections for `RegistryFixedCodec` `Holder<...>` variant
//! components (animal variants, sound variants, and `minecraft:painting/variant`).

use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
};
use bbb_facts::VillagerModelType;
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_protocol::NbtSummaryValue;

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::nbt_support::{nbt_string, normalize_resource_id_units};

/// The datapack registry one variant component's holder belongs to, or `None`
/// for a component this module does not own.
pub(super) fn variant_registry(type_id: i32) -> Option<&'static str> {
    Some(match type_id {
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => "minecraft:villager_type",
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => "minecraft:wolf_variant",
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => "minecraft:wolf_sound_variant",
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => "minecraft:pig_variant",
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => "minecraft:pig_sound_variant",
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => "minecraft:cow_variant",
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => "minecraft:cow_sound_variant",
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => "minecraft:chicken_variant",
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => "minecraft:chicken_sound_variant",
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            "minecraft:zombie_nautilus_variant"
        }
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => "minecraft:frog_variant",
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => "minecraft:painting_variant",
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => "minecraft:cat_variant",
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => "minecraft:cat_sound_variant",
        _ => return None,
    })
}

/// The holder id one summary carries in the field the wire decoder writes.
fn variant_holder_id(type_id: i32, patch: &DataComponentPatchSummary) -> Option<i32> {
    match type_id {
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => patch.villager_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => patch.wolf_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => patch.wolf_sound_variant_id,
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => patch.pig_variant_id,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => patch.pig_sound_variant_id,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => patch.cow_variant_id,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => patch.cow_sound_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => patch.chicken_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => patch.chicken_sound_variant_id,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => patch.zombie_nautilus_variant_id,
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => patch.frog_variant_id,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => patch.painting_variant_id,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => patch.cat_variant_id,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => patch.cat_sound_variant_id,
        _ => None,
    }
}

fn set_variant_holder_id(type_id: i32, patch: &mut DataComponentPatchSummary, holder_id: i32) {
    let field = match type_id {
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => &mut patch.villager_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => &mut patch.wolf_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => &mut patch.wolf_sound_variant_id,
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => &mut patch.pig_variant_id,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => &mut patch.pig_sound_variant_id,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => &mut patch.cow_variant_id,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => &mut patch.cow_sound_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => &mut patch.chicken_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => &mut patch.chicken_sound_variant_id,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            &mut patch.zombie_nautilus_variant_id
        }
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => &mut patch.frog_variant_id,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => &mut patch.painting_variant_id,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => &mut patch.cat_variant_id,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => &mut patch.cat_sound_variant_id,
        _ => unreachable!("only the variant components reach this setter"),
    };
    *field = Some(holder_id);
}

/// `minecraft:painting/variant` is the one variant whose wire form can also be
/// an *inline* variant (`decode_painting_variant_holder_summary`); a direct
/// holder has no writer-bound identifier, so the save form cannot be reached.
fn direct_variant(type_id: i32, patch: &DataComponentPatchSummary) -> bool {
    type_id == VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID
        && patch.painting_variant_direct.is_some()
}

pub(super) fn persistent_variant_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    let registry_id = variant_registry(type_id)
        .expect("the variant dispatch is guarded by the generated component table");
    if direct_variant(type_id, patch) {
        return Err(invalid_persistent_component(
            type_id,
            resource_id,
            format!("an inline {registry_id} variant has no writer-bound identifier"),
        ));
    }
    let holder_id = variant_holder_id(type_id, patch).ok_or(
        ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        },
    )?;
    let name = if type_id == VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID {
        VillagerModelType::for_vanilla_registry_id(holder_id)
            .map(|value| value.resource_id().to_string())
    } else {
        holder_resource_id(registry_id, holder_id)
    }
    .ok_or(
        ItemStackPersistentProjectionError::UnresolvedRegistryHolderName {
            registry_id,
            holder_id,
        },
    )?;
    let name = bbb_protocol::normalize_resource_location(name)
        .map_err(|error| invalid_persistent_component(type_id, resource_id, error.to_string()))?;
    Ok(NbtSummaryValue::String(name.into()))
}

pub(super) fn project_variant(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let registry_id = variant_registry(type_id)
        .expect("the variant dispatch is guarded by the generated component table");
    let named = nbt_string(value).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "variant holder is not a resource id".to_string(),
        )
    })?;
    let named = normalize_resource_id_units(named.units())?;
    let holder_id = if type_id == VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID {
        VillagerModelType::for_resource_id(&named)
            .map(|value| value.vanilla_registry_index() as i32)
            .ok_or_else(
                || ItemStackPersistentProjectionError::UnresolvedRegistryHolder {
                    registry_id,
                    resource_id: named.clone(),
                },
            )?
    } else {
        super::holder::resolve_holder_id(registry_id, &named, holder_id)?
    };
    set_variant_holder_id(type_id, patch, holder_id);
    Ok(())
}
