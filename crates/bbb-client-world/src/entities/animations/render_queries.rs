use super::*;

impl EntityClientAnimationState {
    /// Vanilla `LivingEntityRenderer.extractRenderState`:
    /// `hasRedOverlay = hurtTime > 0 || deathTime > 0`.
    pub fn has_red_overlay(&self) -> bool {
        self.hurt.is_some_and(|state| state.hurt_time > 0)
            || self.death.is_some_and(|state| state.death_time > 0)
    }

    /// Vanilla `Camera.extractRenderState` hurt inputs for a living camera entity:
    /// `hurtTime - cameraEntityPartialTicks` and the ten-tick duration installed by
    /// `LivingEntity.animateHurt`. `None` means no hurt animation has been armed.
    pub fn camera_hurt_time(&self, partial_tick: f32) -> Option<(f32, f32)> {
        self.hurt.map(|state| {
            (
                state.hurt_time as f32 - partial_tick.clamp(0.0, 1.0),
                HURT_ANIMATION_DURATION as f32,
            )
        })
    }

    /// Vanilla `ArmorStandRenderState.wiggle`: `gameTime - lastHit + partialTick`.
    /// Values `>= ArmorStand.WOBBLE_TIME` render as rest.
    pub fn armor_stand_wiggle(&self, partial_tick: f32) -> f32 {
        self.armor_stand_kind()
            .map_or(ARMOR_STAND_WOBBLE_TIME as f32, |state| {
                state.elapsed_ticks as f32 + partial_tick
            })
    }

    /// Vanilla `LivingEntity.getTicksSinceLastKineticHitFeedback(partialTicks)`:
    /// returns `0.0` before any accepted feedback event, otherwise
    /// `gameTime - lastKineticHitFeedbackTime + partialTicks`.
    pub fn ticks_since_kinetic_hit_feedback(&self, partial_tick: f32) -> f32 {
        self.kinetic_hit_feedback
            .map_or(0.0, |state| state.elapsed_ticks as f32 + partial_tick)
    }

    pub(crate) fn can_trigger_kinetic_hit_feedback(&self) -> bool {
        self.kinetic_hit_feedback
            .is_none_or(|state| state.elapsed_ticks > KINETIC_HIT_FEEDBACK_RETRIGGER_TICKS)
    }

    /// Arms a melee swing, mirroring vanilla `LivingEntity.swing(hand)`: a swing only
    /// (re)starts when none is playing, the current one is past halfway, or the counter is
    /// the fresh `-1` — so a rapid re-swing during the first half is ignored (the in-flight
    /// swing keeps ramping). `off_hand` selects the left arm (`ClientboundAnimate` action `3`).
    pub(crate) fn trigger_swing(&mut self, off_hand: bool, duration: i32) {
        let duration = duration.max(1);
        let state = self.attack_swing.get_or_insert(AttackSwingAnimationState {
            swinging: false,
            swing_time: 0,
            duration,
            attack_anim: 0.0,
            prev_attack_anim: 0.0,
            off_hand,
        });
        if !state.swinging || state.swing_time >= duration / 2 || state.swing_time < 0 {
            state.swing_time = -1;
            state.swinging = true;
            state.duration = duration;
            state.off_hand = off_hand;
        }
    }

    pub(crate) fn active_swing_off_hand(&self) -> Option<bool> {
        self.attack_swing
            .as_ref()
            .filter(|state| state.swinging)
            .map(|state| state.off_hand)
    }

    pub(crate) fn refresh_swing_duration(&mut self, duration: i32) {
        if let Some(swing) = self.attack_swing.as_mut().filter(|state| state.swinging) {
            swing.duration = duration.max(1);
        }
    }

    /// Vanilla `LivingEntity.getAttackAnim` interpolates forward through the
    /// wrapped final tick so the arm completes its swing instead of reversing.
    pub fn attack_anim(&self, partial_tick: f32) -> f32 {
        self.attack_swing.map_or(0.0, |s| {
            let mut diff = s.attack_anim - s.prev_attack_anim;
            if diff < 0.0 {
                diff += 1.0;
            }
            s.prev_attack_anim + diff * partial_tick
        })
    }

    /// Whether the active swing is the off (left) hand, vanilla `swingingArm == OFF_HAND`.
    /// `false` for a main-hand swing and every non-swinging entity.
    pub fn attack_arm_off_hand(&self) -> bool {
        self.attack_swing.is_some_and(|s| s.off_hand)
    }

