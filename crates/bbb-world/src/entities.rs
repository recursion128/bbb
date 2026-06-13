use std::collections::BTreeMap;

use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AttributeSnapshot as ProtocolAttributeSnapshot,
    EntityAnimation as ProtocolEntityAnimation, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind, EntityEvent as ProtocolEntityEvent, EntityMove as ProtocolEntityMove,
    EntityPositionSync as ProtocolEntityPositionSync,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate, HurtAnimation as ProtocolHurtAnimation,
    ItemStackSummary as ProtocolItemStackSummary, MoveVehicle as ProtocolMoveVehicle,
    RemoveEntities as ProtocolRemoveEntities, RotateHead as ProtocolRotateHead,
    SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
    SetEntityMotion as ProtocolSetEntityMotion, SetEquipment as ProtocolSetEquipment,
    SetPassengers as ProtocolSetPassengers, TakeItemEntity as ProtocolTakeItemEntity,
    TeleportEntity as ProtocolTeleportEntity, UpdateAttributes as ProtocolUpdateAttributes,
    Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y,
    PLAYER_RELATIVE_DELTA_Z, PLAYER_RELATIVE_ROTATE_DELTA, PLAYER_RELATIVE_X,
    PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y, PLAYER_RELATIVE_Y_ROT, PLAYER_RELATIVE_Z,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{EntityDamageEventState, MobEffectState, WorldStore};

