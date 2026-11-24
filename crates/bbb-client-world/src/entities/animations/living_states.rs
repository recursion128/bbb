use super::*;

/// Desired per-tick target for one vanilla wither side head. When the synced
/// target entity is visible, both pitch and yaw lerp toward it; when it is absent
/// or unknown, vanilla only lerps yaw back toward `yBodyRot` and leaves pitch as-is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WitherHeadTargetRotations {
    pub(crate) x_rot: Option<f32>,
    pub(crate) y_rot: f32,
}

impl WitherHeadTargetRotations {
    pub(crate) fn fallback_to_body(y_body_rot: f32) -> Self {
        Self {
            x_rot: None,
            y_rot: y_body_rot,
        }
    }
}

/// Canonical client-side wither side-head rotations, mirroring vanilla
/// `WitherBoss.xRotHeads` / `yRotHeads`. The renderer copies these current arrays
/// directly into `WitherRenderState`; 26.1 does not partial-lerp the side heads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct WitherHeadRotationsState {
    pub x_head_rots: [f32; 2],
    pub y_head_rots: [f32; 2],
}

impl WitherHeadRotationsState {
    pub(super) fn advance_client_tick(&mut self, targets: [WitherHeadTargetRotations; 2]) {
        for (index, target) in targets.into_iter().enumerate() {
            if let Some(x_rot) = target.x_rot {
                self.x_head_rots[index] = rotlerp(self.x_head_rots[index], x_rot, 40.0);
            }
            self.y_head_rots[index] = rotlerp(self.y_head_rots[index], target.y_rot, 10.0);
        }
    }
}

/// Canonical client-side creeper fuse animation, mirroring vanilla
/// `Creeper.swell`/`oldSwell`. The synced `DATA_SWELL_DIR` (forced to `1` while
/// ignited) advances `current_swell` toward `maxSwell` each client tick;
/// `getSwelling` lerps it for the renderer white swelling overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreeperSwellAnimationState {
    /// Effective fuse direction (`DATA_SWELL_DIR`, or `1` while ignited).
    pub swell_dir: i32,
    /// Vanilla `Creeper.oldSwell`.
    pub previous_swell: i32,
    /// Vanilla `Creeper.swell`.
    pub current_swell: i32,
}

/// Canonical client-side hurt animation countdown, mirroring vanilla
/// `LivingEntity.hurtTime`. A hurt animation or damage event resets it to
/// [`HURT_ANIMATION_DURATION`]; each client tick decrements it toward `0`. While
/// it is positive the renderer projects the red damage overlay
/// (`LivingEntityRenderState.hasRedOverlay`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HurtAnimationState {
    pub hurt_time: i32,
}

/// Vanilla `LivingEntity.lerpYHeadRot` plus `lerpHeadSteps`, armed by
/// `ClientboundRotateHeadPacket` and advanced once per client tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeadRotationLerpState {
    pub target_y_head_rot: f32,
    pub steps_remaining: i32,
}

/// Canonical client-side kinetic hit feedback timer, mirroring vanilla
/// `LivingEntity.lastKineticHitFeedbackTime`. Entity event `2` calls
/// `onKineticHit`; the renderer reads
/// `getTicksSinceLastKineticHitFeedback(partialTicks)` to kick spear items
/// backward for the vanilla ten-tick feedback curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KineticHitFeedbackAnimationState {
    pub elapsed_ticks: i32,
}

/// Canonical armor-stand hit wobble timer. Vanilla stores the hit game time in
/// `ArmorStand.lastHit`; `ArmorStandRenderer.extractRenderState` exposes
/// `wiggle = level.getGameTime() - lastHit + partialTicks`, and
/// `setupRotations` applies it only while `< ArmorStand.WOBBLE_TIME`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmorStandWiggleAnimationState {
    pub elapsed_ticks: i32,
}