    /// Vanilla `LivingEntity.swinging`: whether a melee swing is currently in progress. Gates the player's
    /// `CROSSBOW_HOLD` arm pose (`AvatarRenderer.getArmPose`: `!swinging && crossbow && charged`), which
    /// yields to the swing while attacking. `false` for an entity that is not mid-swing.
    pub fn is_swinging(&self) -> bool {
        self.attack_swing.is_some_and(|s| s.swinging)
    }

    /// Vanilla `LivingEntityRenderState.deathTime`: `entity.deathTime > 0 ?
    /// entity.deathTime + partialTick : 0`, projected for the renderer death
    /// tip-over (`LivingEntityRenderer.setupRotations`). Returns `0.0` for a
    /// living entity that is not dying.
    pub fn death_time(&self, partial_tick: f32) -> f32 {
        self.death.map_or(0.0, |state| {
            if state.death_time > 0 {
                state.death_time as f32 + partial_tick
            } else {
                0.0
            }
        })
    }

    /// Vanilla `EnderDragonRenderState.deathTime`: `dragonDeathTime + partialTick` once the dragon's
    /// own death counter starts ticking, used by `EnderDragonRenderer` for the dissolve body and
    /// procedural death rays.
    pub fn ender_dragon_death_time(&self, partial_tick: f32) -> f32 {
        self.ender_dragon_kind()
            .and_then(|k| k.state)
            .map_or(0.0, |dragon| dragon.death_time(partial_tick))
    }

    /// Vanilla `EnderDragonRenderer.extractRenderState`'s procedural-animation inputs. The flight
    /// samples are pre-interpolated at `state.partialTicks`, which vanilla freezes to `0` for a
    /// dead-or-dying dragon; `distance_to_egg` needs the world heightmap probe and is filled by the
    /// store-and-light projection layer.
    pub(crate) fn ender_dragon_animation_source(
        &self,
        partial_ticks: f32,
    ) -> Option<EnderDragonAnimationRenderState> {
        let dragon = self.ender_dragon_kind().and_then(|k| k.state)?;
        let sample_partial = if dragon.dragon_dying {
            0.0
        } else {
            partial_ticks
        };
        let mut historical = [DragonFlightHistorySample::default(); 24];
        for (delay, slot) in historical.iter_mut().enumerate() {
            *slot = dragon
                .flight_history
                .get_interpolated(delay as i32, sample_partial);
        }
        Some(EnderDragonAnimationRenderState {
            flap_time: dragon.flap_time(partial_ticks),
            is_landing_or_taking_off: vanilla_ender_dragon_phase_is_landing_or_takeoff(
                dragon.phase_id,
            ),
            is_sitting: vanilla_ender_dragon_phase_is_sitting(dragon.phase_id),
            distance_to_egg: 0.0,
            historical,
        })
    }

    /// Vanilla `PolarBear.getStandingAnimationScale(partialTick)` projected for
    /// the renderer `PolarBearModel.setupAnim` standing pose. Returns `0.0` when
    /// the entity is not a rearing polar bear.
    pub fn polar_bear_stand_scale(&self, partial_tick: f32) -> f32 {
        self.polar_bear_kind()
            .map_or(0.0, |state| state.standing_animation_scale(partial_tick))
    }

    /// Vanilla `LivingEntityRenderState.walkAnimationPos`
    /// (`WalkAnimationState.position(partialTick)`): the lerped limb-swing position
    /// that sways the model's legs/arms. Returns `0.0` for an entity that has not
    /// been ticked as a walking living entity.
    pub fn walk_animation_position(&self, partial_tick: f32) -> f32 {
        self.walk_animation
            .map_or(0.0, |walk| walk.position(partial_tick))
    }

    /// Vanilla `LivingEntityRenderState.walkAnimationSpeed`
    /// (`WalkAnimationState.speed(partialTick)`): the lerped limb-swing speed
    /// amplitude. Returns `0.0` for an entity that is not walking.
    pub fn walk_animation_speed(&self, partial_tick: f32) -> f32 {
        self.walk_animation
            .map_or(0.0, |walk| walk.speed(partial_tick))
    }

