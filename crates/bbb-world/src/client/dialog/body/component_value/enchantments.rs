use std::collections::BTreeSet;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use crate::RegistrySet;

use super::{
    component_i32,
    holder_set::{decode_registry_holder_reference, DialogRegistryHolderReferenceState},
    validate_i32_range,
};

const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogEnchantmentEntryState {
    pub enchantment: DialogRegistryHolderReferenceState,
    pub level: i32,
}

pub(super) fn decode_enchantments(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
    what: &str,
) -> Result<Vec<DialogEnchantmentEntryState>, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not an enchantment map"));
    };
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let mut compound_entries = Vec::<&NbtSummaryEntry>::with_capacity(entries.len());
    for entry in entries {
        if let Some(existing) = compound_entries
            .iter_mut()
            .find(|existing| existing.name == entry.name)
        {
            *existing = entry;
        } else {
            compound_entries.push(entry);
        }
    }
    let mut seen_holders = BTreeSet::new();
    compound_entries
        .iter()
        .map(|entry| {
            let entry_what = format!("{what} enchantment {}", entry.name);
            let key = NbtSummaryValue::String(entry.name.clone());
            let enchantment = decode_registry_holder_reference(
                &key,
                ENCHANTMENT_REGISTRY,
                registries,
                &entry_what,
            )?;
            if !seen_holders.insert(enchantment.holder_id) {
                return Err(format!(
                    "{what} contains duplicate enchantment holder {}",
                    enchantment.resource_id
                ));
            }
            let level = component_i32(&entry.value, &format!("{entry_what} level"))?;
            validate_i32_range(level, 1, 255, &format!("{entry_what} level"))?;
            Ok(DialogEnchantmentEntryState { enchantment, level })
        })
        .collect()
}

#[cfg(test)]
mod tests;
