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

mod animations;
mod components;
mod dimensions;
mod dragon;
mod metadata;
mod movement;
mod passengers;
mod projectiles;
pub(crate) mod state;
mod status;
mod store;
mod updates;

pub use animations::{EntityClientAnimationState, PolarBearStandingAnimationState};
pub(crate) use components::{
    EntityAttributes, EntityClientAnimations, EntityDamage, EntityEquipment,
    EntityHurtingProjectile, EntityIdentity, EntityLeash, EntityMetadata, EntityMinecartLerp,
    EntityMobEffects, EntityMount, EntityTransform, EntityTransientEvents,
};
use dimensions::vanilla_client_position_for_entity_data;
pub use dimensions::EntityPickBoundsState;
pub use dragon::{DragonFlightHistorySample, DragonFlightHistoryState, EnderDragonAnimationState};
use movement::entity_vec3;
use projectiles::initial_hurting_projectile_state;
use status::{EntityDamageEventState, MobEffectState};
pub(crate) use store::EntityStore;

// IDs are the vanilla 26.1 EntityType registry order from EntityType.java.
pub(crate) const VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID: i32 = 0;
pub(crate) const VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID: i32 = 1;
pub(crate) const VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID: i32 = 8;
pub(crate) const VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID: i32 = 9;
pub(crate) const VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID: i32 = 12;
pub(crate) const VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID: i32 = 13;
pub(crate) const VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID: i32 = 18;
pub(crate) const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
pub(crate) const VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID: i32 = 20;
pub(crate) const VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID: i32 = 23;
pub(crate) const VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID: i32 = 24;
pub(crate) const VANILLA_ENTITY_TYPE_CHEST_MINECART_ID: i32 = 25;
pub(crate) const VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID: i32 = 29;
pub(crate) const VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID: i32 = 33;
pub(crate) const VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID: i32 = 34;
pub(crate) const VANILLA_ENTITY_TYPE_DONKEY_ID: i32 = 36;
pub(crate) const VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID: i32 = 37;
pub(crate) const VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID: i32 = 49;
pub(crate) const VANILLA_ENTITY_TYPE_FIREBALL_ID: i32 = 52;
pub(crate) const VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID: i32 = 56;
pub(crate) const VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID: i32 = 65;
pub(crate) const VANILLA_ENTITY_TYPE_HORSE_ID: i32 = 66;
pub(crate) const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
pub(crate) const VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID: i32 = 74;
pub(crate) const VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID: i32 = 75;
pub(crate) const VANILLA_ENTITY_TYPE_LLAMA_ID: i32 = 78;
pub(crate) const VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID: i32 = 81;
pub(crate) const VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID: i32 = 82;
pub(crate) const VANILLA_ENTITY_TYPE_MINECART_ID: i32 = 85;
pub(crate) const VANILLA_ENTITY_TYPE_MULE_ID: i32 = 87;
pub(crate) const VANILLA_ENTITY_TYPE_NAUTILUS_ID: i32 = 88;
pub(crate) const VANILLA_ENTITY_TYPE_OAK_BOAT_ID: i32 = 89;
pub(crate) const VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID: i32 = 90;
pub(crate) const VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID: i32 = 94;
pub(crate) const VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID: i32 = 95;
pub(crate) const VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID: i32 = 116;
pub(crate) const VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID: i32 = 118;
pub(crate) const VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID: i32 = 122;
pub(crate) const VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID: i32 = 125;
pub(crate) const VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID: i32 = 126;
pub(crate) const VANILLA_ENTITY_TYPE_TNT_MINECART_ID: i32 = 133;
pub(crate) const VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID: i32 = 134;
pub(crate) const VANILLA_ENTITY_TYPE_WIND_CHARGE_ID: i32 = 143;
pub(crate) const VANILLA_ENTITY_TYPE_WITHER_SKULL_ID: i32 = 147;
pub(crate) const VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID: i32 = 151;
pub(crate) const VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID: i32 = 152;
pub(crate) const VANILLA_ENTITY_TYPE_PLAYER_ID: i32 = 155;
pub(crate) const VANILLA_ENTITY_SILENT_DATA_ID: u8 = 4;
pub(crate) const VANILLA_ENTITY_NO_GRAVITY_DATA_ID: u8 = 5;
pub(crate) const VANILLA_ITEM_ENTITY_STACK_DATA_ID: u8 = 8;

