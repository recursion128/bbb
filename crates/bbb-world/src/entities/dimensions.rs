use serde::{Deserialize, Serialize};

use super::{
    VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
    VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
    VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
    VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
    VANILLA_ENTITY_TYPE_TNT_MINECART_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPickBoundsState {
    pub width: f32,
    pub height: f32,
    pub pick_radius: f32,
}

pub(crate) fn vanilla_pick_bounds_for_type(entity_type_id: i32) -> Option<EntityPickBoundsState> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
        | VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID
        | VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
        | VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID
        | VANILLA_ENTITY_TYPE_MINECART_ID
        | VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID
        | VANILLA_ENTITY_TYPE_TNT_MINECART_ID => Some(EntityPickBoundsState {
            width: 0.98,
            height: 0.7,
            pick_radius: 0.0,
        }),
        VANILLA_ENTITY_TYPE_FIREBALL_ID => Some(EntityPickBoundsState {
            width: 1.0,
            height: 1.0,
            pick_radius: 1.0,
        }),
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID | VANILLA_ENTITY_TYPE_WIND_CHARGE_ID => {
            Some(EntityPickBoundsState {
                width: 0.3125,
                height: 0.3125,
                pick_radius: 1.0,
            })
        }
        _ => None,
    }
}
