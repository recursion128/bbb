//! Passengerless chest and hopper minecart container state.

use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_HOPPER_ID};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;
use std::cell::{Ref, RefCell};

use super::item_slots::container::VehicleCargo;
use crate::entity::{Entity, EntityData, EntityStore, MinecartData};

pub const CHEST_CONTAINER_SIZE: usize = 27;
pub const HOPPER_CONTAINER_SIZE: usize = 5;
pub const CHEST_DISPLAY_OFFSET: i32 = 8;
pub const HOPPER_DISPLAY_OFFSET: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinecartContainerKind {
    Chest,
    Hopper,
}

impl MinecartContainerKind {
    pub const fn container_size(self) -> usize {
        match self {
            Self::Chest => CHEST_CONTAINER_SIZE,
            Self::Hopper => HOPPER_CONTAINER_SIZE,
        }
    }

    pub const fn default_display_offset(self) -> i32 {
        match self {
            Self::Chest => CHEST_DISPLAY_OFFSET,
            Self::Hopper => HOPPER_DISPLAY_OFFSET,
        }
    }

    pub(crate) const fn vehicle_item_resource_id(self) -> &'static str {
        match self {
            Self::Chest => "minecraft:chest_minecart",
            Self::Hopper => "minecraft:hopper_minecart",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("minecart container slot {slot} is outside 0..{size}")]
pub struct MinecartContainerSlotError {
    slot: usize,
    size: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinecartContainerData {
    kind: MinecartContainerKind,
    minecart: MinecartData,
    pub(super) cargo: RefCell<VehicleCargo>,
    enabled: bool,
}

impl MinecartContainerData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.minecart.restore_command_transients(&previous.minecart);
    }

    pub fn new_chest() -> Self {
        Self::new(MinecartContainerKind::Chest)
    }

    pub fn new_hopper() -> Self {
        Self::new(MinecartContainerKind::Hopper)
    }

    fn new(kind: MinecartContainerKind) -> Self {
        Self {
            kind,
            minecart: MinecartData::with_default_display_offset(kind.default_display_offset()),
            cargo: RefCell::new(VehicleCargo::new(vec![None; kind.container_size()])),
            enabled: true,
        }
    }

    pub const fn kind(&self) -> MinecartContainerKind {
        self.kind
    }

    pub const fn minecart(&self) -> &MinecartData {
        &self.minecart
    }

    pub(crate) const fn minecart_mut(&mut self) -> &mut MinecartData {
        &mut self.minecart
    }

    pub const fn container_size(&self) -> usize {
        self.kind.container_size()
    }