    /// Vanilla `SquidRenderState.tentacleAngle` (`Squid.tentacleAngle` lerped):
    /// the tentacle flex angle `SquidModel.setupAnim` writes to every tentacle's
    /// `xRot`. Returns `0.0` for a non-squid entity (and a frozen/unticked squid).
    pub fn squid_tentacle_angle(&self, partial_tick: f32) -> f32 {
        self.squid_kind()
            .map_or(0.0, |squid| squid.tentacle_angle(partial_tick))
    }

    /// Vanilla `SquidRenderState.xBodyRot` (`Squid.xBodyRot` lerped, degrees): the
    /// swim pitch `SquidRenderer.setupRotations` applies to the squid root.
    /// Returns `0.0` for every non-squid entity.
    pub fn squid_x_body_rot(&self, partial_tick: f32) -> f32 {
        self.squid_kind()
            .map_or(0.0, |squid| squid.x_body_rot(partial_tick))
    }

    /// Vanilla `LivingEntityRenderState.bodyRot` for squid (`Squid.yBodyRot`
    /// lerped, degrees). Returns `None` for every non-squid entity so callers can
    /// keep using the synced transform yaw.
    pub fn squid_y_body_rot(&self, partial_tick: f32) -> Option<f32> {
        self.squid_kind()
            .map(|squid| squid.y_body_rot(partial_tick))
    }

    /// Vanilla `SquidRenderState.zBodyRot` (`Squid.zBodyRot` lerped, degrees): the
    /// swim roll `SquidRenderer.setupRotations` applies to the squid root. Returns
    /// `0.0` for every non-squid entity.
    pub fn squid_z_body_rot(&self, partial_tick: f32) -> f32 {
        self.squid_kind()
            .map_or(0.0, |squid| squid.z_body_rot(partial_tick))
    }

    /// Vanilla `ChickenRenderState.flap` (`Chicken.flap` lerped): the wing-flap
    /// phase `ChickenModel.setupAnim` feeds to `(sin(flap) + 1) * flapSpeed`.
    /// Returns `0.0` for every non-chicken entity (and an unticked chicken).
    pub fn chicken_flap(&self, partial_tick: f32) -> f32 {
        self.chicken_kind()
            .map_or(0.0, |flap| flap.flap(partial_tick))
    }

    /// Vanilla `SlimeRenderState.squish` (`Slime.squish` lerped): the squish amount
    /// `SlimeRenderer.scale` turns into the body's non-uniform stretch (`ss = squish
    /// / (size * 0.5 + 1)`, `w = 1 / (ss + 1)`, scale `[w, 1/w, w] * size`). Returns
    /// `0.0` for every non-slime/magma-cube entity (and an unticked one), so the body
    /// holds its undeformed cube shape.
    pub fn slime_squish(&self, partial_tick: f32) -> f32 {
        self.slime_kind()
            .and_then(|k| k.state)
            .map_or(0.0, |slime| slime.squish(partial_tick))
    }

    /// Vanilla `EvokerFangsRenderState.biteProgress` (`EvokerFangs.getAnimationProgress`):
    /// the `0..1` attack ramp `EvokerFangsModel.setupAnim` turns into the jaw snap, the
    /// rise out of the ground, and the final vanish. Returns `0.0` (the hidden,
    /// pre-attack fang) for every non-fang entity and a fang that has not yet started.
    pub fn evoker_fangs_bite_progress(&self, partial_tick: f32) -> f32 {
        self.evoker_fangs_kind()
            .map_or(0.0, |fangs| fangs.bite_progress(partial_tick))
    }

    /// Vanilla `ChickenRenderState.flapSpeed` (`Chicken.flapSpeed` lerped): the
    /// wing-flap amplitude `ChickenModel.setupAnim` multiplies the flap phase by.
    /// Returns `0.0` for every non-chicken entity, so the wings hold the bind pose.
    pub fn chicken_flap_speed(&self, partial_tick: f32) -> f32 {
        self.chicken_kind()
            .map_or(0.0, |flap| flap.flap_speed(partial_tick))
    }

    /// Vanilla `ParrotRenderState.flapAngle` (`ParrotRenderer.extractRenderState`):
    /// the lerped flap phase and speed combined into `(sin(flap) + 1) * flapSpeed`,
    /// which `ParrotModel.setupAnim` feeds to the wing `zRot` (`±(0.0873 +
    /// flapAngle)`) and the body/head/tail bob (`flapAngle * 0.3`). Returns `0.0` for
    /// every non-parrot entity (and an unticked parrot), so the wings hold the bind
    /// pose.
    pub fn parrot_flap_angle(&self, partial_tick: f32) -> f32 {
        self.parrot_kind()
            .map_or(0.0, |flap| flap.flap_angle(partial_tick))
    }