pub(crate) const VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID: i32 = 49;
pub(crate) const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
pub(crate) const VANILLA_ITEM_ENTITY_STACK_DATA_ID: u8 = 8;
const MOVE_VEHICLE_SNAP_EPSILON_SQUARED: f64 = 1e-10;

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
        };

        if let Some(existing) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        {
            *existing = entity;
        } else {
            self.entities.push(entity);
        }
        self.update_entity_count();
        self.update_active_mob_effect_count();
    }

    pub fn apply_entity_animation(&mut self, packet: ProtocolEntityAnimation) -> bool {
        self.counters.entity_animation_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.last_animation_action = Some(packet.action);
        self.counters.entity_animation_updates_applied += 1;
        true
    }

    pub fn apply_entity_event(&mut self, packet: ProtocolEntityEvent) -> bool {
        self.counters.entity_events_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        entity.last_event_id = Some(packet.event_id);
        self.counters.entity_events_applied += 1;
        true
    }

    pub fn apply_hurt_animation(&mut self, packet: ProtocolHurtAnimation) -> bool {
        self.counters.entity_hurt_animations_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.last_hurt_yaw = Some(packet.yaw);
        self.counters.entity_hurt_animations_applied += 1;
        true
    }

    pub fn apply_entity_position_sync(&mut self, packet: ProtocolEntityPositionSync) -> bool {
        self.counters.entity_position_syncs_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.position = entity_vec3(packet.position);
        entity.position_base = entity_vec3(packet.position);
        entity.delta_movement = entity_vec3(packet.delta_movement);
        entity.y_rot = packet.y_rot;
        entity.x_rot = packet.x_rot;
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_position_syncs_applied += 1;
        true
    }

    pub fn apply_entity_move(&mut self, packet: ProtocolEntityMove) -> bool {
        self.counters.entity_moves_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        if packet.delta_x != 0 || packet.delta_y != 0 || packet.delta_z != 0 {
            let position = decode_entity_delta_position(
                entity.position_base,
                packet.delta_x,
                packet.delta_y,
                packet.delta_z,
            );
            entity.position = position;
            entity.position_base = position;
        }
        if let Some(y_rot) = packet.y_rot {
            entity.y_rot = y_rot;
        }
        if let Some(x_rot) = packet.x_rot {
            entity.x_rot = x_rot;
        }
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_moves_applied += 1;
        true
    }

    pub fn apply_teleport_entity(&mut self, packet: ProtocolTeleportEntity) -> bool {
        self.counters.entity_teleports_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        let absolute = entity_absolute_move_rotation(
            entity.position,
            entity.delta_movement,
            entity.y_rot,
            entity.x_rot,
            packet.position,
            packet.delta_movement,
            packet.y_rot,
            packet.x_rot,
            packet.relatives_mask,
        );
        entity.position = absolute.position;
        entity.delta_movement = absolute.delta_movement;
        entity.y_rot = absolute.y_rot;
        entity.x_rot = absolute.x_rot;
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_teleports_applied += 1;
        true
    }

    pub fn apply_set_entity_data(&mut self, packet: ProtocolSetEntityData) -> bool {
        self.counters.entity_data_updates_received += 1;
        self.counters.entity_data_values_received += packet.values.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        for value in packet.values {
            if let Some(existing) = entity
                .data_values
                .iter_mut()
                .find(|existing| existing.data_id == value.data_id)
            {
                *existing = value;
            } else {
                entity.data_values.push(value);
            }
        }
        entity.data_values.sort_by_key(|value| value.data_id);
        self.counters.entity_data_updates_applied += 1;
        true
    }

    pub fn apply_set_equipment(&mut self, packet: ProtocolSetEquipment) -> bool {
        self.counters.entity_equipment_updates_received += 1;
        self.counters.entity_equipment_slots_received += packet.slots.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        for update in packet.slots {
            if let Some(existing) = entity
                .equipment
                .iter_mut()
                .find(|existing| existing.slot == update.slot)
            {
                *existing = update;
            } else {
                entity.equipment.push(update);
            }
        }
        entity.equipment.sort_by_key(|update| update.slot.ordinal());
        self.counters.entity_equipment_updates_applied += 1;
        true
    }

    pub fn apply_update_attributes(&mut self, packet: ProtocolUpdateAttributes) -> bool {
        self.counters.entity_attribute_updates_received += 1;
        self.counters.entity_attributes_received += packet.attributes.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        for attribute in packet.attributes {
            if let Some(existing) = entity
                .attributes
                .iter_mut()
                .find(|existing| existing.attribute_id == attribute.attribute_id)
            {
                *existing = attribute;
            } else {
                entity.attributes.push(attribute);
            }
        }
        entity
            .attributes
            .sort_by_key(|attribute| attribute.attribute_id);
        self.counters.entity_attribute_updates_applied += 1;
        true
    }

    pub fn apply_set_passengers(&mut self, packet: ProtocolSetPassengers) -> bool {
        self.counters.entity_passenger_updates_received += 1;
        self.counters.entity_passenger_ids_received += packet.passenger_ids.len();
        let local_player_id = self.local_player_id;
        let local_player_was_on_packet_vehicle =
            self.local_player_vehicle_id == Some(packet.vehicle_id);
        let Some(vehicle_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == packet.vehicle_id)
        else {
            return false;
        };

        for entity in &mut self.entities {
            if entity.vehicle_id == Some(packet.vehicle_id) {
                entity.vehicle_id = None;
            }
        }
        self.entities[vehicle_index].passengers.clear();

        let mut mounted = Vec::new();
        let mut local_player_mounted_here = false;
        for passenger_id in packet.passenger_ids {
            if passenger_id == packet.vehicle_id || mounted.contains(&passenger_id) {
                continue;
            }
            let is_local_player = local_player_id == Some(passenger_id);
            if is_local_player {
                if let Some(old_vehicle_id) = self.local_player_vehicle_id {
                    if old_vehicle_id != packet.vehicle_id {
                        self.remove_passenger_from_vehicle(old_vehicle_id, passenger_id);
                    }
                }
                self.local_player_vehicle_id = Some(packet.vehicle_id);
                local_player_mounted_here = true;
            }
            let passenger_index = self
                .entities
                .iter()
                .position(|entity| entity.id == passenger_id);
            let Some(passenger_index) = passenger_index else {
                if is_local_player {
                    mounted.push(passenger_id);
                }
                continue;
            };
            if let Some(old_vehicle_id) = self.entities[passenger_index].vehicle_id {
                if let Some(old_vehicle) = self
                    .entities
                    .iter_mut()
                    .find(|entity| entity.id == old_vehicle_id)
                {
                    old_vehicle
                        .passengers
                        .retain(|existing| *existing != passenger_id);
                }
            }
            self.entities[passenger_index].vehicle_id = Some(packet.vehicle_id);
            mounted.push(passenger_id);
        }

        if local_player_was_on_packet_vehicle && !local_player_mounted_here {
            self.local_player_vehicle_id = None;
        }
        self.entities[vehicle_index].passengers = mounted;
        self.counters.entity_passenger_updates_applied += 1;
        true
    }

    pub fn apply_move_vehicle(&mut self, packet: ProtocolMoveVehicle) -> Option<VehicleMoveReport> {
        self.counters.vehicle_moves_received += 1;
        let root_vehicle_id = self.local_player_root_vehicle_id()?;
        let root_vehicle_index = self
            .entities
            .iter()
            .position(|entity| entity.id == root_vehicle_id)?;
        let packet_position = entity_vec3(packet.position);
        let snapped =
            entity_distance_squared(self.entities[root_vehicle_index].position, packet_position)
                > MOVE_VEHICLE_SNAP_EPSILON_SQUARED;

        if snapped {
            let vehicle = &mut self.entities[root_vehicle_index];
            vehicle.position = packet_position;
            vehicle.position_base = packet_position;
            vehicle.y_rot = packet.y_rot;
            vehicle.x_rot = packet.x_rot;
            self.counters.vehicle_moves_snapped += 1;
        }

        self.counters.vehicle_moves_applied += 1;
        self.counters.vehicle_moves_acked += 1;
        let vehicle = &self.entities[root_vehicle_index];
        Some(VehicleMoveReport {
            vehicle_id: vehicle.id,
            position: vehicle.position,
            y_rot: vehicle.y_rot,
            x_rot: vehicle.x_rot,
            on_ground: vehicle.on_ground.unwrap_or(false),
            snapped,
        })
    }

    pub fn apply_set_entity_link(&mut self, packet: ProtocolSetEntityLink) -> bool {
        self.counters.entity_link_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.source_id)
        else {
            return false;
        };

        entity.leash_holder_id = if packet.dest_id == 0 {
            None
        } else {
            Some(packet.dest_id)
        };
        self.counters.entity_link_updates_applied += 1;
        true
    }

    pub fn apply_set_entity_motion(&mut self, packet: ProtocolSetEntityMotion) -> bool {
        self.counters.entity_motion_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.delta_movement = entity_vec3(packet.delta_movement);
        self.counters.entity_motion_updates_applied += 1;
        true
    }

    pub fn apply_rotate_head(&mut self, packet: ProtocolRotateHead) -> bool {
        self.counters.entity_head_rotations_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.y_head_rot = packet.y_head_rot;
        self.counters.entity_head_rotations_applied += 1;
        true
    }

    pub fn apply_take_item_entity(&mut self, packet: ProtocolTakeItemEntity) -> bool {
        self.counters.take_item_entities_received += 1;
        let Some(entity_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == packet.item_id)
        else {
            return false;
        };

        self.counters.take_item_entities_applied += 1;
        let entity_type_id = self.entities[entity_index].entity_type_id;
        if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            return true;
        }

        if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID {
            if let Some(stack) = item_entity_stack_mut(&mut self.entities[entity_index]) {
                if stack.count > 0 && packet.amount > 0 {
                    stack.count = stack.count.saturating_sub(packet.amount).max(0);
                    self.counters.item_entity_stack_shrinks += 1;
                }
                if stack.count > 0 {
                    return true;
                }
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
        let before = self.entities.len();
        self.entities
            .retain(|entity| !removed_ids.contains(&entity.id));
        let removed = before - self.entities.len();
        if self
            .local_player_vehicle_id
            .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
        {
            self.local_player_vehicle_id = None;
        }
        for entity in &mut self.entities {
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
        }
        self.counters.entities_removed += removed;
        self.update_entity_count();
        self.update_active_mob_effect_count();
        removed
    }

    pub fn probe_entity(&self, id: i32) -> Option<&EntityState> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    pub fn local_player_id(&self) -> Option<i32> {
        self.local_player_id
    }

    pub fn local_player_vehicle_id(&self) -> Option<i32> {
        self.local_player_vehicle_id
    }

    pub fn local_player_root_vehicle_id(&self) -> Option<i32> {
        self.resolve_root_vehicle_id(self.local_player_vehicle_id?)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub(crate) fn update_entity_count(&mut self) {
        self.counters.entities_tracked = self.entities.len();
    }

    pub(crate) fn clear_local_player_mount(&mut self, local_player_id: i32) {
        self.local_player_vehicle_id = None;
        for entity in &mut self.entities {
            if entity.id == local_player_id {
                entity.vehicle_id = None;
            }
            entity
                .passengers
                .retain(|passenger_id| *passenger_id != local_player_id);
        }
    }

    fn remove_passenger_from_vehicle(&mut self, vehicle_id: i32, passenger_id: i32) {
        if let Some(vehicle) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == vehicle_id)
        {
            vehicle
                .passengers
                .retain(|existing| *existing != passenger_id);
        }
    }

    fn resolve_root_vehicle_id(&self, vehicle_id: i32) -> Option<i32> {
        let mut root_vehicle_id = vehicle_id;
        for _ in 0..self.entities.len() {
            let vehicle = self.probe_entity(root_vehicle_id)?;
            let Some(parent_vehicle_id) = vehicle.vehicle_id else {
                return Some(root_vehicle_id);
            };
            root_vehicle_id = parent_vehicle_id;
        }
        None
    }
}

