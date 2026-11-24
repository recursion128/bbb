//! Fresh Native item carrying its source entity name.

use crate::entity::Entity;
use crate::world::simulation_level::LevelCtx;
use bbb_server_types::item_stack::RuntimeItemStack;

pub(crate) fn finish_named_entity_drop(
    entity: &Entity,
    stack: RuntimeItemStack,
    level: &LevelCtx<'_, '_>,
) -> Option<RuntimeItemStack> {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID;
    let publication = level.item_stack_resolver();
    let fresh = RuntimeItemStack::from_summary(bbb_protocol::packets::ItemStackSummary {
        item_id: Some(stack.item_id()?),
        count: 1,
        component_patch: Default::default(),
    });
    match entity
        .loot_component(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID)
        .ok()?
    {
        Some(name) => fresh
            .with_component_nbt(
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                Some(&name),
                publication,
            )
            .ok(),
        None => Some(fresh),
    }
}

/// `VehicleEntity.destroy` (`VehicleEntity.java:70-75`) copies the vehicle's
/// custom name onto its dropped item.
pub(crate) fn name_vehicle_drop(
    vehicle: &Entity,
    stack: RuntimeItemStack,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> RuntimeItemStack {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID;
    match vehicle.custom_name() {
        Some(name) => stack
            .clone()
            .with_component_nbt(
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                Some(name.component()),
                resolver,
            )
            .unwrap_or(stack),
        None => stack,
    }
}
