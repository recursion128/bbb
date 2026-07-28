use std::collections::BTreeMap;

use bbb_protocol::packets::{
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind, EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate,
    MinecartStep as ProtocolMinecartStep, Vec3d as ProtocolVec3d,
};
use bbb_protocol::RichTextComponent;
use uuid::Uuid;

use super::status::{EntityDamageEventState, MobEffectState};
use super::{
    AbstractArrowRuntimeState, EntityBlockEffectMovements, EntityBlockEffects,
    EntityClientAnimationState, EntityInterpolationState, EntityPickBoundsState,
    EntityRuntimeReferenceState, EntityState, EntityVec3, HurtingProjectileState,
    ProjectileLifecycleState, VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
};
use crate::{BlockPos, PaintingVariantClientProjection, PublicationShared};

// Match the protocol decoder's per-packet allocation guard while also preventing
// several packets received within one client tick from accumulating without bound.
const MAX_TRACKED_MINECART_LERP_STEPS: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityIdentity {
    pub(crate) id: i32,
    pub(crate) uuid: Uuid,
    pub(crate) entity_type_id: i32,
    pub(crate) data: i32,
    pub(crate) incarnation: EntityIncarnation,
}

/// Store-local identity for one concrete canonical entity instance. Unlike the
/// protocol id and UUID, this changes on every remove/re-add or same-id spawn
/// replacement, matching Vanilla object identity without entering serialized
/// `EntityState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityIncarnation(pub(crate) u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityInstanceReference {
    pub(crate) entity_id: i32,
    pub(crate) incarnation: EntityIncarnation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityTransform {
    pub(crate) position: EntityVec3,
    pub(crate) old_position: EntityVec3,
    pub(crate) position_base: EntityVec3,
    pub(crate) delta_movement: EntityVec3,
    pub(crate) y_rot: f32,
    pub(crate) old_y_rot: f32,
    pub(crate) x_rot: f32,
    pub(crate) old_x_rot: f32,
    pub(crate) y_head_rot: f32,
    pub(crate) old_y_head_rot: f32,
    pub(crate) on_ground: Option<bool>,
    pub(crate) interpolation: Option<EntityInterpolationState>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityInterpolationCollisionInput {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    pub(crate) current_position: EntityVec3,
    pub(crate) target_position: EntityVec3,
    pub(crate) bounds: EntityPickBoundsState,
}

/// Client-side `Entity.mainSupportingBlockPos` plus the private
/// `onGroundNoBlocks` transition bit used by `Entity.checkSupportingBlock`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct EntitySupportingBlock {
    pub(crate) main: Option<BlockPos>,
    pub(crate) on_ground_no_blocks: bool,
    pub(crate) block_effects: EntityBlockEffects,
}

