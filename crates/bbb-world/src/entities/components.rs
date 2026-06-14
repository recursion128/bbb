use std::collections::BTreeMap;

use bbb_protocol::packets::{
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate, MinecartStep as ProtocolMinecartStep,
};
use uuid::Uuid;

use super::status::{EntityDamageEventState, MobEffectState};
use super::{EntityState, EntityVec3};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityIdentity {
    pub(crate) id: i32,
    pub(crate) uuid: Uuid,
    pub(crate) entity_type_id: i32,
    pub(crate) data: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityTransform {
    pub(crate) position: EntityVec3,
    pub(crate) position_base: EntityVec3,
    pub(crate) delta_movement: EntityVec3,
    pub(crate) y_rot: f32,
    pub(crate) x_rot: f32,
    pub(crate) y_head_rot: f32,
    pub(crate) on_ground: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityMetadata {
    pub(crate) data_values: Vec<ProtocolEntityDataValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityEquipment {
    pub(crate) equipment: Vec<ProtocolEquipmentSlotUpdate>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityAttributes {
    pub(crate) attributes: Vec<ProtocolAttributeSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityTransientEvents {
    pub(crate) last_animation_action: Option<u8>,
    pub(crate) last_event_id: Option<i8>,
    pub(crate) last_hurt_yaw: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityMount {
    pub(crate) vehicle_id: Option<i32>,
    pub(crate) passengers: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityLeash {
    pub(crate) holder_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityMobEffects {
    pub(crate) effects: BTreeMap<i32, MobEffectState>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityDamage {
    pub(crate) last_damage: Option<EntityDamageEventState>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityMinecartLerp {
    pub(crate) steps: Vec<ProtocolMinecartStep>,
}

impl From<&EntityState> for EntityIdentity {
    fn from(state: &EntityState) -> Self {
        Self {
            id: state.id,
            uuid: state.uuid,
            entity_type_id: state.entity_type_id,
            data: state.data,
        }
    }
}

impl From<&EntityState> for EntityTransform {
    fn from(state: &EntityState) -> Self {
        Self {
            position: state.position,
            position_base: state.position_base,
            delta_movement: state.delta_movement,
            y_rot: state.y_rot,
            x_rot: state.x_rot,
            y_head_rot: state.y_head_rot,
            on_ground: state.on_ground,
        }
    }
}

impl From<&EntityState> for EntityMetadata {
    fn from(state: &EntityState) -> Self {
        Self {
            data_values: state.data_values.clone(),
        }
    }
}

impl From<&EntityState> for EntityEquipment {
    fn from(state: &EntityState) -> Self {
        Self {
            equipment: state.equipment.clone(),
        }
    }
}

impl From<&EntityState> for EntityAttributes {
    fn from(state: &EntityState) -> Self {
        Self {
            attributes: state.attributes.clone(),
        }
    }
}

impl From<&EntityState> for EntityTransientEvents {
    fn from(state: &EntityState) -> Self {
        Self {
            last_animation_action: state.last_animation_action,
            last_event_id: state.last_event_id,
            last_hurt_yaw: state.last_hurt_yaw,
        }
    }
}

impl From<&EntityState> for EntityMount {
    fn from(state: &EntityState) -> Self {
        Self {
            vehicle_id: state.vehicle_id,
            passengers: state.passengers.clone(),
        }
    }
}

impl From<&EntityState> for EntityLeash {
    fn from(state: &EntityState) -> Self {
        Self {
            holder_id: state.leash_holder_id,
        }
    }
}

impl From<&EntityState> for EntityMobEffects {
    fn from(state: &EntityState) -> Self {
        Self {
            effects: state.mob_effects.clone(),
        }
    }
}

impl From<&EntityState> for EntityDamage {
    fn from(state: &EntityState) -> Self {
        Self {
            last_damage: state.last_damage,
        }
    }
}

impl From<&EntityState> for EntityMinecartLerp {
    fn from(state: &EntityState) -> Self {
        Self {
            steps: state.minecart_lerp_steps.clone(),
        }
    }
}

impl EntityTransform {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.position = self.position;
        state.position_base = self.position_base;
        state.delta_movement = self.delta_movement;
        state.y_rot = self.y_rot;
        state.x_rot = self.x_rot;
        state.y_head_rot = self.y_head_rot;
        state.on_ground = self.on_ground;
    }
}

impl EntityMetadata {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.data_values = self.data_values;
    }
}

impl EntityEquipment {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.equipment = self.equipment;
    }
}

impl EntityAttributes {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.attributes = self.attributes;
    }
}

impl EntityTransientEvents {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.last_animation_action = self.last_animation_action;
        state.last_event_id = self.last_event_id;
        state.last_hurt_yaw = self.last_hurt_yaw;
    }
}

impl EntityMount {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.vehicle_id = self.vehicle_id;
        state.passengers = self.passengers;
    }
}

impl EntityLeash {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.leash_holder_id = self.holder_id;
    }
}

impl EntityMobEffects {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.mob_effects = self.effects;
    }
}

impl EntityDamage {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.last_damage = self.last_damage;
    }
}

impl EntityMinecartLerp {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.minecart_lerp_steps = self.steps;
    }
}