fn entity_vec3(vec: ProtocolVec3d) -> EntityVec3 {
    EntityVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

fn entity_distance_squared(a: EntityVec3, b: EntityVec3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
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

#[derive(Debug, Clone, Copy)]
struct EntityMoveRotation {
    position: EntityVec3,
    delta_movement: EntityVec3,
    y_rot: f32,
    x_rot: f32,
}

fn decode_entity_delta_position(base: EntityVec3, xa: i16, ya: i16, za: i16) -> EntityVec3 {
    if xa == 0 && ya == 0 && za == 0 {
        return base;
    }

    EntityVec3 {
        x: decode_entity_delta_axis(base.x, xa),
        y: decode_entity_delta_axis(base.y, ya),
        z: decode_entity_delta_axis(base.z, za),
    }
}

fn decode_entity_delta_axis(base: f64, delta: i16) -> f64 {
    if delta == 0 {
        base
    } else {
        java_round_to_i64(base * 4096.0).saturating_add(i64::from(delta)) as f64 / 4096.0
    }
}

fn java_round_to_i64(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

fn entity_absolute_move_rotation(
    current_position: EntityVec3,
    current_delta_movement: EntityVec3,
    current_y_rot: f32,
    current_x_rot: f32,
    change_position: ProtocolVec3d,
    change_delta_movement: ProtocolVec3d,
    change_y_rot: f32,
    change_x_rot: f32,
    relatives_mask: i32,
) -> EntityMoveRotation {
    let position = EntityVec3 {
        x: absolute_or_relative_f64(
            current_position.x,
            change_position.x,
            relatives_mask,
            PLAYER_RELATIVE_X,
        ),
        y: absolute_or_relative_f64(
            current_position.y,
            change_position.y,
            relatives_mask,
            PLAYER_RELATIVE_Y,
        ),
        z: absolute_or_relative_f64(
            current_position.z,
            change_position.z,
            relatives_mask,
            PLAYER_RELATIVE_Z,
        ),
    };
    let y_rot = absolute_or_relative_f32(
        current_y_rot,
        change_y_rot,
        relatives_mask,
        PLAYER_RELATIVE_Y_ROT,
    );
    let x_rot = absolute_or_relative_f32(
        current_x_rot,
        change_x_rot,
        relatives_mask,
        PLAYER_RELATIVE_X_ROT,
    )
    .clamp(-90.0, 90.0);

    let rotated_delta = if relatives_mask & PLAYER_RELATIVE_ROTATE_DELTA != 0 {
        rotate_entity_delta(
            current_delta_movement,
            current_y_rot - y_rot,
            current_x_rot - x_rot,
        )
    } else {
        current_delta_movement
    };
    let delta_movement = EntityVec3 {
        x: absolute_or_relative_f64(
            rotated_delta.x,
            change_delta_movement.x,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_X,
        ),
        y: absolute_or_relative_f64(
            rotated_delta.y,
            change_delta_movement.y,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_Y,
        ),
        z: absolute_or_relative_f64(
            rotated_delta.z,
            change_delta_movement.z,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_Z,
        ),
    };

    EntityMoveRotation {
        position,
        delta_movement,
        y_rot,
        x_rot,
    }
}

fn absolute_or_relative_f64(current: f64, change: f64, mask: i32, relative_bit: i32) -> f64 {
    if mask & relative_bit != 0 {
        current + change
    } else {
        change
    }
}

fn absolute_or_relative_f32(current: f32, change: f32, mask: i32, relative_bit: i32) -> f32 {
    if mask & relative_bit != 0 {
        current + change
    } else {
        change
    }
}

fn rotate_entity_delta(delta: EntityVec3, y_rot_degrees: f32, x_rot_degrees: f32) -> EntityVec3 {
    let x_rad = f64::from(x_rot_degrees).to_radians();
    let y_rad = f64::from(y_rot_degrees).to_radians();
    let cos_x = x_rad.cos();
    let sin_x = x_rad.sin();
    let after_x = EntityVec3 {
        x: delta.x,
        y: delta.y * cos_x + delta.z * sin_x,
        z: delta.z * cos_x - delta.y * sin_x,
    };
    let cos_y = y_rad.cos();
    let sin_y = y_rad.sin();
    EntityVec3 {
        x: after_x.x * cos_y + after_x.z * sin_y,
        y: after_x.y,
        z: after_x.z * cos_y - after_x.x * sin_y,
    }
}