impl EntitySupportingBlock {
    /// Vanilla `checkSupportingBlock` only rewrites the supporting-block
    /// fields; the private block-effect runtime (stuck multiplier, fire,
    /// powder-snow state) survives airborne movement and is discarded only
    /// with the entity incarnation.
    pub(crate) fn clear_physical(&mut self) {
        self.main = None;
        self.on_ground_no_blocks = false;
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityMetadata {
    pub(crate) data_values: PublicationShared<Vec<ProtocolEntityDataValue>>,
    pub(crate) painting_variant_projection:
        PublicationShared<Option<PaintingVariantClientProjection>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityEquipment {
    pub(crate) equipment: PublicationShared<Vec<ProtocolEquipmentSlotUpdate>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityAttributes {
    pub(crate) attributes: PublicationShared<Vec<ProtocolAttributeSnapshot>>,
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
    pub(crate) passengers: PublicationShared<Vec<i32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntityLeash {
    pub(crate) holder_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct EntityMobEffects {
    pub(crate) effects: PublicationShared<BTreeMap<i32, MobEffectState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct EntityClientAnimations {
    pub(crate) animations: EntityClientAnimationState,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityDamage {
    pub(crate) last_damage: PublicationShared<Option<EntityDamageEventState>>,
    pub(crate) invulnerable_time: i32,
}

/// Client-owned `LivingEntity.dead`, scoped to one concrete entity incarnation.
///
/// Synchronized health can reach zero without invoking `die`, while
/// `LivingEntity.die` uses this field to guard its sleeping/item-use/pose
/// transition. Only living EntityTypes carry this component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntityLivingLifecycle {
    pub(crate) dead: bool,
}

/// Client-owned `Player.sleepCounter`, scoped to one concrete player entity
/// incarnation. Vanilla advances it in `Player.tick` and action 2 of
/// `ClientboundAnimatePacket` starts the ten-tick wake overlay at 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntityPlayerSleep {
    pub(crate) counter: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LivingDeathWakeTransition {
    pub(crate) position: EntityVec3,
    pub(crate) y_rot: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityMinecartLerp {
    pub(crate) pending_steps: PublicationShared<Vec<ProtocolMinecartStep>>,
    pub(crate) steps: PublicationShared<Vec<ProtocolMinecartStep>>,
    pub(crate) old_step: Option<ProtocolMinecartStep>,
    pub(crate) delay: i32,
}

/// Client `MinecartCommandBlock.MinecartCommandBase` projection. `command`
/// and `last_output` change only when their SynchedEntityData accessors do;
/// `track_output` is not synced, starts true for each concrete incarnation,
/// and is mutated directly by the editor before the server confirms anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityMinecartCommandBlock {
    pub(crate) command: String,
    pub(crate) last_output: Option<RichTextComponent>,
    pub(crate) track_output: bool,
}

impl Default for EntityMinecartCommandBlock {
    fn default() -> Self {
        Self {
            command: String::new(),
            last_output: None,
            track_output: true,
        }
    }
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
    pub(crate) trident_return_ticks: u32,
    pub(crate) arrow_pickup_allowed: bool,
    pub(crate) arrow_no_physics: bool,
    pub(crate) arrow_last_block_state_id: Option<i32>,
    pub(crate) arrow_hit_ground_sound_is_arrow: bool,
}

/// Client-owned mutable state used only while resolving AbstractArrow hits.
/// These values affect later collision ordering and random deflection but are
/// not part of `AbstractArrow.addAdditionalSaveData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityAbstractArrowRuntime {
    pub(crate) random_seed: u64,
    pub(crate) last_deflected_by: Option<EntityInstanceReference>,
    pub(crate) pending_last_deflected_by: Option<EntityRuntimeReferenceState>,
    pub(crate) piercing_ignore_entity_ids: Vec<i32>,
}

impl EntityAbstractArrowRuntime {
    pub(crate) fn from_state(uuid: Uuid, state: Option<&AbstractArrowRuntimeState>) -> Self {
        let initial_random_seed = super::animations::java_random_seed(
            super::animations::spawn_random_bits(uuid, 0x6172_726f_775f_726e) as i64,
        );
        Self {
            random_seed: state.map_or(initial_random_seed, |state| state.random_seed),
            last_deflected_by: None,
            pending_last_deflected_by: state.and_then(|state| state.last_deflected_by),
            piercing_ignore_entity_ids: state
                .map(|state| state.piercing_ignore_entity_ids.clone())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntityExperienceOrbMovement {
    pub(crate) following_player_uuid: Option<Uuid>,
    pub(crate) random_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityBoatStatus {
    UnderWater,
    UnderFlowingWater,
    InWater,
    OnLand,
    InAir,
}

/// Canonical client-owned `AbstractBoat` physics state. Packet transforms stay
/// in `EntityTransform`; only the values retained between Vanilla boat ticks
/// live here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityBoatMovement {
    pub(crate) old_status: Option<EntityBoatStatus>,
    pub(crate) status: Option<EntityBoatStatus>,
    pub(crate) water_level: f64,
    pub(crate) land_friction: f32,
    pub(crate) last_yd: f64,
    pub(crate) delta_rotation: f32,
    pub(crate) out_of_control_ticks: f32,
}

impl Default for EntityBoatMovement {
    fn default() -> Self {
        Self {
            // Java fields start as null. This matters on a boat's first water
            // tick: the air-to-water snap runs only after a preceding IN_AIR
            // status, not merely because the boat spawned in water.
            old_status: None,
            status: None,
            water_level: 0.0,
            land_friction: 0.0,
            last_yd: 0.0,
            delta_rotation: 0.0,
            out_of_control_ticks: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum EntityFishingHookState {
    #[default]
    Flying,
    HookedInEntity,
    Bobbing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntityFishingHookMovement {
    pub(crate) state: EntityFishingHookState,
    pub(crate) owner_uuid: Option<Uuid>,
    pub(crate) hooked_entity_uuid: Option<Uuid>,
    pub(crate) hooked_entity_incarnation: Option<EntityIncarnation>,
    pub(crate) biting: bool,
    pub(crate) left_owner: bool,
    pub(crate) last_deflected_by: Option<EntityIncarnation>,
    pub(crate) random_seed: u64,
    pub(crate) synchronized_random_seed: u64,
}

/// Derived family membership for entities whose client tick samples a
/// collision shape and supporting block. Spawn/re-add regenerates membership
/// from the generated Vanilla movement policy. The same conditional owner
/// retains the exact swept-movement history consumed later in the tick, so no
/// second component or parallel membership index is needed.
#[derive(Debug, Clone, Default)]
pub(crate) struct EntityClientMovementCollisionFacts {
    pub(crate) block_effect_movements: EntityBlockEffectMovements,
}

/// Derived family membership for entities whose client tick samples the
/// canonical fluid interaction box, excluding unrelated ECS archetypes.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EntityClientMovementFluidFacts;

/// Exact Player-family marker used by ExperienceOrb's once-per-tick nearest
/// target batch. The projected targets are restored to canonical spawn order.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EntityExperienceOrbPlayerTargetFacts;

impl From<&EntityState> for EntityIdentity {
    fn from(state: &EntityState) -> Self {
        Self {
            id: state.id,
            uuid: state.uuid,
            entity_type_id: state.entity_type_id,
            data: state.data,
            incarnation: EntityIncarnation(0),
        }
    }
}

impl From<&EntityState> for EntityTransform {
    fn from(state: &EntityState) -> Self {
        Self {
            position: state.position,
            old_position: state.position,
            position_base: state.position_base,
            delta_movement: state.delta_movement,
            y_rot: state.y_rot,
            old_y_rot: state.y_rot,
            x_rot: state.x_rot,
            old_x_rot: state.x_rot,
            y_head_rot: state.y_head_rot,
            old_y_head_rot: state.y_head_rot,
            on_ground: state.on_ground,
            interpolation: state.interpolation.clone(),
        }
    }
}

impl From<&EntityState> for EntitySupportingBlock {
    fn from(state: &EntityState) -> Self {
        Self {
            main: state.main_supporting_block_pos,
            on_ground_no_blocks: state.on_ground_no_blocks,
            block_effects: EntityBlockEffects::default(),
        }
    }
}

impl From<&EntityState> for EntityMetadata {
    fn from(state: &EntityState) -> Self {
        Self {
            data_values: state.data_values.clone().into(),
            painting_variant_projection: state.painting_variant_projection.clone().into(),
        }
    }
}

impl From<&EntityState> for EntityEquipment {
    fn from(state: &EntityState) -> Self {
        Self {
            equipment: state.equipment.clone().into(),
        }
    }
}

impl From<&EntityState> for EntityAttributes {
    fn from(state: &EntityState) -> Self {
        Self {
            attributes: state.attributes.clone().into(),
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
            passengers: state.passengers.clone().into(),
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
            effects: state.mob_effects.clone().into(),
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
            last_damage: state.last_damage.clone().into(),
            invulnerable_time: state.damage_invulnerable_time,
        }
    }
}

impl From<&EntityState> for EntityLivingLifecycle {
    fn from(state: &EntityState) -> Self {
        Self {
            dead: state.living_dead,
        }
    }
}

impl From<&EntityState> for EntityPlayerSleep {
    fn from(state: &EntityState) -> Self {
        Self {
            counter: state.player_sleep_counter.unwrap_or_default().clamp(0, 109),
        }
    }
}

impl From<&EntityState> for EntityMinecartLerp {
    fn from(state: &EntityState) -> Self {
        Self {
            pending_steps: state.minecart_lerp_pending_steps.clone().into(),
            steps: state.minecart_lerp_steps.clone().into(),
            old_step: state.minecart_lerp_old_step,
            delay: state.minecart_lerp_delay,
        }
    }
}

impl From<&EntityState> for EntityMinecartCommandBlock {
    fn from(state: &EntityState) -> Self {
        let command = state
            .data_values
            .iter()
            .find(|value| value.data_id == VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID)
            .and_then(|value| match &value.value {
                EntityDataValueKind::String(command) => Some(command.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let last_output = state
            .data_values
            .iter()
            .find(|value| value.data_id == VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Component(output) => Some(output.clone()),
                _ => None,
            });
        Self {
            command,
            last_output,
            track_output: state.minecart_command_block_track_output.unwrap_or(true),
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

impl EntityMinecartCommandBlock {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.minecart_command_block_track_output = Some(self.track_output);
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
            trident_return_ticks: state.trident_return_ticks,
            arrow_pickup_allowed: state.arrow_pickup_allowed,
            arrow_no_physics: state.arrow_no_physics,
            arrow_last_block_state_id: state.arrow_last_block_state_id,
            arrow_hit_ground_sound_is_arrow: state.arrow_hit_ground_sound_is_arrow,
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
            trident_return_ticks: projectile.trident_return_ticks,
            arrow_pickup_allowed: projectile.arrow_pickup_allowed,
            arrow_no_physics: projectile.arrow_no_physics,
            arrow_last_block_state_id: projectile.arrow_last_block_state_id,
            arrow_hit_ground_sound_is_arrow: projectile.arrow_hit_ground_sound_is_arrow,
        }
    }
}

impl EntityTransform {
    pub(crate) fn render_position(self, partial_ticks: f32) -> EntityVec3 {
        EntityVec3 {
            x: lerp_f64(partial_ticks, self.old_position.x, self.position.x),
            y: lerp_f64(partial_ticks, self.old_position.y, self.position.y),
            z: lerp_f64(partial_ticks, self.old_position.z, self.position.z),
        }
    }

    pub(crate) fn render_y_rot(self, partial_ticks: f32) -> f32 {
        if partial_ticks == 1.0 {
            self.y_rot
        } else {
            rot_lerp(partial_ticks, self.old_y_rot, self.y_rot)
        }
    }

    pub(crate) fn render_body_y_rot(self, partial_ticks: f32) -> f32 {
        rot_lerp(partial_ticks, self.old_y_rot, self.y_rot)
    }

    pub(crate) fn render_x_rot(self, partial_ticks: f32) -> f32 {
        self.old_x_rot + (self.x_rot - self.old_x_rot) * partial_ticks
    }

    pub(crate) fn render_y_head_rot(self, partial_ticks: f32) -> f32 {
        rot_lerp(partial_ticks, self.old_y_head_rot, self.y_head_rot)
    }

    pub(crate) fn set_y_rot(&mut self, y_rot: f32) {
        if y_rot.is_finite() {
            self.y_rot = y_rot;
        }
    }

    pub(crate) fn set_x_rot(&mut self, x_rot: f32) {
        if x_rot.is_finite() {
            self.x_rot = (x_rot % 360.0).clamp(-90.0, 90.0);
        }
    }

    pub(crate) fn set_old_pos_and_rot(&mut self) {
        self.set_old_pos_and_rot_to(self.position, self.y_rot, self.x_rot);
    }

    pub(crate) fn set_old_pos_and_rot_to(&mut self, position: EntityVec3, y_rot: f32, x_rot: f32) {
        self.old_position = position;
        self.old_y_rot = y_rot;
        self.old_x_rot = x_rot;
    }

    pub(crate) fn snap_to(&mut self, position: EntityVec3, y_rot: f32, x_rot: f32) {
        self.position = position;
        self.set_y_rot(y_rot);
        self.set_x_rot(x_rot);
        self.set_old_pos_and_rot();
    }

    pub(crate) fn abs_snap_to(&mut self, position: EntityVec3, y_rot: f32, x_rot: f32) {
        self.position = EntityVec3 {
            x: position.x.clamp(-3.0e7, 3.0e7),
            y: position.y,
            z: position.z.clamp(-3.0e7, 3.0e7),
        };
        self.set_y_rot(y_rot % 360.0);
        self.set_x_rot(x_rot.clamp(-90.0, 90.0) % 360.0);
        self.set_old_pos_and_rot();
    }

    pub(crate) fn begin_client_tick(&mut self) {
        self.set_old_pos_and_rot();
        self.old_y_head_rot = self.y_head_rot;
    }

    pub(crate) fn interpolation_target(&self) -> Option<(EntityVec3, f32, f32)> {
        self.interpolation.as_ref().map(|state| {
            (
                state.target_position,
                state.target_y_rot,
                state.target_x_rot,
            )
        })
    }

    pub(crate) fn move_or_interpolate_to(
        &mut self,
        interpolation_steps: Option<u8>,
        position: Option<EntityVec3>,
        y_rot: Option<f32>,
        x_rot: Option<f32>,
    ) {
        let Some(interpolation_steps) = interpolation_steps else {
            self.cancel_interpolation();
            if let Some(position) = position {
                self.position = position;
            }
            if let Some(y_rot) = y_rot {
                self.set_y_rot(y_rot % 360.0);
            }
            if let Some(x_rot) = x_rot {
                self.set_x_rot(x_rot % 360.0);
            }
            return;
        };

        let current_target_position = self
            .interpolation
            .as_ref()
            .map_or(self.position, |state| state.target_position);
        let current_target_y_rot = self
            .interpolation
            .as_ref()
            .map_or(self.y_rot, |state| state.target_y_rot);
        let current_target_x_rot = self
            .interpolation
            .as_ref()
            .map_or(self.x_rot, |state| state.target_x_rot);
        let target_position = position.unwrap_or(current_target_position);
        let target_y_rot = y_rot.unwrap_or(current_target_y_rot);
        let target_x_rot = x_rot.unwrap_or(current_target_x_rot);

        if interpolation_steps == 0 {
            self.snap_to(target_position, target_y_rot, target_x_rot);
            self.cancel_interpolation();
            return;
        }
        if self.interpolation.as_ref().is_some_and(|state| {
            state.target_position == target_position
                && state.target_y_rot == target_y_rot
                && state.target_x_rot == target_x_rot
        }) {
            return;
        }
        self.interpolation = Some(EntityInterpolationState {
            remaining_steps: interpolation_steps,
            target_position,
            target_y_rot,
            target_x_rot,
            previous_position: self.position,
            previous_y_rot: self.y_rot,
            previous_x_rot: self.x_rot,
        });
    }

    pub(crate) fn advance_interpolation_tick(
        &mut self,
        has_interpolation_handler: bool,
        is_ticking: bool,
        can_carry_position_delta: bool,
    ) {
        let Some(mut state) = self.interpolation.take() else {
            return;
        };
        if !has_interpolation_handler || state.remaining_steps == 0 {
            return;
        }
        if !is_ticking {
            self.interpolation = Some(state);
            return;
        }

        // Vanilla carries independent client-side movement into the target only
        // when the shifted target bounding box remains collision-free. WorldStore
        // owns the block/entity/border query and passes that decision into this
        // narrow mutable component step.
        if can_carry_position_delta {
            state.target_position.x += self.position.x - state.previous_position.x;
            state.target_position.y += self.position.y - state.previous_position.y;
            state.target_position.z += self.position.z - state.previous_position.z;
        }
        state.target_y_rot += self.y_rot - state.previous_y_rot;
        state.target_x_rot += self.x_rot - state.previous_x_rot;

        let alpha = 1.0 / f64::from(state.remaining_steps);
        self.position.x += (state.target_position.x - self.position.x) * alpha;
        self.position.y += (state.target_position.y - self.position.y) * alpha;
        self.position.z += (state.target_position.z - self.position.z) * alpha;
        self.set_y_rot(rot_lerp(alpha as f32, self.y_rot, state.target_y_rot) % 360.0);
        self.set_x_rot(self.x_rot + (state.target_x_rot - self.x_rot) * alpha as f32);
        state.remaining_steps -= 1;
        state.previous_position = self.position;
        state.previous_y_rot = self.y_rot;
        state.previous_x_rot = self.x_rot;
        if state.remaining_steps > 0 {
            self.interpolation = Some(state);
        }
    }

    pub(crate) fn cancel_interpolation(&mut self) {
        self.interpolation = None;
    }

    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.position = self.position;
        state.position_base = self.position_base;
        state.delta_movement = self.delta_movement;
        state.y_rot = self.y_rot;
        state.x_rot = self.x_rot;
        state.y_head_rot = self.y_head_rot;
        state.on_ground = self.on_ground;
        state.interpolation = self.interpolation;
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
        state.data_values = self.data_values.into_value();
        state.painting_variant_projection = self.painting_variant_projection.into_value();
    }
}

impl EntityEquipment {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.equipment = self.equipment.into_value();
    }
}

impl EntityAttributes {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.attributes = self.attributes.into_value();
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
        state.passengers = self.passengers.into_value();
    }
}

impl EntityLeash {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.leash_holder_id = self.holder_id;
    }
}

impl EntityMobEffects {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.mob_effects = self.effects.into_value();
    }
}

impl EntityClientAnimations {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.client_animations = self.animations;
    }
}

impl EntityDamage {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.last_damage = self.last_damage.into_value();
        state.damage_invulnerable_time = self.invulnerable_time;
    }
}

impl EntityLivingLifecycle {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.living_dead = self.dead;
    }
}

impl EntityPlayerSleep {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.player_sleep_counter = Some(self.counter);
    }
}

impl EntityMinecartLerp {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.minecart_lerp_pending_steps = self.pending_steps.into_value();
        state.minecart_lerp_steps = self.steps.into_value();
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