/// Canonical client-side melee-swing animation, mirroring vanilla `LivingEntity`'s
/// `swingTime`/`attackAnim`/`oAttackAnim`. The `ClientboundAnimate` packet (action `0`
/// main hand / `3` off hand) calls `swing()` ([`EntityClientAnimationState::trigger_swing`]),
/// which arms the current hand item's `getSwingAnimation().duration()` ramp (defaulting
/// to [`ATTACK_SWING_DURATION`]); `updateSwingTime` advances it each client tick and the
/// renderer projects `getAttackAnim(partialTick)` into `HumanoidModel.setupAttackAnimation`'s
/// body twist + arm swing. `oAttackAnim` is kept for the partial-tick lerp.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttackSwingAnimationState {
    /// Vanilla `LivingEntity.swinging`: a swing is currently playing.
    pub swinging: bool,
    /// Vanilla `LivingEntity.swingTime`: the integer tick counter, `-1` for the tick
    /// just after `swing()` before the first `updateSwingTime`.
    pub swing_time: i32,
    /// Vanilla `LivingEntity.getCurrentSwingDuration()` for this swing. The value is
    /// refreshed when the active hand item or relevant mob effects change, matching
    /// the runtime duration `updateSwingTime` observes.
    #[serde(default = "default_attack_swing_duration")]
    pub duration: i32,
    /// Vanilla `LivingEntity.attackAnim`: `swingTime / duration`, the current-tick value.
    pub attack_anim: f32,
    /// Vanilla `LivingEntity.oAttackAnim`: the previous-tick `attackAnim`, the lerp start.
    pub prev_attack_anim: f32,
    /// Whether the swing is the off hand (left arm); vanilla `swingingArm`.
    pub off_hand: bool,
}

/// Canonical client-side death animation counter, mirroring vanilla
/// `LivingEntity.deathTime`. It begins when the synced health (`DATA_HEALTH_ID`)
/// reaches `<= 0` (`LivingEntity.isDeadOrDying`) and increments each client tick
/// (`LivingEntity.tickDeath`). While it is positive the renderer tips the entity
/// over (`LivingEntityRenderer.setupRotations`) and projects the red damage
/// overlay (`hasRedOverlay`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeathAnimationState {
    pub death_time: i32,
}

/// Canonical client-side `LivingEntity` swim-amount accumulator, mirroring vanilla
/// `swimAmountO` / `swimAmount`. Each client tick saves the previous value, then eases
/// toward `1.0` while `isVisuallySwimming()` and toward `0.0` otherwise by `0.09`.
/// The drowned consumes this today for `DrownedModel.setupAnim` and
/// `DrownedRenderer.setupRotations`; the state shape is generic so player/humanoid
/// swim poses can reuse it later.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingSwimAmountState {
    pub visually_swimming: bool,
    pub previous_swim_amount: f32,
    pub current_swim_amount: f32,
}

impl LivingSwimAmountState {
    pub(super) fn set_visually_swimming(&mut self, visually_swimming: bool) {
        self.visually_swimming = visually_swimming;
    }

    pub(super) fn advance_client_tick(&mut self) {
        self.previous_swim_amount = self.current_swim_amount;
        if self.visually_swimming {
            self.current_swim_amount = (self.current_swim_amount + LIVING_SWIM_AMOUNT_PER_TICK)
                .min(LIVING_SWIM_AMOUNT_MAX);
        } else {
            self.current_swim_amount =
                (self.current_swim_amount - LIVING_SWIM_AMOUNT_PER_TICK).max(0.0);
        }
    }

    pub(super) fn swim_amount(self, partial_tick: f32) -> f32 {
        self.previous_swim_amount
            + partial_tick * (self.current_swim_amount - self.previous_swim_amount)
    }

    pub(super) fn is_settled(self) -> bool {
        !self.visually_swimming
            && self.previous_swim_amount == 0.0
            && self.current_swim_amount == 0.0
    }
}

/// Canonical client-side elytra wing rotation accumulator, mirroring vanilla
/// `LivingEntity.elytraAnimationState`'s per-tick target/ease/partial-lerp behavior.
/// `HumanoidMobRenderer.extractHumanoidRenderState` samples it into
/// `HumanoidRenderState.elytraRotX/Y/Z`, and `WingsLayer` consumes those values
/// when a WINGS chest equipment layer is present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElytraAnimationState {
    pub rot_x: f32,
    pub rot_y: f32,
    pub rot_z: f32,
    pub rot_x_old: f32,
    pub rot_y_old: f32,
    pub rot_z_old: f32,
}

impl Default for ElytraAnimationState {
    fn default() -> Self {
        // Source rows and renderer defaults use the steady non-flying pose when no
        // ticked state exists. Start the accumulator there so native snapshots do not
        // briefly fold wings before the first client-animation tick; the tick path
        // below still follows vanilla's target/ease/partial-lerp math.
        Self {
            rot_x: ELYTRA_DEFAULT_X_ROT,
            rot_y: ELYTRA_DEFAULT_Y_ROT,
            rot_z: ELYTRA_DEFAULT_Z_ROT,
            rot_x_old: ELYTRA_DEFAULT_X_ROT,
            rot_y_old: ELYTRA_DEFAULT_Y_ROT,
            rot_z_old: ELYTRA_DEFAULT_Z_ROT,
        }
    }
}

