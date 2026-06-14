use std::collections::BTreeMap;

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeSnapshot as ProtocolAttributeSnapshot,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate,
    ItemStackSummary as ProtocolItemStackSummary, MinecartStep as ProtocolMinecartStep,
    RemoveEntities as ProtocolRemoveEntities, TakeItemEntity as ProtocolTakeItemEntity,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

mod components;
mod metadata;
mod movement;
mod passengers;
pub(crate) mod state;
mod status;
mod store;
mod updates;

pub(crate) use components::{
    EntityAttributes, EntityEquipment, EntityIdentity, EntityMetadata, EntityTransform,
    EntityTransientEvents,
};
use movement::entity_vec3;
use status::{EntityDamageEventState, MobEffectState};
pub(crate) use store::EntityStore;

pub(crate) const VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID: i32 = 49;
pub(crate) const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
pub(crate) const VANILLA_ENTITY_TYPE_CHEST_MINECART_ID: i32 = 25;
pub(crate) const VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID: i32 = 29;
pub(crate) const VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID: i32 = 56;
pub(crate) const VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID: i32 = 65;
pub(crate) const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;
pub(crate) const VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID: i32 = 122;
pub(crate) const VANILLA_ENTITY_TYPE_TNT_MINECART_ID: i32 = 133;
pub(crate) const VANILLA_ITEM_ENTITY_STACK_DATA_ID: u8 = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub data: i32,
    pub position: EntityVec3,
    pub position_base: EntityVec3,
    pub delta_movement: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub on_ground: Option<bool>,
    pub data_values: Vec<ProtocolEntityDataValue>,
    pub equipment: Vec<ProtocolEquipmentSlotUpdate>,
    pub attributes: Vec<ProtocolAttributeSnapshot>,
    pub vehicle_id: Option<i32>,
    pub passengers: Vec<i32>,
    pub leash_holder_id: Option<i32>,
    pub last_animation_action: Option<u8>,
    pub last_event_id: Option<i8>,
    pub last_hurt_yaw: Option<f32>,
    #[serde(default)]
    pub mob_effects: BTreeMap<i32, MobEffectState>,
    #[serde(default)]
    pub last_damage: Option<EntityDamageEventState>,
    #[serde(default)]
    pub minecart_lerp_steps: Vec<ProtocolMinecartStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleMoveReport {
    pub vehicle_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub snapped: bool,
}

impl WorldStore {
    pub fn apply_add_entity(&mut self, packet: ProtocolAddEntity) {
        self.counters.entities_received += 1;
        let entity = EntityState {
            id: packet.id,
            uuid: packet.uuid,
            entity_type_id: packet.entity_type_id,
            data: packet.data,
            position: entity_vec3(packet.position),
            position_base: entity_vec3(packet.position),
            delta_movement: entity_vec3(packet.delta_movement),
            y_rot: packet.y_rot,
            x_rot: packet.x_rot,
            y_head_rot: packet.y_head_rot,
            on_ground: None,
            data_values: Vec::new(),
            equipment: Vec::new(),
            attributes: Vec::new(),
            vehicle_id: None,
            passengers: Vec::new(),
            leash_holder_id: None,
            last_animation_action: None,
            last_event_id: None,
            last_hurt_yaw: None,
            mob_effects: BTreeMap::new(),
            last_damage: None,
            minecart_lerp_steps: Vec::new(),
        };

        self.entities.insert_or_replace(entity);
        self.update_entity_count();
        self.update_active_mob_effect_count();
    }

    pub fn apply_take_item_entity(&mut self, packet: ProtocolTakeItemEntity) -> bool {
        self.counters.take_item_entities_received += 1;
        let Some(entity_type_id) = self.entities.entity_type_id(packet.item_id) else {
            return false;
        };

        self.counters.take_item_entities_applied += 1;
        if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            return true;
        }

        if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID {
            let mut stack_shrank = false;
            let keep_entity = self
                .entities
                .with_mut(packet.item_id, |entity| {
                    if let Some(stack) = item_entity_stack_mut(entity) {
                        if stack.count > 0 && packet.amount > 0 {
                            stack.count = stack.count.saturating_sub(packet.amount).max(0);
                            stack_shrank = true;
                        }
                        return stack.count > 0;
                    }
                    false
                })
                .unwrap_or(false);
            if stack_shrank {
                self.counters.item_entity_stack_shrinks += 1;
            }
            if keep_entity {
                return true;
            }
        }

        let removed = self.remove_entities_by_ids(&[packet.item_id]);
        self.counters.take_item_entities_removed += removed;
        true
    }

    pub fn apply_remove_entities(&mut self, packet: ProtocolRemoveEntities) -> usize {
        self.counters.entity_removes_received += packet.entity_ids.len();
        self.remove_entities_by_ids(&packet.entity_ids)
    }

    fn remove_entities_by_ids(&mut self, removed_ids: &[i32]) -> usize {
        let removed = self.entities.remove_ids(removed_ids);
        if self
            .local_player_vehicle_id
            .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
        {
            self.local_player_vehicle_id = None;
        }
        self.entities.for_each_mut(|entity| {
            if entity
                .vehicle_id
                .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
            {
                entity.vehicle_id = None;
            }
            if entity
                .leash_holder_id
                .is_some_and(|holder_id| removed_ids.contains(&holder_id))
            {
                entity.leash_holder_id = None;
            }
            entity
                .passengers
                .retain(|passenger_id| !removed_ids.contains(passenger_id));
        });
        self.counters.entities_removed += removed;
        self.update_entity_count();
        self.update_active_mob_effect_count();
        removed
    }

    pub fn probe_entity(&self, id: i32) -> Option<&EntityState> {
        self.entities.get(id)
    }

    pub fn local_player_id(&self) -> Option<i32> {
        self.local_player_id
    }

    pub fn local_player_vehicle_id(&self) -> Option<i32> {
        self.local_player_vehicle_id
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub(crate) fn update_entity_count(&mut self) {
        self.counters.entities_tracked = self.entities.len();
    }
}

fn item_entity_stack_mut(entity: &mut EntityState) -> Option<&mut ProtocolItemStackSummary> {
    entity.data_values.iter_mut().find_map(|value| {
        if value.data_id == VANILLA_ITEM_ENTITY_STACK_DATA_ID {
            if let EntityDataValueKind::ItemStack(stack) = &mut value.value {
                return Some(stack);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests;
