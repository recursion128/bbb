use crate::Result;

use super::{
    nbt::{decode_nbt_root, find_entry, NbtValue},
    state::EndGatewayBlockEntityData,
};

/// Decodes the end gateway's `Age` field from a block-entity NBT payload.
/// Vanilla `TheEndGatewayBlockEntity.loadAdditional` reads `Age` with default
/// `0`; the teleport cooldown is not saved and arrives through BlockEvent(1).
pub(crate) fn decode_end_gateway_block_entity_data(
    raw_nbt: &[u8],
) -> Result<Option<EndGatewayBlockEntityData>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let Some(NbtValue::Long(age)) = find_entry(&entries, "Age") else {
        return Ok(None);
    };
    Ok(Some(EndGatewayBlockEntityData { age: *age }))
}
