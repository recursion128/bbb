use bbb_protocol::{packets::ItemMapDecorationSummary, JavaString};
use serde::{Deserialize, Serialize};

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogMapDecorationState {
    pub id: JavaString,
    pub decoration_type: DialogRegistryHolderReferenceState,
    pub x_bits: u64,
    pub z_bits: u64,
    pub rotation_bits: u32,
}

pub(super) fn project_map_decorations(
    decorations: &[ItemMapDecorationSummary],
) -> Vec<DialogMapDecorationState> {
    decorations
        .iter()
        .map(|decoration| DialogMapDecorationState {
            id: decoration.id.clone(),
            decoration_type: DialogRegistryHolderReferenceState {
                holder_id: decoration.decoration_type_id,
                resource_id: decoration.decoration_type_resource_id.clone(),
            },
            x_bits: decoration.x_bits,
            z_bits: decoration.z_bits,
            rotation_bits: decoration.rotation_bits,
        })
        .collect()
}

#[cfg(test)]
mod tests;
