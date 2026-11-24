use crate::entity::{chest_boat, minecart_container, Entity, EntityData};
use bbb_server_types::item_stack::RuntimeItemStack;
use std::cell::{Ref, RefCell};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::entity) struct VehicleCargo {
    pub(in crate::entity) items: Vec<Option<RuntimeItemStack>>,
    pub(in crate::entity) loot: crate::block_entity::ContainerLootTable,
}

impl VehicleCargo {
    pub(in crate::entity) fn new(items: Vec<Option<RuntimeItemStack>>) -> Self {
        Self {
            items,
            loot: crate::block_entity::ContainerLootTable::default(),
        }
    }

    pub(in crate::entity) fn item(
        cargo: &RefCell<Self>,
        slot: usize,
    ) -> Option<Ref<'_, RuntimeItemStack>> {
        Ref::filter_map(cargo.borrow(), |cargo| {
            cargo
                .items
                .get(slot)?
                .as_ref()
                .filter(|stack| !stack.is_empty())
        })
        .ok()
    }

    pub(in crate::entity) fn loot_table_id(cargo: &RefCell<Self>) -> Option<Ref<'_, str>> {
        Ref::filter_map(cargo.borrow(), |cargo| cargo.loot.table_id()).ok()
    }
}

fn cargo(entity: &Entity) -> Option<&RefCell<VehicleCargo>> {
    if let Some(data) = chest_boat::chest_boat(entity.data()) {
        Some(&data.cargo)
    } else {
        Some(&minecart_container::minecart_container(entity.data())?.cargo)
    }
}

fn cargo_mut(entity: &mut Entity) -> Option<&mut VehicleCargo> {
    let cargo = match &mut entity.data {
        EntityData::ChestBoat(data) => &mut data.cargo,
        EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => &mut data.cargo,
        _ => return None,
    };
    Some(cargo.get_mut())
}

pub(super) fn unpack_vehicle_cargo(
    entity: &Entity,
    fill: impl FnOnce(
        bbb_protocol::packets::Vec3d,
        &crate::block_entity::ContainerLoot,
        Vec<usize>,
    ) -> Result<Vec<(usize, RuntimeItemStack)>, String>,
) -> Result<Option<String>, String> {
    let Some(cargo) = cargo(entity) else {
        return Ok(None);
    };
    let (request, available) = {
        let mut cargo = cargo.borrow_mut();
        let Some(request) = cargo.loot.take() else {
            return Ok(None);
        };
        let available = cargo
            .items
            .iter()
            .enumerate()
            .filter(|(_, stack)| stack.as_ref().is_none_or(RuntimeItemStack::is_empty))
            .map(|(slot, _)| slot)
            .collect();
        (request, available)
    };
    let written = fill(entity.position(), &request, available).map_err(|error| {
        format!(
            "{} could not unpack into the vehicle: {error}",
            request.table_id
        )
    })?;
    let mut cargo = cargo.borrow_mut();
    for (slot, stack) in written {
        cargo.items[slot] = limited_stack(stack)?;
    }
    Ok(Some(request.table_id))
}

pub(super) fn size(entity: &Entity) -> Option<usize> {
    if chest_boat::chest_boat(entity.data()).is_some() {
        Some(chest_boat::CONTAINER_SIZE)
    } else {
        minecart_container::minecart_container(entity.data()).map(|data| data.container_size())
    }
}

pub(super) fn read(entity: &Entity, slot: usize) -> Option<RuntimeItemStack> {
    let cargo = cargo(entity)?.borrow();
    let stack = cargo
        .items
        .get(slot)?
        .as_ref()
        .filter(|stack| !stack.is_empty());
    Some(stack.cloned().unwrap_or_else(super::empty))
}

pub(super) fn write(
    entity: &mut Entity,
    slot: usize,
    stack: RuntimeItemStack,
) -> Result<bool, String> {
    let Some(target) = cargo_mut(entity).and_then(|cargo| cargo.items.get_mut(slot)) else {
        return Ok(false);
    };
    // ContainerEntity.java:147-151; Container.java:34-39 limits the Native count.
    *target = limited_stack(stack)?;
    Ok(true)
}

fn limited_stack(mut stack: RuntimeItemStack) -> Result<Option<RuntimeItemStack>, String> {
    Ok(if stack.is_empty() {
        None
    } else {
        let limit = stack
            .max_stack_size()
            .map_err(|error| error.to_string())?
            .min(99);
        stack.set_count(stack.count().min(limit));
        Some(stack)
    })
}

pub(super) fn set_count(entity: &mut Entity, slot: usize, count: i32) -> Result<bool, String> {
    let Some(target) = cargo_mut(entity).and_then(|cargo| cargo.items.get_mut(slot)) else {
        return Ok(false);
    };
    // HopperBlockEntity.java:248-264: restore the same mutable object without setItem's limit.
    if let Some(original) = target {
        *original = original.with_count(count);
    }
    Ok(true)
}

pub(super) fn has_loot(entity: &Entity) -> bool {
    cargo(entity).is_some_and(|cargo| cargo.borrow().loot.is_set())
}
