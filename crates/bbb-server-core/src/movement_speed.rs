use bbb_protocol::block_speed_factors::vanilla_block_state_speed_factor;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_BUBBLE_COLUMN_ID, VANILLA_BLOCK_WATER_ID};
use bbb_protocol::content_registry::ContentRegistry;

pub fn block_speed_factor_with_registry(state: i32, registry: &ContentRegistry) -> Option<f32> {
    if let Some(factor) = vanilla_block_state_speed_factor(state) {
        return Some(factor);
    }
    registry.block_by_state_id(state).map(|_| 1.0)
}

// Entity.java:1021-1029: water/bubble and a non-default local factor skip the below read.
pub fn select_block_speed_factor(
    state: i32,
    here: f32,
    below: impl FnOnce() -> Option<f32>,
) -> Option<f32> {
    select_block_speed_factor_with_registry(state, here, ContentRegistry::vanilla(), below)
}

pub fn select_block_speed_factor_with_registry(
    state: i32,
    here: f32,
    registry: &ContentRegistry,
    below: impl FnOnce() -> Option<f32>,
) -> Option<f32> {
    let kind = match vanilla_block_state_block_type_id(state) {
        Some(kind) => Some(kind),
        None => {
            registry.block_by_state_id(state)?;
            None
        }
    };
    if kind == Some(VANILLA_BLOCK_WATER_ID)
        || kind == Some(VANILLA_BLOCK_BUBBLE_COLUMN_ID)
        || here != 1.0
    {
        Some(here)
    } else {
        below()
    }
}
