use bbb_protocol::{
    entity_types::vanilla_entity_type, packets::BeeOccupantSummary, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBeeOccupantState {
    pub entity_type: DialogRegistryHolderReferenceState,
    pub entity_data: NbtSummaryValue,
    pub ticks_in_hive: i32,
    pub min_ticks_in_hive: i32,
}

pub(super) fn project_bees(
    bees: &[BeeOccupantSummary],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<Vec<DialogBeeOccupantState>, String> {
    bees.iter()
        .map(|bee| {
            let resource_id = vanilla_entity_type(bee.entity_type_id)
                .map(|entity| entity.resource_id)
                .or_else(|| {
                    (!content_registry.is_vanilla_only())
                        .then(|| {
                            content_registry
                                .entity_by_id(bee.entity_type_id)
                                .map(|spec| spec.resource_id.as_str())
                        })
                        .flatten()
                })
                .ok_or_else(|| format!("unknown bee entity type id {}", bee.entity_type_id))?;
            let entity_data = bee
                .entity_data_tag
                .as_ref()
                .ok_or_else(|| "missing bee entity data tag".to_string())?
                .clone();
            Ok(DialogBeeOccupantState {
                entity_type: DialogRegistryHolderReferenceState {
                    holder_id: bee.entity_type_id,
                    resource_id: resource_id.to_string(),
                },
                entity_data,
                ticks_in_hive: bee.ticks_in_hive,
                min_ticks_in_hive: bee.min_ticks_in_hive,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
