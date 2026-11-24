use bbb_protocol::packets::ItemEnchantmentSummary;
use serde::{Deserialize, Serialize};

use crate::FrozenRegistryAccess;

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogEnchantmentEntryState {
    pub enchantment: DialogRegistryHolderReferenceState,
    pub level: i32,
}

pub(super) fn project_enchantments(
    entries: &[ItemEnchantmentSummary],
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<Vec<DialogEnchantmentEntryState>, String> {
    entries
        .iter()
        .map(|value| {
            let registries = registries.ok_or_else(|| {
                "enchantment holder cannot be decoded without registry context".to_string()
            })?;
            let enchantment = registries
                .require_entry_by_holder_id("minecraft:enchantment", value.holder_id)
                .map_err(|error| error.to_string())?;
            Ok(DialogEnchantmentEntryState {
                enchantment: DialogRegistryHolderReferenceState {
                    holder_id: enchantment.holder_id,
                    resource_id: enchantment.resource_id.to_string(),
                },
                level: value.level,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