impl ElytraAnimationState {
    pub(super) fn advance_client_tick(
        &mut self,
        is_fall_flying: bool,
        is_crouching: bool,
        delta_movement: EntityVec3,
    ) {
        self.rot_x_old = self.rot_x;
        self.rot_y_old = self.rot_y;
        self.rot_z_old = self.rot_z;

        let (target_x_rot, target_y_rot, target_z_rot) = if is_fall_flying {
            let ratio = elytra_fall_flying_ratio(delta_movement);
            (
                lerp_f32(ratio, ELYTRA_DEFAULT_X_ROT, ELYTRA_FALL_FLYING_X_ROT),
                ELYTRA_DEFAULT_Y_ROT,
                lerp_f32(ratio, ELYTRA_DEFAULT_Z_ROT, ELYTRA_FALL_FLYING_Z_ROT),
            )
        } else if is_crouching {
            (
                ELYTRA_CROUCHING_X_ROT,
                ELYTRA_CROUCHING_Y_ROT,
                ELYTRA_CROUCHING_Z_ROT,
            )
        } else {
            (
                ELYTRA_DEFAULT_X_ROT,
                ELYTRA_DEFAULT_Y_ROT,
                ELYTRA_DEFAULT_Z_ROT,
            )
        };

        self.rot_x += (target_x_rot - self.rot_x) * ELYTRA_ROT_EASE;
        self.rot_y += (target_y_rot - self.rot_y) * ELYTRA_ROT_EASE;
        self.rot_z += (target_z_rot - self.rot_z) * ELYTRA_ROT_EASE;
    }

    pub(super) fn rot_x(self, partial_tick: f32) -> f32 {
        lerp_f32(partial_tick, self.rot_x_old, self.rot_x)
    }

    pub(super) fn rot_y(self, partial_tick: f32) -> f32 {
        lerp_f32(partial_tick, self.rot_y_old, self.rot_y)
    }

    pub(super) fn rot_z(self, partial_tick: f32) -> f32 {
        lerp_f32(partial_tick, self.rot_z_old, self.rot_z)
    }
}

pub(super) fn elytra_fall_flying_ratio(delta_movement: EntityVec3) -> f32 {
    if delta_movement.y >= 0.0 {
        return 1.0;
    }
    let length = (delta_movement.x * delta_movement.x
        + delta_movement.y * delta_movement.y
        + delta_movement.z * delta_movement.z)
        .sqrt();
    if length < 1.0e-5 {
        return 1.0;
    }
    let normalized_y = (delta_movement.y / length) as f32;
    1.0 - (-normalized_y).powf(1.5)
}

pub(super) fn lerp_f32(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

pub(super) fn lerp_f64(delta: f32, start: f64, end: f64) -> f64 {
    start + f64::from(delta) * (end - start)
}

pub(super) fn lerp_vec3(delta: f32, start: EntityVec3, end: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: lerp_f64(delta, start.x, end.x),
        y: lerp_f64(delta, start.y, end.y),
        z: lerp_f64(delta, start.z, end.z),
    }
}