    /// Vanilla `GuardianRenderState.tailAnimation` (`Guardian.getTailAnimation`
    /// lerped): the tail-sway phase `GuardianModel.setupAnim` feeds to the three
    /// tail segments' `yRot`. Returns `0.0` for every non-guardian entity (and an
    /// unticked guardian), so the tail holds the bind pose.
    pub fn guardian_tail_animation(&self, partial_tick: f32) -> f32 {
        self.guardian_kind()
            .and_then(|k| k.tail)
            .map_or(0.0, |tail| tail.tail_animation(partial_tick))
    }

    /// Vanilla `GuardianRenderState.spikesAnimation` (`Guardian.getSpikesAnimation(partialTick)`,
    /// partial-lerped): the spike-withdrawal phase `GuardianModel.setupAnim` turns into
    /// `withdrawal = (1 - spikesAnimation) · 0.55`. Returns `1.0` (withdrawal `0`, the fully-extended
    /// rest pose) for every non-guardian entity and an unticked guardian.
    pub fn guardian_spikes_animation(&self, partial_tick: f32) -> f32 {
        self.guardian_kind()
            .and_then(|k| k.spikes)
            .map_or(1.0, |spikes| spikes.spikes_animation(partial_tick))
    }

    /// The breeze shoot's elapsed seconds since `Pose.SHOOTING` started (vanilla `Breeze.shoot`),
    /// projected for the renderer `BreezeModel.setupAnim` `shootAnimation.apply`. Returns `-1.0` (the
    /// stopped-animation sentinel) for a non-shooting breeze and every other entity; a non-negative
    /// value clamps past the 1.125s length to its final frame (the action one-shots do not loop).
    pub fn breeze_shoot_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze_kind()
            .and_then(|state| state.shoot.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The breeze slide's elapsed seconds since `Pose.SLIDING` started (vanilla `Breeze.slide`).
    /// Returns `-1.0` (stopped) for a non-sliding breeze and every other entity; clamps past 0.2s.
    pub fn breeze_slide_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze_kind()
            .and_then(|state| state.slide.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The breeze slideBack's elapsed seconds since it last LEFT `Pose.SLIDING` (vanilla
    /// `Breeze.slideBack`, the 0.1s return that ends at the neutral pose). Returns `-1.0` (stopped) for
    /// a breeze that has never slid and every other entity; clamps past 0.1s to its neutral final frame.
    pub fn breeze_slide_back_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze_kind()
            .and_then(|state| {
                state
                    .slide_back
                    .elapsed_seconds(self.age_ticks, partial_tick)
            })
            .unwrap_or(-1.0)
    }

    /// The breeze inhale's elapsed seconds since `Pose.INHALING` started (vanilla `Breeze.inhale`).
    /// Returns `-1.0` (stopped) for a non-inhaling breeze and every other entity; clamps past 2.0s.
    pub fn breeze_inhale_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze_kind()
            .and_then(|state| state.inhale.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The breeze long-jump's elapsed seconds since `Pose.LONG_JUMPING` started (vanilla
    /// `Breeze.longJump`). Returns `-1.0` (stopped) for a non-jumping breeze and every other entity;
    /// clamps past 0.5s.
    pub fn breeze_long_jump_seconds(&self, partial_tick: f32) -> f32 {
        self.breeze_kind()
            .and_then(|state| {
                state
                    .long_jump
                    .elapsed_seconds(self.age_ticks, partial_tick)
            })
            .unwrap_or(-1.0)
    }

    /// Advance the shared crossbow-draw counter (the client's `getTicksUsingItem` reconstruction that
    /// `animateCrossbowCharge` reads) for an entity whose synced charging flag is `is_charging`: count up
    /// while drawing, reset to `0` once it stops, and drop the state once it has fully settled back to rest.
    /// Shared by the pillager and the regular piglin (only their flag's metadata id differs).
    pub(super) fn advance_crossbow_charge(&mut self, is_charging: bool) {
        if is_charging || self.crossbow_charge.is_some() {
            let charge = self
                .crossbow_charge
                .get_or_insert_with(CrossbowChargeAnimationState::default);
            charge.advance_client_tick(is_charging);
            if charge.is_settled() {
                self.crossbow_charge = None;
            }
        }
    }
}
