//! Infested blocks and the host states they preserve.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::*;

use super::weathering::with_properties_of;

fn block_pair(block_type: i32) -> Option<(i32, i32)> {
    Some(match block_type {
        VANILLA_BLOCK_STONE_ID | VANILLA_BLOCK_INFESTED_STONE_ID => {
            (VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_INFESTED_STONE_ID)
        }
        VANILLA_BLOCK_COBBLESTONE_ID | VANILLA_BLOCK_INFESTED_COBBLESTONE_ID => (
            VANILLA_BLOCK_COBBLESTONE_ID,
            VANILLA_BLOCK_INFESTED_COBBLESTONE_ID,
        ),
        VANILLA_BLOCK_STONE_BRICKS_ID | VANILLA_BLOCK_INFESTED_STONE_BRICKS_ID => (
            VANILLA_BLOCK_STONE_BRICKS_ID,
            VANILLA_BLOCK_INFESTED_STONE_BRICKS_ID,
        ),
        VANILLA_BLOCK_MOSSY_STONE_BRICKS_ID | VANILLA_BLOCK_INFESTED_MOSSY_STONE_BRICKS_ID => (
            VANILLA_BLOCK_MOSSY_STONE_BRICKS_ID,
            VANILLA_BLOCK_INFESTED_MOSSY_STONE_BRICKS_ID,
        ),
        VANILLA_BLOCK_CRACKED_STONE_BRICKS_ID | VANILLA_BLOCK_INFESTED_CRACKED_STONE_BRICKS_ID => (
            VANILLA_BLOCK_CRACKED_STONE_BRICKS_ID,
            VANILLA_BLOCK_INFESTED_CRACKED_STONE_BRICKS_ID,
        ),
        VANILLA_BLOCK_CHISELED_STONE_BRICKS_ID
        | VANILLA_BLOCK_INFESTED_CHISELED_STONE_BRICKS_ID => (
            VANILLA_BLOCK_CHISELED_STONE_BRICKS_ID,
            VANILLA_BLOCK_INFESTED_CHISELED_STONE_BRICKS_ID,
        ),
        VANILLA_BLOCK_DEEPSLATE_ID | VANILLA_BLOCK_INFESTED_DEEPSLATE_ID => (
            VANILLA_BLOCK_DEEPSLATE_ID,
            VANILLA_BLOCK_INFESTED_DEEPSLATE_ID,
        ),
        _ => return None,
    })
}

pub(crate) fn is_compatible_host(state: i32) -> bool {
    vanilla_block_state_block_type_id(state)
        .is_some_and(|block| block_pair(block).is_some_and(|(host, _)| block == host))
}

pub(crate) fn infested_state_by_host(state: i32) -> Option<i32> {
    let block = vanilla_block_state_block_type_id(state)?;
    let (host, infested) = block_pair(block)?;
    (block == host).then(|| with_properties_of(infested, state))?
}

pub(crate) fn host_state_by_infested(state: i32) -> Option<i32> {
    let block = vanilla_block_state_block_type_id(state)?;
    let (host, infested) = block_pair(block)?;
    (block == infested).then(|| with_properties_of(host, state))?
}