    pub fn item(&self, slot: usize) -> Option<Ref<'_, RuntimeItemStack>> {
        VehicleCargo::item(&self.cargo, slot)
    }

    pub fn set_item(
        &mut self,
        slot: usize,
        stack: Option<RuntimeItemStack>,
    ) -> Result<(), MinecartContainerSlotError> {
        let size = self.container_size();
        let target = self
            .cargo
            .get_mut()
            .items
            .get_mut(slot)
            .ok_or(MinecartContainerSlotError { slot, size })?;
        *target = stack.filter(|stack| !stack.is_empty());
        Ok(())
    }

    pub fn remove_item(
        &mut self,
        slot: usize,
    ) -> Result<Option<RuntimeItemStack>, MinecartContainerSlotError> {
        let size = self.container_size();
        let target = self
            .cargo
            .get_mut()
            .items
            .get_mut(slot)
            .ok_or(MinecartContainerSlotError { slot, size })?;
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

    pub(crate) fn set_loot_table_seed(&mut self, seed: i64) {
        self.cargo.get_mut().loot.set_seed(seed);
    }

    pub fn set_loot_table(&mut self, id: String, seed: i64) -> Result<(), String> {
        self.cargo.get_mut().loot.set_id(id, seed)
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.kind == MinecartContainerKind::Hopper {
            self.enabled = enabled;
        }
    }

    /// `MinecartChest.java:42-52` and `MinecartHopper.java:35-45`.
    pub fn default_display_block_state_id(&self) -> i32 {
        let block_type_id = match self.kind {
            MinecartContainerKind::Chest => VANILLA_BLOCK_CHEST_ID,
            MinecartContainerKind::Hopper => VANILLA_BLOCK_HOPPER_ID,
        };
        vanilla_block_default_state_id(block_type_id).expect("built-in display block")
    }

    pub(crate) fn items(&self) -> Ref<'_, [Option<RuntimeItemStack>]> {
        Ref::map(self.cargo.borrow(), |cargo| cargo.items.as_slice())
    }

    pub(crate) fn from_parts(
        kind: MinecartContainerKind,
        minecart: MinecartData,
        items: Vec<Option<RuntimeItemStack>>,
        enabled: bool,
    ) -> Result<Self, MinecartContainerSlotError> {
        let size = kind.container_size();
        if items.len() != size {
            return Err(MinecartContainerSlotError {
                slot: items.len(),
                size,
            });
        }
        Ok(Self {
            kind,
            minecart,
            cargo: RefCell::new(VehicleCargo::new(items)),
            enabled,
        })
    }

    /// `AbstractMinecartContainer.applyNaturalSlowdown`
    /// (`AbstractMinecartContainer.java:101-113`).
    pub(crate) fn apply_natural_slowdown(&self, movement: Vec3d, in_water: bool) -> Vec3d {
        let mut total_percent = 0.0_f32;
        let cargo = self.cargo.borrow();
        for original in cargo.items.iter().flatten().filter(|item| !item.is_empty()) {
            let stack = original;
            let count = crate::entity::item::stack_count(stack) as f32;
            let max = crate::entity::item::stack_max_size(stack).unwrap_or(1) as f32;
            total_percent += count / max;
        }
        total_percent /= self.container_size() as f32;
        let signal = if total_percent > 0.0 {
            (total_percent * 14.0).floor() as i32 + 1
        } else {
            0
        }
        .clamp(0, 15);
        let mut keep = 0.98_f32;
        if !cargo.loot.is_set() {
            keep += (15 - signal) as f32 * 0.001_f32;
        }
        if in_water {
            keep *= 0.95_f32;
        }
        movement.multiply(f64::from(keep), 0.0, f64::from(keep))
    }
}

pub fn minecart_container(data: &EntityData) -> Option<&MinecartContainerData> {
    match data {
        EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => Some(data),
        _ => None,
    }
}

pub(crate) fn minecart_container_mut(data: &mut EntityData) -> Option<&mut MinecartContainerData> {
    match data {
        EntityData::ChestMinecart(data) | EntityData::HopperMinecart(data) => Some(data),
        _ => None,
    }
}

pub fn spawn_minecart_container(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: Vec3d,
    y_rot: f32,
) -> Option<i32> {
    let data = match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => {
            EntityData::ChestMinecart(MinecartContainerData::new_chest())
        }
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => {
            EntityData::HopperMinecart(MinecartContainerData::new_hopper())
        }
        _ => return None,
    };
    let uuid = store.next_uuid();
    store.restore_initialized(uuid, entity_type_id, position, data, |entity| {
        entity.set_rot(y_rot, 0.0);
    })
}

pub fn spawn_structure_chest_minecart(
    store: &mut EntityStore,
    position: Vec3d,
    loot_table: &str,
    seed: i64,
) -> Option<i32> {
    let mut data = MinecartContainerData::new_chest();
    data.set_loot_table(loot_table.to_owned(), seed).ok()?;
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
        position,
        EntityData::ChestMinecart(data),
        |_| {},
    )
}

/// The cargo a destroying removal drops; `Containers.dropContents` leaves the slots empty.
pub(crate) fn take_destroyed_cargo(entity: &mut Entity) -> Option<Vec<Option<RuntimeItemStack>>> {
    entity
        .removal_reason()
        .filter(|reason| reason.should_destroy())?;
    let cargo = minecart_container_mut(&mut entity.data)?;
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
