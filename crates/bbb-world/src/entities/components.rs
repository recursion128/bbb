use std::collections::BTreeMap;

use bbb_protocol::packets::{
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate, MinecartStep as ProtocolMinecartStep,
    Vec3d as ProtocolVec3d,
};
use uuid::Uuid;

use super::status::{EntityDamageEventState, MobEffectState};
use super::{
    EntityClientAnimationState, EntityState, EntityVec3, HurtingProjectileState,
    ProjectileLifecycleState,
};
use crate::BlockPos;

// Match the protocol decoder's per-packet allocation guard while also preventing
// several packets received within one client tick from accumulating without bound.
const MAX_TRACKED_MINECART_LERP_STEPS: usize = 65_536;

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

/// Client-side `Entity.mainSupportingBlockPos` plus the private
/// `onGroundNoBlocks` transition bit used by `Entity.checkSupportingBlock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntitySupportingBlock {
    pub(crate) main: Option<BlockPos>,
    pub(crate) on_ground_no_blocks: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityMetadata {
    pub(crate) data_values: Vec<ProtocolEntityDataValue>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityEquipment {
    pub(crate) equipment: Vec<ProtocolEquipmentSlotUpdate>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityAttributes {
    pub(crate) attributes: Vec<ProtocolAttributeSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct EntityTransientEvents {
    pub(crate) last_animation_action: Option<u8>,
    pub(crate) last_event_id: Option<i8>,
    pub(crate) last_hurt_yaw: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EntityMount {
    pub(crate) vehicle_id: Option<i32>,
    pub(crate) passengers: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntityLeash {
    pub(crate) holder_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EntityMobEffects {
    pub(crate) effects: BTreeMap<i32, MobEffectState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct EntityClientAnimations {
    pub(crate) animations: EntityClientAnimationState,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityDamage {
    pub(crate) last_damage: Option<EntityDamageEventState>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityMinecartLerp {
    pub(crate) pending_steps: Vec<ProtocolMinecartStep>,
    pub(crate) steps: Vec<ProtocolMinecartStep>,
    pub(crate) old_step: Option<ProtocolMinecartStep>,
    pub(crate) delay: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityHurtingProjectile {
    pub(crate) acceleration_power: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityProjectileLifecycle {
    pub(crate) owner_uuid: Option<Uuid>,
    pub(crate) left_owner: bool,
    pub(crate) has_been_shot: bool,
    pub(crate) trident_in_ground_time: u32,
    pub(crate) trident_dealt_damage: bool,
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

impl From<&EntityState> for EntitySupportingBlock {
    fn from(state: &EntityState) -> Self {
        Self {
            main: state.main_supporting_block_pos,
            on_ground_no_blocks: state.on_ground_no_blocks,
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

impl From<&EntityState> for EntityClientAnimations {
    fn from(state: &EntityState) -> Self {
        let mut animations = state.client_animations;
        animations.sync_targets_from_metadata(state.entity_type_id, &state.data_values);
        Self { animations }
    }
}

impl From<&EntityState> for EntityDamage {
    fn from(state: &EntityState) -> Self {
        Self {
            last_damage: state.last_damage.clone(),
        }
    }
}

impl From<&EntityState> for EntityMinecartLerp {
    fn from(state: &EntityState) -> Self {
        Self {
            pending_steps: state.minecart_lerp_pending_steps.clone(),
            steps: state.minecart_lerp_steps.clone(),
            old_step: state.minecart_lerp_old_step,
            delay: state.minecart_lerp_delay,
        }
    }
}

impl EntityMinecartLerp {
    pub(crate) fn queue(&mut self, steps: Vec<ProtocolMinecartStep>) -> bool {
        if self
            .pending_steps
            .len()
            .saturating_add(self.steps.len())
            .saturating_add(steps.len())
            > MAX_TRACKED_MINECART_LERP_STEPS
        {
            return false;
        }
        self.pending_steps.extend(steps);
        true
    }

    pub(crate) fn render_step(&self, partial_ticks: f32) -> Option<ProtocolMinecartStep> {
        let total_weight: f32 = self.steps.iter().map(|step| step.weight).sum();
        let index_alpha = self.current_lerp_step(partial_ticks, total_weight)?;
        let current = *self.steps.get(index_alpha.index)?;
        let previous = if index_alpha.index > 0 {
            *self.steps.get(index_alpha.index - 1)?
        } else {
            self.old_step.unwrap_or(current)
        };
        Some(ProtocolMinecartStep {
            position: lerp_vec3(index_alpha.alpha, previous.position, current.position),
            movement: lerp_vec3(index_alpha.alpha, previous.movement, current.movement),
            y_rot: rot_lerp(index_alpha.alpha, previous.y_rot, current.y_rot),
            x_rot: rot_lerp(index_alpha.alpha, previous.x_rot, current.x_rot),
            weight: current.weight,
        })
    }

    pub(crate) fn advance_client_tick(&mut self, transform: &mut EntityTransform) {
        self.delay = self.delay.saturating_sub(1);
        if self.delay <= 0 {
            self.old_step = Some(minecart_step_from_transform(*transform));
            self.steps.clear();
            if !self.pending_steps.is_empty() {
                self.steps = std::mem::take(&mut self.pending_steps);
                let total_weight: f32 = self.steps.iter().map(|step| step.weight).sum();
                self.delay = if total_weight == 0.0 { 0 } else { 3 };
            }
        }
        if let Some(step) = self.render_step(1.0) {
            transform.position = entity_vec3(step.position);
            transform.delta_movement = entity_vec3(step.movement);
            transform.y_rot = step.y_rot;
            transform.x_rot = step.x_rot;
        }
    }

    fn current_lerp_step(
        &self,
        partial_ticks: f32,
        total_weight: f32,
    ) -> Option<MinecartLerpStepAlpha> {
        if self.steps.is_empty() {
            return None;
        }
        if total_weight == 0.0 {
            return Some(MinecartLerpStepAlpha {
                index: self.steps.len() - 1,
                alpha: 1.0,
            });
        }

        let alpha = (3.0 - self.delay as f32 + partial_ticks) / 3.0;
        let target = total_weight * alpha;
        let mut count_up = 0.0;
        for (index, step) in self.steps.iter().enumerate() {
            let weight = step.weight;
            if weight <= 0.0 {
                continue;
            }
            count_up += weight;
            if count_up >= target {
                let previous_count = count_up - weight;
                return Some(MinecartLerpStepAlpha {
                    index,
                    alpha: (target - previous_count) / weight,
                });
            }
        }
        Some(MinecartLerpStepAlpha {
            index: self.steps.len() - 1,
            alpha: 1.0,
        })
    }
}

fn minecart_step_from_transform(transform: EntityTransform) -> ProtocolMinecartStep {
    ProtocolMinecartStep {
        position: ProtocolVec3d {
            x: transform.position.x,
            y: transform.position.y,
            z: transform.position.z,
        },
        movement: ProtocolVec3d {
            x: transform.delta_movement.x,
            y: transform.delta_movement.y,
            z: transform.delta_movement.z,
        },
        y_rot: transform.y_rot,
        x_rot: transform.x_rot,
        weight: 0.0,
    }
}

fn entity_vec3(vec: ProtocolVec3d) -> EntityVec3 {
    EntityVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MinecartLerpStepAlpha {
    index: usize,
    alpha: f32,
}

fn lerp_vec3(alpha: f32, from: ProtocolVec3d, to: ProtocolVec3d) -> ProtocolVec3d {
    ProtocolVec3d {
        x: lerp_f64(alpha, from.x, to.x),
        y: lerp_f64(alpha, from.y, to.y),
        z: lerp_f64(alpha, from.z, to.z),
    }
}

fn lerp_f64(alpha: f32, from: f64, to: f64) -> f64 {
    from + (to - from) * f64::from(alpha)
}

fn rot_lerp(alpha: f32, from: f32, to: f32) -> f32 {
    from + wrap_degrees(to - from) * alpha
}

fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

impl From<HurtingProjectileState> for EntityHurtingProjectile {
    fn from(state: HurtingProjectileState) -> Self {
        Self {
            acceleration_power: state.acceleration_power,
        }
    }
}

impl From<EntityHurtingProjectile> for HurtingProjectileState {
    fn from(projectile: EntityHurtingProjectile) -> Self {
        Self {
            acceleration_power: projectile.acceleration_power,
        }
    }
}

impl From<ProjectileLifecycleState> for EntityProjectileLifecycle {
    fn from(state: ProjectileLifecycleState) -> Self {
        Self {
            owner_uuid: state.owner_uuid,
            left_owner: state.left_owner,
            has_been_shot: state.has_been_shot,
            trident_in_ground_time: state.trident_in_ground_time,
            trident_dealt_damage: state.trident_dealt_damage,
        }
    }
}

impl From<EntityProjectileLifecycle> for ProjectileLifecycleState {
    fn from(projectile: EntityProjectileLifecycle) -> Self {
        Self {
            owner_uuid: projectile.owner_uuid,
            left_owner: projectile.left_owner,
            has_been_shot: projectile.has_been_shot,
            trident_in_ground_time: projectile.trident_in_ground_time,
            trident_dealt_damage: projectile.trident_dealt_damage,
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

impl EntitySupportingBlock {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.main_supporting_block_pos = self.main;
        state.on_ground_no_blocks = self.on_ground_no_blocks;
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

impl EntityClientAnimations {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.client_animations = self.animations;
    }
}

impl EntityDamage {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.last_damage = self.last_damage;
    }
}

impl EntityMinecartLerp {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.minecart_lerp_pending_steps = self.pending_steps;
        state.minecart_lerp_steps = self.steps;
        state.minecart_lerp_old_step = self.old_step;
        state.minecart_lerp_delay = self.delay;
    }
}

impl EntityHurtingProjectile {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.hurting_projectile = Some(HurtingProjectileState::from(self));
    }
}

impl EntityProjectileLifecycle {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.projectile_lifecycle = Some(ProjectileLifecycleState::from(self));
    }
}
