use super::*;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_name, vanilla_block_state_with_property,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_COMMAND_BLOCK_ID, VANILLA_BLOCK_FURNACE_ID,
    VANILLA_BLOCK_HOPPER_ID, VANILLA_BLOCK_SPAWNER_ID, VANILLA_BLOCK_TNT_ID,
};

pub(super) fn minecart_display_offset_for_components(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
    _registries: &RegistrySet,
) -> Option<i32> {
    if !is_vanilla_minecart_type(entity_type_id) {
        return None;
    }
    let accessors =
        vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(entity_type_id)?;
    let custom_state_id = data_values
        .iter()
        .find(|value| value.data_id == accessors.custom_display_block_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalBlockState(state) => *state,
            _ => None,
        });
    let has_display = if let Some(state_id) = custom_state_id {
        vanilla_block_state_name(state_id).is_some_and(|name| name != "minecraft:air")
    } else {
        default_minecart_display_block_type_id(entity_type_id).is_some()
    };
    has_display.then(|| {
        data_values
            .iter()
            .find(|value| value.data_id == accessors.display_offset_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Int(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(accessors.display_offset_default)
    })
}

pub(super) fn default_minecart_display_block_state(
    entity_type_id: i32,
    furnace_has_fuel: bool,
    registries: &RegistrySet,
) -> Option<&'static BlockStateInfo> {
    let block_type_id = default_minecart_display_block_type_id(entity_type_id)?;
    let mut state_id = vanilla_block_default_state_id(block_type_id)?;
    if block_type_id == VANILLA_BLOCK_FURNACE_ID && furnace_has_fuel {
        state_id = vanilla_block_state_with_property(state_id, "lit", "true")?;
    }
    registries.block_state(state_id)
}

fn default_minecart_display_block_type_id(entity_type_id: i32) -> Option<i32> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => Some(VANILLA_BLOCK_CHEST_ID),
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID => Some(VANILLA_BLOCK_COMMAND_BLOCK_ID),
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID => Some(VANILLA_BLOCK_FURNACE_ID),
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => Some(VANILLA_BLOCK_HOPPER_ID),
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID => Some(VANILLA_BLOCK_SPAWNER_ID),
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID => Some(VANILLA_BLOCK_TNT_ID),
        _ => None,
    }
}

pub(super) fn default_primed_tnt_block_state(
    registries: &RegistrySet,
    default: crate::entities::schema::VanillaEntityMetadataDefault,
) -> Option<&'static BlockStateInfo> {
    let crate::entities::schema::VanillaEntityMetadataDefault::BlockStateDefault {
        resource_id,
        properties,
    } = default
    else {
        unreachable!("generated PrimedTnt block-state default has the wrong kind")
    };
    let properties = properties
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    let state_id = registries.block_state_id_by_name_and_properties(resource_id, &properties)?;
    registries.block_state(state_id)
}

pub(super) fn entity_block_pos(position: super::super::EntityVec3) -> crate::BlockPos {
    crate::BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}