pub(super) fn horizontal_distance(delta: EntityVec3) -> f32 {
    (delta.x * delta.x + delta.z * delta.z).sqrt() as f32
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerCloakAnimationState {
    pub initialized: bool,
    pub entity_position_old: EntityVec3,
    pub entity_position: EntityVec3,
    pub cloak_position_old: EntityVec3,
    pub cloak_position: EntityVec3,
    pub walk_dist_old: f32,
    pub walk_dist: f32,
    pub bob_old: f32,
    pub bob: f32,
    pub fall_flying_ticks: u32,
    /// Vanilla `ClientAvatarState.deltaMovementOnPreviousTick`. Despite the
    /// name, `ClientAvatarState.tick` replaces it with the current movement once
    /// per client tick; render extraction lerps this snapshot toward the live
    /// entity movement for the partial frame.
    #[serde(default)]
    pub delta_movement_on_previous_tick: EntityVec3,
}

impl Default for PlayerCloakAnimationState {
    fn default() -> Self {
        Self {
            initialized: false,
            entity_position_old: EntityVec3::default(),
            entity_position: EntityVec3::default(),
            cloak_position_old: EntityVec3::default(),
            cloak_position: EntityVec3::default(),
            walk_dist_old: 0.0,
            walk_dist: 0.0,
            bob_old: 0.0,
            bob: 0.0,
            fall_flying_ticks: 0,
            delta_movement_on_previous_tick: EntityVec3::default(),
        }
    }
}

impl PlayerCloakAnimationState {
    /// Vanilla `ClientAvatarState.tick` / `moveCloak` and `AbstractClientPlayer.updateBob`.
    /// Local walk distance advances after movement; remote players use synced feet travel.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::entities) fn advance_client_tick(
        &mut self,
        position: EntityVec3,
        delta_movement: EntityVec3,
        on_ground: bool,
        is_alive: bool,
        is_swimming: bool,
        is_fall_flying: bool,
        advance_walk_distance: bool,
    ) {
        if !self.initialized {
            self.initialized = true;
            self.entity_position_old = position;
            self.entity_position = position;
            self.cloak_position_old = position;
            self.cloak_position = position;
        }

        self.delta_movement_on_previous_tick = delta_movement;

        let previous_position = self.entity_position;
        self.entity_position_old = previous_position;
        self.entity_position = position;
        if advance_walk_distance {
            self.advance_walk_distance(EntityVec3 {
                x: position.x - previous_position.x,
                y: position.y - previous_position.y,
                z: position.z - previous_position.z,
            });
        }

        self.move_cloak(position);

        self.bob_old = self.bob;
        let target_bob = if on_ground && is_alive && !is_swimming {
            horizontal_distance(delta_movement).min(0.1)
        } else {
            0.0
        };
        self.bob += (target_bob - self.bob) * 0.4;

        self.fall_flying_ticks = if is_fall_flying {
            self.fall_flying_ticks.saturating_add(1)
        } else {
            0
        };
    }

    pub(super) fn advance_walk_distance(&mut self, travel: EntityVec3) {
        self.walk_dist_old = self.walk_dist;
        self.walk_dist += horizontal_distance(travel) * PLAYER_CLOAK_WALK_DISTANCE_SCALE;
    }

    pub(super) fn move_cloak(&mut self, position: EntityVec3) {
        let (x_old, x) = move_cloak_axis(self.cloak_position.x, position.x);
        let (y_old, y) = move_cloak_axis(self.cloak_position.y, position.y);
        let (z_old, z) = move_cloak_axis(self.cloak_position.z, position.z);
        self.cloak_position_old = EntityVec3 {
            x: x_old,
            y: y_old,
            z: z_old,
        };
        self.cloak_position = EntityVec3 { x, y, z };
    }

    pub(super) fn cape_state(self, partial_tick: f32, y_body_rot: f32) -> (f32, f32, f32) {
        let cloak = lerp_vec3(partial_tick, self.cloak_position_old, self.cloak_position);
        let entity = lerp_vec3(partial_tick, self.entity_position_old, self.entity_position);
        let delta_x = cloak.x - entity.x;
        let delta_y = cloak.y - entity.y;
        let delta_z = cloak.z - entity.z;
        let y_body_rot_rad = y_body_rot.to_radians();
        let forward_x = f64::from(y_body_rot_rad.sin());
        let forward_z = f64::from(-y_body_rot_rad.cos());

        let mut flap = (delta_y as f32 * 10.0).clamp(-6.0, 32.0);
        let fall_flying_scale =
            ((self.fall_flying_ticks as f32 + partial_tick).powi(2) / 100.0).clamp(0.0, 1.0);
        let lean = ((delta_x * forward_x + delta_z * forward_z) as f32
            * 100.0
            * (1.0 - fall_flying_scale))
            .clamp(0.0, 150.0);
        let lean2 = ((delta_x * forward_z - delta_z * forward_x) as f32 * 100.0).clamp(-20.0, 20.0);
        let bob = lerp_f32(partial_tick, self.bob_old, self.bob);
        let walk_dist = lerp_f32(partial_tick, self.walk_dist_old, self.walk_dist);
        flap += (walk_dist * 6.0).sin() * 32.0 * bob;

        (flap, lean, lean2)
    }

    pub(super) fn camera_view_bob(self, partial_tick: f32) -> EntityCameraViewBobState {
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        EntityCameraViewBobState {
            backwards_interpolated_walk_distance: -(self.walk_dist
                + (self.walk_dist - self.walk_dist_old) * partial_tick),
            bob: lerp_f32(partial_tick, self.bob_old, self.bob),
        }
    }
}

pub(super) fn move_cloak_axis(current_cloak: f64, position: f64) -> (f64, f64) {
    let delta = position - current_cloak;
    if !(-PLAYER_CLOAK_TELEPORT_THRESHOLD..=PLAYER_CLOAK_TELEPORT_THRESHOLD).contains(&delta) {
        (position, position)
    } else {
        (current_cloak, current_cloak + delta * PLAYER_CLOAK_EASE)
    }
}