pub(crate) fn is_vanilla_boat_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID
            | VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID
            | VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID
            | VANILLA_ENTITY_TYPE_BIRCH_BOAT_ID
            | VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_CHERRY_BOAT_ID
            | VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_DARK_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_JUNGLE_BOAT_ID
            | VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_MANGROVE_BOAT_ID
            | VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_PALE_OAK_BOAT_ID
            | VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID
            | VANILLA_ENTITY_TYPE_SPRUCE_BOAT_ID
            | VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID
    )
}

pub(crate) fn is_vanilla_abstract_horse_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

pub(crate) fn is_vanilla_abstract_nautilus_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    )
}

pub(crate) fn is_vanilla_can_equip_saddle_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    )
}

pub(crate) fn is_vanilla_can_wear_horse_armor_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_HORSE_ID | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

pub(crate) fn is_vanilla_horse_slot_always_active_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    )
}

pub(crate) fn is_vanilla_llama_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemEntityStackState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub stack: ProtocolItemStackSummary,
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
    pub client_animations: EntityClientAnimationState,
    #[serde(default)]
    pub last_damage: Option<EntityDamageEventState>,
    #[serde(default)]
    pub minecart_lerp_steps: Vec<ProtocolMinecartStep>,
    #[serde(default)]
    pub hurting_projectile: Option<HurtingProjectileState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HurtingProjectileState {
    pub acceleration_power: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePowerUpdateState {
    pub entity_id: i32,
    pub acceleration_power: f64,
    pub applied: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityTransformState {
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityStatusProbeState {
    pub id: i32,
    pub entity_type_id: i32,
    pub last_animation_action: Option<u8>,
    pub last_event_id: Option<i8>,
    pub last_hurt_yaw: Option<f32>,
    pub mob_effects: BTreeMap<i32, MobEffectState>,
    pub last_damage: Option<EntityDamageEventState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityCameraPoseState {
    pub id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub eye_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPickTargetState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub bounds: EntityPickBoundsState,
}

impl EntityTransformState {
    pub(crate) fn from_components(identity: &EntityIdentity, transform: EntityTransform) -> Self {
        Self {
            id: identity.id,
            uuid: identity.uuid,
            entity_type_id: identity.entity_type_id,
            data: identity.data,
            position: transform.position,
            position_base: transform.position_base,
            delta_movement: transform.delta_movement,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            y_head_rot: transform.y_head_rot,
            on_ground: transform.on_ground,
        }
    }
}

impl WorldStore {
    pub fn apply_add_entity(&mut self, packet: ProtocolAddEntity) {
        self.counters.entities_received += 1;
        let packet_position = entity_vec3(packet.position);
        let entity = EntityState {
            id: packet.id,
            uuid: packet.uuid,
            entity_type_id: packet.entity_type_id,
            data: packet.data,
            position: vanilla_client_position_for_entity_data(
                packet.entity_type_id,
                packet_position,
                packet.data,
                &[],
            )
            .unwrap_or(packet_position),
            position_base: packet_position,
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
            client_animations: EntityClientAnimationState::default(),
            last_damage: None,
            minecart_lerp_steps: Vec::new(),
            hurting_projectile: initial_hurting_projectile_state(packet.entity_type_id),
        };

        self.entities.insert_or_replace(entity);
        self.update_entity_count();
        self.update_active_mob_effect_count();
    }

    pub fn apply_take_item_entity(&mut self, packet: ProtocolTakeItemEntity) -> bool {
        self.counters.take_item_entities_received += 1;
        let Some(entity_type_id) = self.entities.entity_type_id(packet.item_id) else {
            self.counters.take_item_entities_ignored += 1;
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
                .with_metadata_mut(packet.item_id, |metadata| {
                    if let Some(stack) = item_entity_stack_mut(&mut metadata.data_values) {
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
        let received = packet.entity_ids.len();
        self.counters.entity_removes_received += received;
        let removed = self.remove_entities_by_ids(&packet.entity_ids);
        self.counters.entity_removes_ignored += received.saturating_sub(removed);
        removed
    }

    fn remove_entities_by_ids(&mut self, removed_ids: &[i32]) -> usize {
        let removed = self.entities.remove_ids(removed_ids);
        if self
            .local_player_vehicle_id
            .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
        {
            self.local_player_vehicle_id = None;
        }
        self.entities.for_each_mount_mut(|_, mount| {
            if mount
                .vehicle_id
                .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
            {
                mount.vehicle_id = None;
            }
            mount
                .passengers
                .retain(|passenger_id| !removed_ids.contains(passenger_id));
        });
        self.entities.for_each_leash_mut(|_, leash| {
            if leash
                .holder_id
                .is_some_and(|holder_id| removed_ids.contains(&holder_id))
            {
                leash.holder_id = None;
            }
        });
        self.counters.entities_removed += removed;
        self.update_entity_count();
        self.update_active_mob_effect_count();
        removed
    }

    pub fn probe_entity(&self, id: i32) -> Option<EntityState> {
        self.entities.get(id)
    }

    pub fn probe_entity_status(&self, id: i32) -> Option<EntityStatusProbeState> {
        let entity = self.probe_entity(id)?;
        Some(EntityStatusProbeState {
            id: entity.id,
            entity_type_id: entity.entity_type_id,
            last_animation_action: entity.last_animation_action,
            last_event_id: entity.last_event_id,
            last_hurt_yaw: entity.last_hurt_yaw,
            mob_effects: entity.mob_effects,
            last_damage: entity.last_damage,
        })
    }

    pub fn probe_entity_transform(&self, id: i32) -> Option<EntityTransformState> {
        self.entities.transform_state(id)
    }

    pub fn probe_entity_camera_pose(&self, id: i32) -> Option<EntityCameraPoseState> {
        self.entities.camera_pose_state(id)
    }

    pub fn probe_entity_pick_bounds(&self, id: i32) -> Option<EntityPickBoundsState> {
        let identity = self.entities.identity(id)?;
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .player_info_entry(identity.uuid)
                .is_some_and(|info| info.is_spectator())
        {
            return None;
        }
        self.entities.pick_bounds(id)
    }

    pub fn entity_pick_targets(&self) -> Vec<EntityPickTargetState> {
        self.entity_pick_targets_at_partial_tick(1.0)
    }

    pub fn entity_pick_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityPickTargetState> {
        self.entities
            .pick_targets_at_partial_tick(partial_ticks)
            .into_iter()
            .filter(|target| {
                let Some(identity) = self.entities.identity(target.entity_id) else {
                    return true;
                };
                identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    || !self
                        .player_info_entry(identity.uuid)
                        .is_some_and(|info| info.is_spectator())
            })
            .collect()
    }

    pub fn entity_transforms(&self) -> Vec<EntityTransformState> {
        self.entities.transform_states()
    }

    pub fn item_entity_stacks(&self) -> Vec<ItemEntityStackState> {
        self.entities.item_entity_stacks()
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

    pub fn hurting_projectile(&self, id: i32) -> Option<HurtingProjectileState> {
        self.entities
            .hurting_projectile(id)
            .map(HurtingProjectileState::from)
    }

    pub fn last_projectile_power_update(&self) -> Option<&ProjectilePowerUpdateState> {
        self.last_projectile_power.as_ref()
    }

    pub(crate) fn update_entity_count(&mut self) {
        self.counters.entities_tracked = self.entities.len();
    }
}

fn item_entity_stack_mut(
    data_values: &mut [ProtocolEntityDataValue],
) -> Option<&mut ProtocolItemStackSummary> {
    data_values.iter_mut().find_map(|value| {
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
