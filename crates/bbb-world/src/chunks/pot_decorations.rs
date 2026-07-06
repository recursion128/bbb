use crate::Result;

use super::{
    nbt::{decode_nbt_root, find_entry, NbtValue},
    state::DecoratedPotSherdsState,
};

/// Vanilla `PotDecorations.getItem`: `Items.BRICK` marks an undecorated face
/// (`PotDecorations.java:41-48`), and `ordered()` pads missing faces with it.
const VANILLA_POT_EMPTY_SHERD_ITEM: &str = "minecraft:brick";

/// Decodes the decorated pot's `sherds` list from a block-entity NBT payload
/// (`DecoratedPotBlockEntity.loadAdditional` reading `PotDecorations.CODEC` —
/// `BuiltInRegistries.ITEM.byNameCodec().sizeLimitedListOf(4)`, an item-id
/// string list in `back/left/right/front` order,
/// `DecoratedPotBlockEntity.java:56-64` / `PotDecorations.java:23-52`).
/// `Ok(None)` when the payload has no `sherds` list — `saveAdditional` skips
/// the field entirely for an undecorated pot.
pub(crate) fn decode_decorated_pot_sherds(
    raw_nbt: &[u8],
) -> Result<Option<DecoratedPotSherdsState>> {
    let Some(root) = decode_nbt_root(raw_nbt)? else {
        return Ok(None);
    };
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let Some(NbtValue::List(sherds)) = find_entry(&entries, "sherds") else {
        return Ok(None);
    };
    let item = |index: usize| -> Option<String> {
        match sherds.get(index) {
            Some(NbtValue::String(id)) if id != VANILLA_POT_EMPTY_SHERD_ITEM => Some(id.clone()),
            _ => None,
        }
    };
    Ok(Some(DecoratedPotSherdsState {
        back: item(0),
        left: item(1),
        right: item(2),
        front: item(3),
    }))
}
