use std::collections::BTreeMap;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::{
    AttributeSnapshot as ProtocolAttributeSnapshot, EntityDataValue as ProtocolEntityDataValue,
    EntityDataValueKind, EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate,
    MinecartStep as ProtocolMinecartStep, Vec3d as ProtocolVec3d,
};
use bbb_protocol::RichTextComponent;
use uuid::Uuid;

use super::client_movement::{
    client_movement_uses_collision_facts, experience_orb_movement_from_uuid,
    fishing_hook_movement_from_uuid,
};
use super::content_facts::{entity_is_living, entity_metadata_is_living};
use super::projectiles::{
    entity_abstract_arrow_runtime_from_state, initial_hurting_projectile_state,
    initial_projectile_lifecycle,
};
use super::schema::vanilla_entity_metadata_accessor_for_entity_type;
use super::status::{EntityDamageEventState, MobEffectState};
use super::{
    is_vanilla_boat_type, is_vanilla_minecart_type, AbstractArrowRuntimeState,
    EntityBlockEffectMovements, EntityBlockEffects, EntityClientAnimationState,
    EntityInterpolationState, EntityPickBoundsState, EntityRuntimeReferenceState, EntityState,
    EntityVec3, HurtingProjectileState, LivingEntityBrainState, MinecartFurnaceState,
    ProjectileLifecycleState, VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
    VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_SHULKER_ID,
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

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct EntityDamage {
    pub(crate) last_damage: PublicationShared<Option<EntityDamageEventState>>,
    pub(crate) invulnerable_time: i32,
}

/// Client-owned `LivingEntity.dead` plus the codec-backed brain memories.
/// Present for every living entity so an empty brain stays distinguishable
/// from a non-living entity in canonical snapshots; synchronized health can
/// reach zero without `die`, which `dead` guards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct EntityLiving {
    pub(crate) dead: bool,
    pub(crate) brain: LivingEntityBrainState,
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
    pub(crate) command: bbb_protocol::JavaString,
    pub(crate) last_output: Option<RichTextComponent>,
    pub(crate) track_output: bool,
}

impl Default for EntityMinecartCommandBlock {
    fn default() -> Self {
        Self {
            command: Default::default(),
            last_output: None,
            track_output: true,
        }
    }
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
    pub(crate) in_ground_time: u32,
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
            in_ground_time: 0,
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
    pub(crate) biting: bool,
    pub(crate) left_owner: bool,
    pub(crate) last_deflected_by: Option<i32>,
    pub(crate) random_seed: u64,
}

