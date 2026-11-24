use super::*;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EntityClientTickParticleTriggers {
    /// Vanilla `MinecartTNT.tick` decrements a positive fuse and then emits one
    /// smoke particle at `y + 0.5` on both client and server.
    pub(crate) minecart_tnt_smoke: bool,
    /// Vanilla `MinecartFurnace.tick` emits one large-smoke particle when the
    /// synced fuel bit is set and the entity RNG's `nextInt(4)` returns zero.
    pub(crate) minecart_furnace_large_smoke: bool,
    /// Vanilla `Blaze.aiStep` emits two large-smoke particles every client tick.
    pub(crate) blaze_large_smoke: Option<[BlazeSmokeParticleOffset; 2]>,
    /// The optional burn sound is ordered before the two smoke particles.
    pub(crate) blaze_burn_sound: Option<BlazeBurnSoundParameters>,
    /// Vanilla `Warden.tick` emits this on the same edge that resets the heart
    /// animation, after its silent gate and two-float voice-pitch draw.
    pub(crate) warden_heartbeat_pitch: Option<f32>,
    /// Ordered `EnderDragon.aiStep` sound edges. Flap always precedes the
    /// mutually-exclusive ambient or sitting-attack growl.
    pub(crate) ender_dragon_sounds: EnderDragonClientSoundTriggers,
    /// Vanilla `LightningBolt.tick` emits thunder then impact on its first
    /// client tick (`life == 2` before decrement).
    pub(crate) lightning_bolt_sounds: Option<LightningBoltSoundParameters>,
    /// Vanilla Enderman/Endermite `aiStep`: two portal particles per tick.
    pub(crate) ender_portal: Option<[EnderAmbientParticleOffset; 2]>,
    /// Vanilla moving, submerged Guardian: two bubbles behind its view vector.
    pub(crate) guardian_moving_bubbles: Option<[GuardianAmbientParticleOffset; 2]>,
    /// Vanilla dry Guardian upward flop sound after a remembered supporting face.
    pub(crate) guardian_flop_sound: bool,
    /// Vanilla dry, grounded Guardian random impulse applied after particles.
    pub(crate) guardian_dry_jump: Option<GuardianDryJumpUpdate>,
    /// Vanilla active-target LookControl result, applied after ambient bubbles.
    pub(crate) guardian_look_update: Option<GuardianLookUpdate>,
    /// Vanilla active-target beam bubbles in exact entity RNG order.
    pub(crate) guardian_attack_bubbles: Option<Vec<EntityVec3>>,
    /// Vanilla `Slime.tick` landing particle ring.
    pub(crate) slime_landing_particles: Option<SlimeLandingParticleTrigger>,
    pub(crate) ravager_stun: Option<RavagerStunParticleOffset>,
    pub(crate) evoker_fangs_crit:
        Option<[EvokerFangsCritParticleOffset; EVOKER_FANGS_CRIT_PARTICLE_COUNT]>,
    /// Vanilla `AbstractNautilus.tick` clears DASH before decrementing its cooldown.
    pub(crate) nautilus_clear_dash: bool,
    pub(crate) nautilus_bubble: Option<EntityVec3>,
    /// Vanilla `CopperGolem.setupAnimationStates` local head-spin sound edge (`CopperGolem.java:324-327`).
    pub(crate) copper_golem_spin_sound: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlazeSmokeParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_unit: f64,
    pub(crate) z_unit: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlazeBurnSoundParameters {
    pub(crate) volume: f32,
    pub(crate) pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LightningBoltSoundParameters {
    pub(crate) thunder_pitch: f32,
    pub(crate) impact_pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EnderAmbientParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_unit: f64,
    pub(crate) z_unit: f64,
    pub(crate) velocity: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianAmbientParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_unit: f64,
    pub(crate) z_unit: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianDryJumpUpdate {
    pub(crate) x_impulse: f64,
    pub(crate) z_impulse: f64,
    pub(crate) y_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianLookUpdate {
    pub(crate) x_rot: f32,
    pub(crate) y_head_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GuardianAttackTargetTickInput {
    pub(crate) position: EntityVec3,
    pub(crate) eye_height: f32,
    pub(crate) half_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlimeLandingParticleTrigger {
    pub(crate) kind: super::super::SlimeLandingParticleKind,
    pub(crate) offsets: Vec<EntityVec3>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RavagerStunParticleOffset {
    pub(crate) jitter_x: f64,
    pub(crate) jitter_z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EvokerFangsCritParticleOffset {
    pub(crate) x_unit: f64,
    pub(crate) y_offset: f64,
    pub(crate) z_unit: f64,
    pub(crate) velocity: EntityVec3,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityClientAnimationState {
    #[serde(default)]
    pub kind: EntityKindAnimation,
    #[serde(default)]
    pub age_ticks: u32,
    /// Vanilla `LivingEntity.recreateFromPacket` seeds both body-yaw endpoints
    /// from the packet head yaw. `Shulker.recreateFromPacket` immediately
    /// overrides both to zero. This value is consumed only before the first
    /// client tick by entities without the base mob body controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spawn_body_y_rot: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) base_mob_body_rotation: Option<MobBodyRotationState>,
    /// Vanilla `AgeableMob.ageLockParticleTimer`. A local golden-dandelion interaction starts this
    /// at 40 ticks; while nonzero it also prevents another local age-lock interaction.
    #[serde(skip)]
    pub(crate) age_lock_particle_timer_ticks: u8,
    /// Vanilla `ItemBasedSteering` client-only `boosting`/`boostTime` owner for
    /// Pig and Strider. The synchronized total remains in canonical metadata.
    #[serde(skip)]
    pub(crate) item_based_steering: Option<ItemBasedSteeringClientState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hurt: Option<HurtAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_rotation_lerp: Option<HeadRotationLerpState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinetic_hit_feedback: Option<KineticHitFeedbackAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_swing: Option<AttackSwingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death: Option<DeathAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub living_swim: Option<LivingSwimAmountState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elytra: Option<ElytraAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossbow_charge: Option<CrossbowChargeAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_animation: Option<WalkAnimationState>,
    /// Client-owned fields consulted by repeated local mob interactions.
    /// Vanilla does not synchronize these fields: tameable sitting intent,
    /// a brown mooshroom's pending stew ingredient, and Tadpole's private age.
    #[serde(skip)]
    pub(crate) ordered_to_sit: bool,
    #[serde(skip)]
    pub(crate) mooshroom_stew_ingredient_item_id: Option<i32>,
    #[serde(skip)]
    pub(crate) tadpole_age_ticks: i32,
    /// Shared deterministic stand-in for Vanilla `Entity.random`. Local
    /// interactions and unfiltered synchronized enchantment effects consume
    /// this cursor in call order. It is retained per incarnation and kept out
    /// of stable probe serialization like the other client RNG cursors.
    #[serde(skip)]
    pub(crate) entity_random: Option<LevelEventSoundRandomState>,
    /// Client-local `PlayerRideableJumping` input retained between the local
    /// player's release edge and the controlled vehicle's ridden tick. It is
    /// cloned into immutable publications for old-generation isolation but is
    /// omitted from the stable renderer/probe serialization shape.
    #[serde(skip)]
    pub(crate) ridden_jump: Option<RiddenJumpState>,
}

/// Vanilla `AbstractHorse.playerJumpPendingScale`, `allowStandSliding`, and
/// the local `standCounter` started by `onPlayerJump`. This belongs to the
/// canonical entity client-tick owner; it is not a parallel input cache.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct RiddenJumpState {
    pub(crate) player_jump_pending_scale: f32,
    pub(crate) allow_stand_sliding: bool,
    pub(crate) local_stand_ticks_remaining: u8,
}

/// The client-observable `BaseSpawner` state owned by `MinecartSpawner`. Vanilla constructs it
/// with delay `20` / minimum delay `200`; delegated entity event `1` resets delay to the minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecartSpawnerClientState {
    pub spawn_delay: i32,
    pub min_spawn_delay: i32,
}

/// Vanilla `VehicleEntity` hurt-roll state as consumed by `AbstractBoatRenderer` and
/// `AbstractMinecartRenderer`: `hurtTime`, `hurtDir`, and `damage` are synced data values that the
/// client decrements every tick before rendering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoatDamageAnimationState {
    pub hurt_time: i32,
    pub hurt_dir: i32,
    pub damage: f32,
}

impl BoatDamageAnimationState {
    pub(super) fn from_metadata(
        accessors: VanillaVehicleMetadataAccessors,
        data_values: &[EntityDataValue],
    ) -> Self {
        Self {
            hurt_time: entity_data_int(
                data_values,
                accessors.hurt_time_data_id,
                accessors.hurt_time_default,
            ),
            hurt_dir: entity_data_int(
                data_values,
                accessors.hurt_dir_data_id,
                accessors.hurt_dir_default,
            ),
            damage: entity_data_float(
                data_values,
                accessors.damage_data_id,
                accessors.damage_default,
            ),
        }
    }

    pub(super) fn advance_client_tick(&mut self) {
        if self.hurt_time > 0 {
            self.hurt_time -= 1;
        }
        if self.damage > 0.0 {
            self.damage = (self.damage - 1.0).max(0.0);
        }
    }

    pub(super) fn animate_hurt(&mut self) {
        self.hurt_dir = -self.hurt_dir;
        self.hurt_time = HURT_ANIMATION_DURATION;
        self.damage *= 11.0;
    }

    pub(super) fn render_state(self, partial_tick: f32) -> (f32, i32, f32) {
        (
            self.hurt_time as f32 - partial_tick,
            self.hurt_dir,
            (self.damage - partial_tick).max(0.0),
        )
    }

    pub(super) fn is_settled(self) -> bool {
        self.hurt_time <= 0 && self.damage <= 0.0
    }
}

impl Default for BoatDamageAnimationState {
    fn default() -> Self {
        let accessors = vanilla_entity_metadata_vehicle_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        )
        .expect("generated oak-boat VehicleEntity metadata accessors");
        Self::from_metadata(accessors, &[])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MinecartTntFuseAnimationState {
    pub fuse_remaining_ticks: i32,
}

impl MinecartTntFuseAnimationState {
    pub(super) fn advance_client_tick(&mut self) {
        if self.fuse_remaining_ticks > 0 {
            self.fuse_remaining_ticks -= 1;
        }
    }

    pub(super) fn fuse_remaining_in_ticks(&self, partial_tick: f32) -> f32 {
        self.fuse_remaining_ticks as f32 - partial_tick + 1.0
    }
}

/// Vanilla `AbstractBoat` bubble-column wobble state: the client keeps a
/// multiplier and previous/current angles, increasing the multiplier while
/// synced `bubbleTime > 0` and easing it back to zero otherwise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoatBubbleAnimationState {
    pub multiplier: f32,
    pub previous_angle: f32,
    pub current_angle: f32,
}

impl BoatBubbleAnimationState {
    pub(super) fn advance_client_tick(&mut self, age_ticks: u32, bubble_time: i32) {
        if bubble_time > 0 {
            self.multiplier += 0.05;
        } else {
            self.multiplier -= 0.1;
        }
        self.multiplier = self.multiplier.clamp(0.0, 1.0);
        self.previous_angle = self.current_angle;
        self.current_angle = 10.0 * (0.5 * age_ticks as f32).sin() * self.multiplier;
    }

    pub(super) fn angle(self, partial_tick: f32) -> f32 {
        self.previous_angle + partial_tick * (self.current_angle - self.previous_angle)
    }

    pub(super) fn is_settled(self, bubble_time: i32) -> bool {
        bubble_time <= 0
            && self.multiplier == 0.0
            && self.previous_angle == 0.0
            && self.current_angle == 0.0
    }
}

/// Vanilla `AbstractBoat.paddlePositions`: client-side rowing accumulators for
/// the left and right paddles. `AbstractBoatRenderer.extractRenderState` projects
/// these through `getRowingTime(side, partialTick)` before `AbstractBoatModel`
/// turns them into paddle rotations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoatPaddleAnimationState {
    pub left: f32,
    pub right: f32,
}

impl BoatPaddleAnimationState {
    pub(super) fn advance_client_tick(&mut self, left_active: bool, right_active: bool) {
        self.left = advance_boat_paddle(self.left, left_active);
        self.right = advance_boat_paddle(self.right, right_active);
    }

    pub(super) fn rowing_times(
        self,
        partial_tick: f32,
        left_active: bool,
        right_active: bool,
    ) -> [f32; 2] {
        [
            boat_rowing_time(self.left, partial_tick, left_active),
            boat_rowing_time(self.right, partial_tick, right_active),
        ]
    }

    pub(super) fn is_settled(self) -> bool {
        self.left == 0.0 && self.right == 0.0
    }
}

pub(super) fn advance_boat_paddle(position: f32, active: bool) -> f32 {
    if active {
        position + BOAT_PADDLE_ADVANCE_PER_TICK
    } else {
        0.0
    }
}

pub(super) fn boat_rowing_time(position: f32, partial_tick: f32, active: bool) -> f32 {
    if active {
        let start = position - BOAT_PADDLE_ADVANCE_PER_TICK;
        start + partial_tick.clamp(0.0, 1.0) * (position - start)
    } else {
        0.0
    }
}

/// Canonical client-side arrow impact wobble, mirroring vanilla
/// `AbstractArrow.shakeTime`. A metadata update of `IN_GROUND` to `true` sets it
/// to 7 once the entity has ticked at least once; each client tick decrements it,
/// and `ArrowRenderer.extractRenderState` projects `shakeTime - partialTick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowShakeAnimationState {
    pub shake_time: i32,
}

impl ArrowShakeAnimationState {
    pub(super) fn started() -> Self {
        Self {
            shake_time: ARROW_SHAKE_TICKS,
        }
    }

    pub(super) fn advance_client_tick(&mut self) {
        self.shake_time = (self.shake_time - 1).max(0);
    }

    pub(super) fn shake(self, partial_tick: f32) -> f32 {
        self.shake_time as f32 - partial_tick
    }

    pub(super) fn is_settled(self) -> bool {
        self.shake_time <= 0
    }
}
