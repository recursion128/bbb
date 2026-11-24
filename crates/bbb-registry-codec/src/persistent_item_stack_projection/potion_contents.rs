use bbb_facts::{
    data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    potion_types::{vanilla_potion, vanilla_potion_for_resource_id},
};
use bbb_protocol::{nbt_summary::entry, packets::PotionContentsSummary, NbtSummaryValue};

use super::{
    consumable_components::{persistent_mob_effect_instance_nbt, project_mob_effect_instance},
    error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result},
    nbt_support::normalize_resource_id_units,
};
use crate::registry_codec::RegistryElementValueRef;
fn invalid(detail: impl Into<String>) -> ItemStackPersistentProjectionError {
    invalid_persistent_component(
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        "minecraft:potion_contents",
        detail.into(),
    )
}

pub fn read(
    value: RegistryElementValueRef<'_>,
    max_hidden_depth: usize,
) -> Result<PotionContentsSummary> {
    if !value.is_compound() && !value.is_string() {
        return Err(invalid(
            "potion contents is neither a potion id nor a record",
        ));
    }
    let potion = if value.is_string() {
        Some(value)
    } else {
        value.field("potion")
    };
    let potion_id = potion
        .map(|value| {
            let id = value
                .java_string_ref()
                .ok_or_else(|| invalid("potion holder is not a resource ID"))?;
            vanilla_potion_for_resource_id(&normalize_resource_id_units(id.units())?)
                .map(|p| p.id)
                .ok_or_else(|| invalid(format!("unknown potion {id}")))
        })
        .transpose()?;
    let custom_color = value
        .field("custom_color")
        .map(|value| {
            value
                .codec_i32()
                .ok_or_else(|| invalid("custom_color is not numeric"))
        })
        .transpose()?;
    let custom_name = value
        .field("custom_name")
        .map(|value| {
            value
                .java_string()
                .ok_or_else(|| invalid("custom_name is not a string"))
        })
        .transpose()?;
    let custom_effects = match value.field("custom_effects") {
        Some(effects) => effects
            .list_elements()
            .ok_or_else(|| invalid("custom_effects is not a list"))?
            .map(|effect| {
                project_mob_effect_instance(
                    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                    "minecraft:potion_contents",
                    effect,
                    max_hidden_depth,
                )
            })
            .collect::<Result<Vec<_>>>()?,
        None => Vec::new(),
    };
    Ok(PotionContentsSummary {
        potion_id,
        custom_color,
        custom_effects,
        custom_name,
    })
}

pub fn write(contents: &PotionContentsSummary, max_hidden_depth: usize) -> Result<NbtSummaryValue> {
    let mut fields = Vec::new();
    if let Some(id) = contents.potion_id {
        let potion =
            vanilla_potion(id).ok_or_else(|| invalid(format!("unknown potion holder {id}")))?;
        fields.push(entry(
            "potion",
            NbtSummaryValue::String(potion.resource_id.into()),
        ));
    }
    if let Some(color) = contents.custom_color {
        fields.push(entry("custom_color", NbtSummaryValue::Int(color)));
    }
    if let Some(name) = &contents.custom_name {
        fields.push(entry("custom_name", name.to_nbt()));
    }
    if !contents.custom_effects.is_empty() {
        let effects = contents
            .custom_effects
            .iter()
            .map(|effect| {
                persistent_mob_effect_instance_nbt(
                    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                    "minecraft:potion_contents",
                    effect,
                    max_hidden_depth,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        fields.push(entry("custom_effects", NbtSummaryValue::List(effects)));
    }
    Ok(NbtSummaryValue::Compound(fields))
}
