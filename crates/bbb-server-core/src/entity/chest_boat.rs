//! Passengerless `AbstractChestBoat` state shared by the ten chest boat and raft types.

use bbb_protocol::entity_types::{
    vanilla_entity_type, VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID, VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;
use std::cell::{Ref, RefCell};

use super::item_slots::container::VehicleCargo;
use crate::entity::{BoatData, Entity, EntityData, EntityStore};

/// `AbstractChestBoat.CONTAINER_SIZE` (`AbstractChestBoat.java:33`).
pub const CONTAINER_SIZE: usize = 27;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("chest boat slot {slot} is outside 0..{CONTAINER_SIZE}")]
pub struct ChestBoatSlotError {
    slot: usize,
}

/// `AbstractChestBoat`'s boat fields and fixed three-row inventory.
#[derive(Debug, Clone, PartialEq)]
pub struct ChestBoatData {
    boat: BoatData,
    pub(super) cargo: RefCell<VehicleCargo>,
}

impl ChestBoatData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.boat.restore_command_transients(&previous.boat);
    }

    pub fn new() -> Self {
        Self {
            boat: BoatData::with_catalog(
                &VANILLA_ENTITY_TYPES[VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID as usize],
            ),
            cargo: RefCell::new(VehicleCargo::new(vec![None; CONTAINER_SIZE])),
        }
    }

    pub(crate) fn for_entity_type(entity_type_id: i32) -> Option<Self> {
        if !is_chest_boat_type(entity_type_id) {
            return None;
        }
        Some(Self {
            boat: BoatData::with_catalog(vanilla_entity_type(entity_type_id)?),
            cargo: RefCell::new(VehicleCargo::new(vec![None; CONTAINER_SIZE])),
        })
    }

    pub const fn boat(&self) -> &BoatData {
        &self.boat
    }

    pub(crate) const fn boat_mut(&mut self) -> &mut BoatData {
        &mut self.boat
    }

    pub fn item(&self, slot: usize) -> Option<Ref<'_, RuntimeItemStack>> {
        VehicleCargo::item(&self.cargo, slot)
    }

    pub fn set_item(
        &mut self,
        slot: usize,
        stack: Option<RuntimeItemStack>,
    ) -> Result<(), ChestBoatSlotError> {
        let target = self
            .cargo
            .get_mut()
            .items
            .get_mut(slot)
            .ok_or(ChestBoatSlotError { slot })?;
        *target = stack.filter(|stack| !stack.is_empty());
        Ok(())
    }

    pub fn remove_item(
        &mut self,
        slot: usize,
    ) -> Result<Option<RuntimeItemStack>, ChestBoatSlotError> {
        let target = self
            .cargo
            .get_mut()
            .items
            .get_mut(slot)
            .ok_or(ChestBoatSlotError { slot })?;
        if target.as_ref().is_none_or(RuntimeItemStack::is_empty) {
            return Ok(None);
        }
        Ok(target.take())
    }

    pub fn loot_table_id(&self) -> Option<Ref<'_, str>> {
        VehicleCargo::loot_table_id(&self.cargo)
    }

    pub fn loot_table_seed(&self) -> i64 {
        self.cargo.borrow().loot.seed()
    }

    pub fn set_loot_table(&mut self, id: String, seed: i64) -> Result<(), String> {
        self.cargo.get_mut().loot.set_id(id, seed)
    }

    pub(crate) fn set_loot_table_seed(&mut self, seed: i64) {
        self.cargo.get_mut().loot.set_seed(seed);
    }

    pub(crate) fn from_parts(
        boat: BoatData,
        items: Vec<Option<RuntimeItemStack>>,
    ) -> Result<Self, ChestBoatSlotError> {
        if items.len() != CONTAINER_SIZE {
            return Err(ChestBoatSlotError { slot: items.len() });
        }
        Ok(Self {
            boat,
            cargo: RefCell::new(VehicleCargo::new(items)),
        })
    }
}

impl Default for ChestBoatData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn chest_boat(data: &EntityData) -> Option<&ChestBoatData> {
    match data {
        EntityData::ChestBoat(data) => Some(data),
        _ => None,
    }
}

pub(crate) fn chest_boat_mut(data: &mut EntityData) -> Option<&mut ChestBoatData> {
    match data {
        EntityData::ChestBoat(data) => Some(data),
        _ => None,
    }
}

pub(super) const fn is_chest_boat_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID
    )
}

pub fn spawn_chest_boat(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: Vec3d,
    y_rot: f32,
) -> Option<i32> {
    let data = EntityData::ChestBoat(ChestBoatData::for_entity_type(entity_type_id)?);
    let uuid = store.next_uuid();
    store.restore_initialized(uuid, entity_type_id, position, data, |entity| {
        entity.set_rot(y_rot, 0.0);
    })
}

/// The cargo a destroying removal drops; `Containers.dropContents` leaves the slots empty.
pub(crate) fn take_destroyed_cargo(entity: &mut Entity) -> Option<Vec<Option<RuntimeItemStack>>> {
    entity
        .removal_reason()
        .filter(|reason| reason.should_destroy())?;
    let cargo = chest_boat_mut(&mut entity.data)?;
    Some(
        cargo
            .cargo
            .get_mut()
            .items
            .iter_mut()
            .map(Option::take)
            .collect(),
    )
}