/// Swept-movement history of the entity families whose client tick samples a
/// collision shape and supporting block, consumed later in the same tick.
#[derive(Debug, Clone, Default)]
pub(crate) struct EntityClientMovementCollisionFacts {
    pub(crate) block_effect_movements: EntityBlockEffectMovements,
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
            transform.position = step.position;
            transform.delta_movement = step.movement;
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
        position: transform.position,
        movement: transform.delta_movement,
        y_rot: transform.y_rot,
        x_rot: transform.x_rot,
        weight: 0.0,
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

pub(super) fn rot_lerp(alpha: f32, from: f32, to: f32) -> f32 {
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

pub(crate) fn sanitize_entity_position(position: EntityVec3, previous: EntityVec3) -> EntityVec3 {
    const MAX_HORIZONTAL: f64 = 3.0e7;
    const MAX_VERTICAL: f64 = 2.0e7;
    EntityVec3 {
        x: if position.x.is_finite() {
            position.x.clamp(-MAX_HORIZONTAL, MAX_HORIZONTAL)
        } else {
            previous.x.clamp(-MAX_HORIZONTAL, MAX_HORIZONTAL)
        },
        y: if position.y.is_finite() {
            position.y.clamp(-MAX_VERTICAL, MAX_VERTICAL)
        } else {
            previous.y.clamp(-MAX_VERTICAL, MAX_VERTICAL)
        },
        z: if position.z.is_finite() {
            position.z.clamp(-MAX_HORIZONTAL, MAX_HORIZONTAL)
        } else {
            previous.z.clamp(-MAX_HORIZONTAL, MAX_HORIZONTAL)
        },
    }
}

pub(crate) fn sanitize_entity_velocity(velocity: EntityVec3) -> EntityVec3 {
    const MAX_VELOCITY: f64 = 10_000.0;
    EntityVec3 {
        x: if velocity.x.is_finite() {
            velocity.x.clamp(-MAX_VELOCITY, MAX_VELOCITY)
        } else {
            0.0
        },
        y: if velocity.y.is_finite() {
            velocity.y.clamp(-MAX_VELOCITY, MAX_VELOCITY)
        } else {
            0.0
        },
        z: if velocity.z.is_finite() {
            velocity.z.clamp(-MAX_VELOCITY, MAX_VELOCITY)
        } else {
            0.0
        },
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
        self.old_position = sanitize_entity_position(position, self.old_position);
        if y_rot.is_finite() {
            self.old_y_rot = y_rot;
        }
        if x_rot.is_finite() {
            self.old_x_rot = (x_rot % 360.0).clamp(-90.0, 90.0);
        }
    }

    pub(crate) fn snap_to(&mut self, position: EntityVec3, y_rot: f32, x_rot: f32) {
        self.position = sanitize_entity_position(position, self.position);
        self.set_y_rot(y_rot);
        self.set_x_rot(x_rot);
        self.set_old_pos_and_rot();
    }

    pub(crate) fn abs_snap_to(&mut self, position: EntityVec3, y_rot: f32, x_rot: f32) {
        self.position = sanitize_entity_position(position, self.position);
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
}

#[derive(Debug, Clone)]
pub(crate) enum EntityMinecartVariant {
    Plain,
    CommandBlock(EntityMinecartCommandBlock),
    Furnace(MinecartFurnaceState),
}

#[derive(Debug, Clone)]
pub(crate) struct EntityMinecartFamily {
    pub(crate) lerp: EntityMinecartLerp,
    pub(crate) variant: EntityMinecartVariant,
}

#[derive(Debug, Clone)]
pub(crate) struct EntityAbstractArrowFamily {
    pub(crate) lifecycle: Option<ProjectileLifecycleState>,
    pub(crate) arrow: EntityAbstractArrowRuntime,
}

#[derive(Debug, Clone)]
pub(crate) struct EntityExperienceOrbFamily {
    pub(crate) movement: EntityExperienceOrbMovement,
    pub(crate) collision_facts: EntityClientMovementCollisionFacts,
}

#[derive(Debug, Clone)]
pub(crate) struct EntityFishingHookFamily {
    pub(crate) movement: EntityFishingHookMovement,
    pub(crate) collision_facts: EntityClientMovementCollisionFacts,
}

#[derive(Debug, Clone)]
pub(crate) struct EntityProjectileFamily {
    pub(crate) lifecycle: Option<ProjectileLifecycleState>,
    pub(crate) hurting: Option<HurtingProjectileState>,
}

/// Disjoint family-specific client state matching the Vanilla entity class hierarchy.
#[derive(Debug, Clone, Default)]
pub(crate) enum EntityFamilyRuntime {
    #[default]
    None,
    Living {
        living: EntityLiving,
        /// `Player.sleepCounter`; Vanilla's wake overlay starts it at 100.
        player_sleep_counter: Option<i32>,
    },
    Minecart(Box<EntityMinecartFamily>),
    Boat(EntityBoatMovement),
    Projectile(Box<EntityProjectileFamily>),
    AbstractArrow(Box<EntityAbstractArrowFamily>),
    ExperienceOrb(Box<EntityExperienceOrbFamily>),
    FishingHook(Box<EntityFishingHookFamily>),
    CollisionOnly(Box<EntityClientMovementCollisionFacts>),
}

impl EntityFamilyRuntime {
    fn from_defaults(
        entity_type_id: i32,
        uuid: Uuid,
        owner: EntitySpawnOwner,
        state: Option<&EntityState>,
        content_registry: &ContentRegistry,
    ) -> Self {
        if entity_metadata_is_living(entity_type_id, content_registry) {
            Self::Living {
                living: EntityLiving::default(),
                player_sleep_counter: (entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID)
                    .then_some(0),
            }
        } else if is_vanilla_minecart_type(entity_type_id) {
            let variant = if entity_type_id == VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID {
                EntityMinecartVariant::CommandBlock(EntityMinecartCommandBlock::default())
            } else if entity_type_id == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID {
                EntityMinecartVariant::Furnace(MinecartFurnaceState::default())
            } else {
                EntityMinecartVariant::Plain
            };
            Self::Minecart(Box::new(EntityMinecartFamily {
                lerp: EntityMinecartLerp::default(),
                variant,
            }))
        } else if is_vanilla_boat_type(entity_type_id) {
            Self::Boat(EntityBoatMovement::default())
        } else if let Some(arrow) = entity_abstract_arrow_runtime_from_state(
            entity_type_id,
            uuid,
            state.and_then(|state| state.abstract_arrow_runtime.as_ref()),
        ) {
            Self::AbstractArrow(Box::new(EntityAbstractArrowFamily {
                lifecycle: initial_projectile_lifecycle(
                    entity_type_id,
                    owner.uuid,
                    owner.is_player,
                ),
                arrow,
            }))
        } else if entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
            Self::FishingHook(Box::new(EntityFishingHookFamily {
                movement: fishing_hook_movement_from_uuid(uuid, owner.uuid),
                collision_facts: EntityClientMovementCollisionFacts::default(),
            }))
        } else if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            Self::ExperienceOrb(Box::new(EntityExperienceOrbFamily {
                movement: experience_orb_movement_from_uuid(uuid),
                collision_facts: EntityClientMovementCollisionFacts::default(),
            }))
        } else if client_movement_uses_collision_facts(entity_type_id) {
            Self::CollisionOnly(Box::default())
        } else {
            let lifecycle =
                initial_projectile_lifecycle(entity_type_id, owner.uuid, owner.is_player);
            let hurting = initial_hurting_projectile_state(entity_type_id);
            if lifecycle.is_some() || hurting.is_some() {
                Self::Projectile(Box::new(EntityProjectileFamily { lifecycle, hurting }))
            } else {
                Self::None
            }
        }
    }

    pub(crate) fn living(&self) -> Option<&EntityLiving> {
        match self {
            Self::Living { living, .. } => Some(living),
            _ => None,
        }
    }

    pub(crate) fn living_mut(&mut self) -> Option<&mut EntityLiving> {
        match self {
            Self::Living { living, .. } => Some(living),
            _ => None,
        }
    }

    pub(crate) fn player_sleep_counter(&self) -> Option<i32> {
        match self {
            Self::Living {
                player_sleep_counter,
                ..
            } => *player_sleep_counter,
            _ => None,
        }
    }

    pub(crate) fn player_sleep_counter_mut(&mut self) -> Option<&mut i32> {
        match self {
            Self::Living {
                player_sleep_counter,
                ..
            } => player_sleep_counter.as_mut(),
            _ => None,
        }
    }

    pub(crate) fn minecart_lerp(&self) -> Option<&EntityMinecartLerp> {
        match self {
            Self::Minecart(minecart) => Some(&minecart.lerp),
            _ => None,
        }
    }

    pub(crate) fn minecart_lerp_mut(&mut self) -> Option<&mut EntityMinecartLerp> {
        match self {
            Self::Minecart(minecart) => Some(&mut minecart.lerp),
            _ => None,
        }
    }

    pub(crate) fn minecart_command_block(&self) -> Option<&EntityMinecartCommandBlock> {
        match self {
            Self::Minecart(minecart) => match &minecart.variant {
                EntityMinecartVariant::CommandBlock(command_block) => Some(command_block),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn minecart_command_block_mut(&mut self) -> Option<&mut EntityMinecartCommandBlock> {
        match self {
            Self::Minecart(minecart) => match &mut minecart.variant {
                EntityMinecartVariant::CommandBlock(command_block) => Some(command_block),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn minecart_furnace(&self) -> Option<&MinecartFurnaceState> {
        match self {
            Self::Minecart(minecart) => match &minecart.variant {
                EntityMinecartVariant::Furnace(furnace) => Some(furnace),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn minecart_furnace_mut(&mut self) -> Option<&mut MinecartFurnaceState> {
        match self {
            Self::Minecart(minecart) => match &mut minecart.variant {
                EntityMinecartVariant::Furnace(furnace) => Some(furnace),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn boat(&self) -> Option<&EntityBoatMovement> {
        match self {
            Self::Boat(boat) => Some(boat),
            _ => None,
        }
    }

    pub(crate) fn boat_mut(&mut self) -> Option<&mut EntityBoatMovement> {
        match self {
            Self::Boat(boat) => Some(boat),
            _ => None,
        }
    }

    pub(crate) fn hurting_projectile(&self) -> Option<&HurtingProjectileState> {
        match self {
            Self::Projectile(projectile) => projectile.hurting.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn hurting_projectile_mut(&mut self) -> Option<&mut HurtingProjectileState> {
        match self {
            Self::Projectile(projectile) => projectile.hurting.as_mut(),
            _ => None,
        }
    }

    pub(crate) fn projectile_lifecycle(&self) -> Option<&ProjectileLifecycleState> {
        match self {
            Self::Projectile(projectile) => projectile.lifecycle.as_ref(),
            Self::AbstractArrow(arrow) => arrow.lifecycle.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn projectile_lifecycle_mut(&mut self) -> Option<&mut ProjectileLifecycleState> {
        match self {
            Self::Projectile(projectile) => projectile.lifecycle.as_mut(),
            Self::AbstractArrow(arrow) => arrow.lifecycle.as_mut(),
            _ => None,
        }
    }

    pub(crate) fn abstract_arrow(&self) -> Option<&EntityAbstractArrowRuntime> {
        match self {
            Self::AbstractArrow(arrow) => Some(&arrow.arrow),
            _ => None,
        }
    }

    pub(crate) fn abstract_arrow_mut(&mut self) -> Option<&mut EntityAbstractArrowRuntime> {
        match self {
            Self::AbstractArrow(arrow) => Some(&mut arrow.arrow),
            _ => None,
        }
    }

    pub(crate) fn projectile_and_arrow(
        &self,
    ) -> Option<(&ProjectileLifecycleState, &EntityAbstractArrowRuntime)> {
        match self {
            Self::AbstractArrow(arrow) => Some((arrow.lifecycle.as_ref()?, &arrow.arrow)),
            _ => None,
        }
    }

    pub(crate) fn projectile_and_arrow_mut(
        &mut self,
    ) -> Option<(
        &mut ProjectileLifecycleState,
        &mut EntityAbstractArrowRuntime,
    )> {
        match self {
            Self::AbstractArrow(arrow) => {
                let EntityAbstractArrowFamily { lifecycle, arrow } = &mut **arrow;
                Some((lifecycle.as_mut()?, arrow))
            }
            _ => None,
        }
    }

    pub(crate) fn experience_orb(&self) -> Option<&EntityExperienceOrbMovement> {
        match self {
            Self::ExperienceOrb(orb) => Some(&orb.movement),
            _ => None,
        }
    }

    pub(crate) fn experience_orb_mut(&mut self) -> Option<&mut EntityExperienceOrbMovement> {
        match self {
            Self::ExperienceOrb(orb) => Some(&mut orb.movement),
            _ => None,
        }
    }

    pub(crate) fn fishing_hook(&self) -> Option<&EntityFishingHookMovement> {
        match self {
            Self::FishingHook(hook) => Some(&hook.movement),
            _ => None,
        }
    }

    pub(crate) fn fishing_hook_mut(&mut self) -> Option<&mut EntityFishingHookMovement> {
        match self {
            Self::FishingHook(hook) => Some(&mut hook.movement),
            _ => None,
        }
    }

    pub(crate) fn collision_facts(&self) -> Option<&EntityClientMovementCollisionFacts> {
        match self {
            Self::ExperienceOrb(orb) => Some(&orb.collision_facts),
            Self::FishingHook(hook) => Some(&hook.collision_facts),
            Self::CollisionOnly(facts) => Some(facts),
            _ => None,
        }
    }

    pub(crate) fn collision_facts_mut(
        &mut self,
    ) -> Option<&mut EntityClientMovementCollisionFacts> {
        match self {
            Self::ExperienceOrb(orb) => Some(&mut orb.collision_facts),
            Self::FishingHook(hook) => Some(&mut hook.collision_facts),
            Self::CollisionOnly(facts) => Some(facts),
            _ => None,
        }
    }
}

/// One tracked client entity. Family-specific members live in the compact
/// `EntityFamilyRuntime` enum matching Vanilla's entity class hierarchy.
#[derive(Debug, Clone)]
pub(crate) struct EntityRecord {
    pub(crate) identity: EntityIdentity,
    pub(crate) transform: EntityTransform,
    pub(crate) supporting_block: EntitySupportingBlock,
    pub(crate) metadata: EntityMetadata,
    pub(crate) equipment: EntityEquipment,
    pub(crate) attributes: EntityAttributes,
    pub(crate) events: EntityTransientEvents,
    pub(crate) mount: EntityMount,
    pub(crate) leash: EntityLeash,
    pub(crate) effects: EntityMobEffects,
    pub(crate) animations: EntityClientAnimationState,
    pub(crate) damage: EntityDamage,
    pub(crate) family: EntityFamilyRuntime,
}

/// The add-entity packet's `data` owner as resolved when the entity spawns.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EntitySpawnOwner {
    pub(crate) uuid: Option<Uuid>,
    pub(crate) is_player: bool,
}

impl EntityRecord {
    pub(crate) fn spawned_with_registry(
        identity: EntityIdentity,
        transform: EntityTransform,
        owner: EntitySpawnOwner,
        content_registry: &ContentRegistry,
    ) -> Self {
        let entity_type_id = identity.entity_type_id;
        let uuid = identity.uuid;
        let mut animations = EntityClientAnimationState {
            base_mob_body_rotation:
                super::animations::MobBodyRotationState::from_spawn_with_registry(
                    entity_type_id,
                    transform.y_head_rot,
                    content_registry,
                ),
            spawn_body_y_rot: entity_is_living(entity_type_id, content_registry).then_some(
                if entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
                    0.0
                } else {
                    transform.y_head_rot
                },
            ),
            ..EntityClientAnimationState::default()
        };
        animations.initialize_spawn_defaults(entity_type_id, uuid);
        Self {
            metadata: entity_spawn_metadata(entity_type_id, transform),
            animations,
            ..Self::family_defaults(identity, transform, owner, None, content_registry)
        }
    }

    pub(crate) fn from_state_with_registry(
        state: &EntityState,
        incarnation: EntityIncarnation,
        owner: EntitySpawnOwner,
        content_registry: &ContentRegistry,
    ) -> Self {
        let identity = EntityIdentity {
            id: state.id,
            uuid: state.uuid,
            entity_type_id: state.entity_type_id,
            data: state.data,
            incarnation,
        };
        let transform = EntityTransform {
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
            interpolation: state.interpolation,
        };
        let mut record =
            Self::family_defaults(identity, transform, owner, Some(state), content_registry);
        let entity_type_id = state.entity_type_id;
        record.supporting_block.main = state.main_supporting_block_pos;
        record.supporting_block.on_ground_no_blocks = state.on_ground_no_blocks;
        record.metadata = EntityMetadata {
            data_values: state.data_values.clone().into(),
            painting_variant_projection: state.painting_variant_projection.clone().into(),
        };
        record.equipment.equipment = state.equipment.clone().into();
        record.attributes.attributes = state.attributes.clone().into();
        record.events = EntityTransientEvents {
            last_animation_action: state.last_animation_action,
            last_event_id: state.last_event_id,
            last_hurt_yaw: state.last_hurt_yaw,
        };
        record.mount = EntityMount {
            vehicle_id: state.vehicle_id,
            passengers: state.passengers.clone().into(),
        };
        record.leash.holder_id = state.leash_holder_id;
        record.effects.effects = state.mob_effects.clone().into();
        record.animations = state.client_animations;
        record.animations.sync_targets_from_metadata_with_registry(
            entity_type_id,
            &state.data_values,
            content_registry,
        );
        record.damage = EntityDamage {
            last_damage: state.last_damage.clone().into(),
            invulnerable_time: state.damage_invulnerable_time,
        };
        if let Some(living) = record.family.living_mut() {
            living.dead = state.living_dead;
            living.brain = state.living_brain.unwrap_or_default();
        }
        if let Some(counter) = record.family.player_sleep_counter_mut() {
            *counter = state.player_sleep_counter.unwrap_or_default().clamp(0, 109);
        }
        if let Some(lerp) = record.family.minecart_lerp_mut() {
            *lerp = EntityMinecartLerp {
                pending_steps: state.minecart_lerp_pending_steps.clone().into(),
                steps: state.minecart_lerp_steps.clone().into(),
                old_step: state.minecart_lerp_old_step,
                delay: state.minecart_lerp_delay,
            };
        }
        if let Some(command_block) = record.family.minecart_command_block_mut() {
            *command_block = EntityMinecartCommandBlock {
                command: state
                    .data_values
                    .iter()
                    .find(|value| value.data_id == VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID)
                    .and_then(|value| match &value.value {
                        EntityDataValueKind::String(command) => Some(command.clone()),
                        _ => None,
                    })
                    .unwrap_or_default(),
                last_output: state
                    .data_values
                    .iter()
                    .find(|value| {
                        value.data_id == VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID
                    })
                    .and_then(|value| match &value.value {
                        EntityDataValueKind::Component(output) => Some(output.clone()),
                        _ => None,
                    }),
                track_output: state.minecart_command_block_track_output.unwrap_or(true),
            };
        }
        if let Some(furnace) = record.family.minecart_furnace_mut() {
            *furnace = state.minecart_furnace.unwrap_or_default();
        }
        if let (Some(projectile), Some(state)) = (
            record.family.hurting_projectile_mut(),
            state.hurting_projectile,
        ) {
            *projectile = state;
        }
        if let (Some(projectile), Some(state)) = (
            record.family.projectile_lifecycle_mut(),
            state.projectile_lifecycle,
        ) {
            *projectile = state;
        }
        record
    }

    fn family_defaults(
        identity: EntityIdentity,
        transform: EntityTransform,
        owner: EntitySpawnOwner,
        state: Option<&EntityState>,
        content_registry: &ContentRegistry,
    ) -> Self {
        let family = EntityFamilyRuntime::from_defaults(
            identity.entity_type_id,
            identity.uuid,
            owner,
            state,
            content_registry,
        );
        Self {
            transform,
            supporting_block: EntitySupportingBlock::default(),
            metadata: EntityMetadata::default(),
            equipment: EntityEquipment::default(),
            attributes: EntityAttributes::default(),
            events: EntityTransientEvents::default(),
            mount: EntityMount::default(),
            leash: EntityLeash::default(),
            effects: EntityMobEffects::default(),
            animations: EntityClientAnimationState::default(),
            damage: EntityDamage::default(),
            family,
            identity,
        }
    }

    /// The serialized probe/control projection.
    pub(crate) fn to_state(
        &self,
        abstract_arrow_runtime: Option<AbstractArrowRuntimeState>,
    ) -> EntityState {
        let transform = &self.transform;
        let lerp = self.family.minecart_lerp();
        let living = self.family.living();
        EntityState {
            id: self.identity.id,
            uuid: self.identity.uuid,
            entity_type_id: self.identity.entity_type_id,
            data: self.identity.data,
            position: transform.position,
            position_base: transform.position_base,
            delta_movement: transform.delta_movement,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            y_head_rot: transform.y_head_rot,
            on_ground: transform.on_ground,
            interpolation: transform.interpolation,
            main_supporting_block_pos: self.supporting_block.main,
            on_ground_no_blocks: self.supporting_block.on_ground_no_blocks,
            data_values: (*self.metadata.data_values).clone(),
            painting_variant_projection: (*self.metadata.painting_variant_projection).clone(),
            equipment: (*self.equipment.equipment).clone(),
            attributes: (*self.attributes.attributes).clone(),
            vehicle_id: self.mount.vehicle_id,
            passengers: (*self.mount.passengers).clone(),
            leash_holder_id: self.leash.holder_id,
            last_animation_action: self.events.last_animation_action,
            last_event_id: self.events.last_event_id,
            last_hurt_yaw: self.events.last_hurt_yaw,
            mob_effects: (*self.effects.effects).clone(),
            client_animations: self.animations,
            last_damage: (*self.damage.last_damage).clone(),
            damage_invulnerable_time: self.damage.invulnerable_time,
            living_dead: living.is_some_and(|living| living.dead),
            living_brain: living.map(|living| living.brain),
            player_sleep_counter: self.family.player_sleep_counter(),
            minecart_lerp_pending_steps: lerp
                .map(|lerp| (*lerp.pending_steps).clone())
                .unwrap_or_default(),
            minecart_lerp_steps: lerp.map(|lerp| (*lerp.steps).clone()).unwrap_or_default(),
            minecart_lerp_old_step: lerp.and_then(|lerp| lerp.old_step),
            minecart_lerp_delay: lerp.map_or(0, |lerp| lerp.delay),
            minecart_command_block_track_output: self
                .family
                .minecart_command_block()
                .map(|command_block| command_block.track_output),
            minecart_furnace: self.family.minecart_furnace().copied(),
            hurting_projectile: self.family.hurting_projectile().copied(),
            projectile_lifecycle: self.family.projectile_lifecycle().copied(),
            abstract_arrow_runtime,
        }
    }
}

/// Vanilla `FallingBlockEntity.recreateFromPacket` seeds the synchronized
/// `DATA_START_POS` from the add-entity position before the first metadata
/// update. Retaining that client default keeps weighted block-model selection
/// stable even before (or in the absence of) the server's dirty metadata item.
fn entity_spawn_metadata(entity_type_id: i32, transform: EntityTransform) -> EntityMetadata {
    let mut metadata = EntityMetadata::default();
    if entity_type_id != VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID {
        return metadata;
    }
    let Some(accessor) =
        vanilla_entity_metadata_accessor_for_entity_type(entity_type_id, "DATA_START_POS")
    else {
        return metadata;
    };
    metadata.data_values.push(ProtocolEntityDataValue {
        data_id: accessor.data_id,
        serializer_id: accessor.serializer_id,
        value: EntityDataValueKind::BlockPos(bbb_protocol::packets::BlockPos {
            x: transform.position.x.floor() as i32,
            y: transform.position.y.floor() as i32,
            z: transform.position.z.floor() as i32,
        }),
    });
    metadata
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::vanilla_entity_metadata_type_is_living;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPES;

    #[test]
    fn compact_record_payloads_preserve_disjoint_family_defaults() {
        assert!(std::mem::size_of::<EntityFamilyRuntime>() <= 64);
        assert!(std::mem::size_of::<crate::BlockEntityPayload>() <= 24);
        assert!(std::mem::size_of::<crate::BlockEntityRecord>() <= 80);

        let owner = EntitySpawnOwner {
            uuid: Some(Uuid::from_u128(0xabc)),
            is_player: true,
        };
        let uuid = Uuid::from_u128(0x1234_5678);
        for type_id in (0..VANILLA_ENTITY_TYPES.len() as i32).chain([-1, i32::MIN, 999_999]) {
            let family = EntityFamilyRuntime::from_defaults(
                type_id,
                uuid,
                owner,
                None,
                ContentRegistry::vanilla(),
            );
            assert_eq!(
                family.living().copied(),
                vanilla_entity_metadata_type_is_living(type_id).then(EntityLiving::default),
                "living mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.player_sleep_counter(),
                (type_id == VANILLA_ENTITY_TYPE_PLAYER_ID).then_some(0),
                "player_sleep_counter mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.minecart_lerp().cloned(),
                is_vanilla_minecart_type(type_id).then(EntityMinecartLerp::default),
                "minecart_lerp mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.minecart_command_block().cloned(),
                (type_id == VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID)
                    .then(EntityMinecartCommandBlock::default),
                "minecart_command_block mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.minecart_furnace().copied(),
                (type_id == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID)
                    .then(MinecartFurnaceState::default),
                "minecart_furnace mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.hurting_projectile().copied(),
                initial_hurting_projectile_state(type_id),
                "hurting_projectile mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.projectile_lifecycle().copied(),
                initial_projectile_lifecycle(type_id, owner.uuid, owner.is_player),
                "projectile_lifecycle mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.abstract_arrow().cloned(),
                entity_abstract_arrow_runtime_from_state(type_id, uuid, None),
                "abstract_arrow mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.experience_orb().copied(),
                (type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID)
                    .then(|| experience_orb_movement_from_uuid(uuid)),
                "experience_orb mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.boat().copied(),
                is_vanilla_boat_type(type_id).then(EntityBoatMovement::default),
                "boat mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.fishing_hook().copied(),
                (type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
                    .then(|| fishing_hook_movement_from_uuid(uuid, owner.uuid)),
                "fishing_hook mismatch for entity_type_id={type_id}",
            );
            assert_eq!(
                family.collision_facts().is_some(),
                client_movement_uses_collision_facts(type_id),
                "collision_facts mismatch for entity_type_id={type_id}",
            );
        }
    }
}
