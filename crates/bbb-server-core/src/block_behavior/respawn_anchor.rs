//! `RespawnAnchorBlock` (`RespawnAnchorBlock.java`).

use crate::fluid::with_property;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::item_types::VANILLA_ITEM_GLOWSTONE_ID;

use super::{block_type_of, int_property};

/// `RespawnAnchorBlock.MAX_CHARGES` (`RespawnAnchorBlock.java:48`).
pub const MAX_CHARGES: i32 = 4;

/// `RespawnAnchorBlock.CHARGE` (`RespawnAnchorBlock.java:49`).
const CHARGE: &str = "charges";

/// `RespawnAnchorBlock.useItemOn` is unique to that class.
#[must_use]
pub fn is_respawn_anchor(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_item_on_override)
        == Some(VanillaBlockUseItemOnOverride::RespawnAnchorBlock)
}

/// `state.getValue(CHARGE)` (`RespawnAnchorBlock.java:136`).
#[must_use]
pub fn charges(block_state_id: i32) -> Option<i32> {
    int_property(block_state_id, CHARGE)
}

/// `RespawnAnchorBlock.canBeCharged` (`RespawnAnchorBlock.java:136-138`).
#[must_use]
pub fn can_be_charged(block_state_id: i32) -> bool {
    charges(block_state_id).is_some_and(|charge| charge < MAX_CHARGES)
}

/// `RespawnAnchorBlock.isRespawnFuel` (`RespawnAnchorBlock.java:132-134`).
#[must_use]
pub fn is_respawn_fuel(item_id: Option<i32>) -> bool {
    item_id == Some(VANILLA_ITEM_GLOWSTONE_ID)
}

/// `state.setValue(CHARGE, charges)` (`RespawnAnchorBlock.java:182`).
#[must_use]
pub fn with_charges(block_state_id: i32, charges: i32) -> Option<i32> {
    with_property(block_state_id, CHARGE, &charges.to_string())
}
