use super::*;

impl EntityClientAnimationState {
    /// Vanilla `Sheep.eatAnimationTick`, exposed for renderer head-pose
    /// projection. Returns `0` when the sheep is not currently eating.
    pub fn sheep_eat_animation_tick(&self) -> i32 {
        self.sheep_kind()
            .map_or(0, |state| state.eat_animation_tick)
    }

    /// Vanilla `EquineRenderState.animateTail` (`AbstractHorse.tailCounter > 0`), exposed for
    /// renderer tail yRot projection. Returns `false` for non-equine entities and idle equines.
    pub fn equine_animate_tail(&self) -> bool {
        self.equine_kind()
            .and_then(|k| k.tail)
            .is_some_and(EquineTailAnimationState::animate_tail)
    }

    /// Vanilla `EquineRenderState.eatAnimation` (`AbstractHorse.getEatAnim(partialTick)`), driven by
    /// the synced `AbstractHorse.DATA_ID_FLAGS` eating bit. Returns `0.0` for non-equines and idle
    /// equines.
    pub fn equine_eat_animation(&self, partial_tick: f32) -> f32 {
        self.equine_kind()
            .and_then(|k| k.pose)
            .map_or(0.0, |state| state.eat_animation(partial_tick))
    }

    /// Vanilla `EquineRenderState.standAnimation` (`AbstractHorse.getStandAnim(partialTick)`), driven
    /// by the synced standing bit. Returns `0.0` for non-equines and all-fours equines.
    pub fn equine_stand_animation(&self, partial_tick: f32) -> f32 {
        self.equine_kind()
            .and_then(|k| k.pose)
            .map_or(0.0, |state| state.stand_animation(partial_tick))
    }

    /// Vanilla `EquineRenderState.feedingAnimation` (`AbstractHorse.getMouthAnim(partialTick)`),
    /// driven by the synced open-mouth bit. Returns `0.0` for non-equines and closed-mouth equines.
    pub fn equine_feeding_animation(&self, partial_tick: f32) -> f32 {
        self.equine_kind()
            .and_then(|k| k.pose)
            .map_or(0.0, |state| state.mouth_animation(partial_tick))
    }

    /// Vanilla `Goat.lowerHeadTick` (the `0..=20` ram counter), exposed for the native layer to derive
    /// `getRammingXHeadRot` (which scales it by the adult/baby max head pitch). Returns `0` when the
    /// goat is not ramming. No partial-tick lerp: vanilla `getRammingXHeadRot` reads the raw int.
    pub fn goat_lower_head_tick(&self) -> i32 {
        self.goat_kind().map_or(0, |state| state.lower_head_tick)
    }

    /// Vanilla `IronGolemRenderState.attackTicksRemaining` (the partial-lerped `attackAnimationTick`),
    /// exposed for the renderer `IronGolemModel.setupAnim` smash arm wave. `0.0` when not attacking.
    pub fn iron_golem_attack_ticks_remaining(&self, partial_tick: f32) -> f32 {
        self.iron_golem_kind()
            .map_or(0.0, |state| state.attack_ticks_remaining(partial_tick))
    }

    /// Vanilla `IronGolemRenderState.offerFlowerTick` (the raw `offerFlowerTick`), exposed for the
    /// renderer offer-flower arm hold. `0` when the golem is not offering a poppy.
    pub fn iron_golem_offer_flower_tick(&self) -> i32 {
        self.iron_golem_kind()
            .map_or(0, |state| state.offer_flower_tick)
    }

    /// Vanilla `RavagerRenderState.stunnedTicksRemaining` (partial-lerped `stunnedTick`), exposed for the
    /// renderer `RavagerModel.setupAnim` head-shake stun pose. `0.0` when not stunned.
    pub fn ravager_stunned_ticks_remaining(&self, partial_tick: f32) -> f32 {
        self.ravager_kind()
            .map_or(0.0, |state| state.stunned_ticks_remaining(partial_tick))
    }

    /// Vanilla `RavagerRenderState.attackTicksRemaining` (partial-lerped `attackTick`), exposed for the
    /// renderer ravager neck-lunge / mouth-open bite pose. `0.0` when not attacking.
    pub fn ravager_attack_ticks_remaining(&self, partial_tick: f32) -> f32 {
        self.ravager_kind()
            .map_or(0.0, |state| state.attack_ticks_remaining(partial_tick))
    }

    /// Vanilla `RavagerRenderState.roarAnimation` (the `0..1` roar ramp), exposed for the renderer
    /// ravager mouth-open roar pose. `0.0` when not roaring.
    pub fn ravager_roar_animation(&self, partial_tick: f32) -> f32 {
        self.ravager_kind()
            .map_or(0.0, |state| state.roar_animation(partial_tick))
    }

    /// Vanilla `HoglinRenderState.attackAnimationRemainingTicks` (the RAW `attackAnimationRemainingTicks`,
    /// not partial-lerped), exposed for the renderer `HoglinModel.setupAnim` headbutt. `0` when the
    /// hoglin / zoglin is not mid-headbutt.
    pub fn hoglin_attack_animation_tick(&self) -> i32 {
        self.hoglin_kind()
            .map_or(0, |state| state.attack_animation_tick)
    }

    /// The rabbit hop's elapsed seconds since its `hopAnimationState` started (vanilla
    /// `Rabbit.hopAnimationState`, the 0.75s looping `RabbitAnimation.HOP`), projected for the
    /// renderer `RabbitModel.setupAnim` `hopAnimation.apply`. Returns `-1.0` (the stopped-animation
    /// sentinel) for a resting rabbit and every other entity; a non-negative value is wrapped by the
    /// looping def length each frame in the renderer.
    pub fn rabbit_hop_seconds(&self, partial_tick: f32) -> f32 {
        self.rabbit_kind()
            .and_then(|state| state.hop.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Bat.flyAnimationState` elapsed seconds, started by `Bat.setupAnimationStates` on the
    /// first flying client tick and stopped while the bat is resting. Returns `-1.0` before that
    /// first tick, while resting, and for every non-bat entity.
    pub fn bat_flying_seconds(&self, partial_tick: f32) -> f32 {
        self.bat_kind()
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The creaking attack's elapsed seconds since entity event `4` started it (vanilla
    /// `attackAnimationState`), projected for the renderer `CreakingModel.setupAnim`
    /// `attackAnimation.apply`. Returns `-1.0` (the stopped-animation sentinel) for a non-attacking
    /// creaking and every other entity; a non-negative value is wrapped by the 0.7083s looping length
    /// each frame in the renderer.
    pub fn creaking_attack_seconds(&self, partial_tick: f32) -> f32 {
        self.creaking_kind()
            .and_then(|state| state.attack.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The creaking invulnerable stagger's elapsed seconds since entity event `66` started it (vanilla
    /// `invulnerabilityAnimationState`), projected for the renderer `CreakingModel.setupAnim`
    /// `invulnerableAnimation.apply`. Returns `-1.0` (stopped) for a non-staggering creaking and every
    /// other entity; a non-negative value clamps past the 0.2917s length to its final frame.
    pub fn creaking_invulnerable_seconds(&self, partial_tick: f32) -> f32 {
        self.creaking_kind()
            .and_then(|state| state.invuln.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The creaking death collapse's elapsed seconds since `isTearingDown()` became true (vanilla
    /// `deathAnimationState`), projected for the renderer `CreakingModel.setupAnim`
    /// `deathAnimation.apply`. Returns `-1.0` (stopped) for a non-tearing-down creaking and every
    /// other entity; a non-negative value clamps past the 2.25s length to its final frame.
    pub fn creaking_death_seconds(&self, partial_tick: f32) -> f32 {
        self.creaking_kind()
            .and_then(|state| state.death.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Creaking.hasGlowingEyes()`, the client-ticked teardown flicker bit consumed by
    /// `CreakingRenderer` instead of synced `isActive()` while `isTearingDown()` holds. The exact
    /// process-local RNG seed is not on the wire; the state preserves Vanilla's Java RNG shape,
    /// inclusive delay ranges, death-time schedule, and toggle ordering with a stable local seed.
    pub fn creaking_eyes_glowing(&self) -> bool {
        self.creaking_kind()
            .is_some_and(CreakingAnimationState::eyes_glowing)
    }

    /// Vanilla `Warden.getTendrilAnimation(partialTick)`, exposed for the renderer
    /// `WardenModel.animateTendrils` sway. Returns `0.0` when the entity is not a
    /// warden with an active tendril pulse.
    pub fn warden_tendril_animation(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.tendril)
            .map_or(0.0, |state| state.tendril_animation(partial_tick))
    }

    /// Vanilla `Warden.getHeartAnimation(partialTick)`, exposed for the renderer
    /// warden heart emissive overlay's alpha. Returns `0.0` when the entity is not a
    /// warden (or before its first client tick spins up the heartbeat).
    pub fn warden_heart_animation(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.heart)
            .map_or(0.0, |state| state.heart_animation(partial_tick))
    }

    /// The warden roar's elapsed seconds since `Pose.ROARING` started (vanilla
    /// `roarAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for the renderer
    /// `WardenModel.setupAnim` `roarAnimation.apply`. Returns `-1.0` (the stopped-animation
    /// sentinel) for a non-roaring warden and every other entity, so the renderer applies no
    /// `WARDEN_ROAR` keyframe; a non-negative value is clamped past the 4.2s length to its final
    /// frame (vanilla's "hold the last frame").
    pub fn warden_roar_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.combat)
            .and_then(|state| state.roar.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden sniff's elapsed seconds since `Pose.SNIFFING` started (vanilla
    /// `sniffAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `sniffAnimation.apply`. Returns `-1.0` (stopped) for a non-sniffing warden and every other
    /// entity; a non-negative value clamps past the 4.16s length to its final frame.
    pub fn warden_sniff_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.combat)
            .and_then(|state| state.sniff.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden attack's elapsed seconds since entity event `4` started it (vanilla
    /// `attackAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `attackAnimation.apply`. Returns `-1.0` (stopped) for a non-attacking warden and every other
    /// entity; a non-negative value clamps past the 0.33333s length to its final frame.
    pub fn warden_attack_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.combat)
            .and_then(|state| state.attack.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden sonic boom's elapsed seconds since entity event `62` started it (vanilla
    /// `sonicBoomAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `sonicBoomAnimation.apply`. Returns `-1.0` (stopped) for a non-booming warden and every other
    /// entity; a non-negative value clamps past the 3.0s length to its final frame.
    pub fn warden_sonic_boom_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.combat)
            .and_then(|state| {
                state
                    .sonic_boom
                    .elapsed_seconds(self.age_ticks, partial_tick)
            })
            .unwrap_or(-1.0)
    }

    /// The warden emerge's elapsed seconds since `Pose.EMERGING` started (vanilla
    /// `emergeAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `emergeAnimation.apply`. Returns `-1.0` (stopped) for a non-emerging warden and every other
    /// entity; a non-negative value clamps past the 6.68s length to its final frame.
    pub fn warden_emerge_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.combat)
            .and_then(|state| state.emerge.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The warden dig's elapsed seconds since `Pose.DIGGING` started (vanilla
    /// `diggingAnimationState`), projected for the renderer `WardenModel.setupAnim`
    /// `diggingAnimation.apply`. Returns `-1.0` (stopped) for a non-digging warden and every other
    /// entity; a non-negative value clamps past the 5.0s length to its final frame.
    pub fn warden_dig_seconds(&self, partial_tick: f32) -> f32 {
        self.warden_kind()
            .and_then(|k| k.combat)
            .and_then(|state| state.dig.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Creeper.getSwelling(partialTick)`, exposed for the renderer white
    /// swelling overlay. Returns `0.0` when the entity is not a priming creeper.
    pub fn creeper_swelling(&self, partial_tick: f32) -> f32 {
        self.creeper_kind()
            .map_or(0.0, |state| state.swelling(partial_tick))
    }

    /// Vanilla `MinecartTntRenderState.fuseRemainingInTicks`: `MinecartTNT.getFuse() -
    /// partialTick + 1.0`, or `-1.0` while the cart is not primed. `TntMinecartRenderer` consumes
    /// this for the TNT display-block scale pulse and white overlay strobe.
    pub fn minecart_tnt_fuse_remaining_in_ticks(&self, partial_tick: f32) -> f32 {
        self.minecart_kind()
            .and_then(|k| k.tnt_fuse)
            .map_or(-1.0, |state| state.fuse_remaining_in_ticks(partial_tick))
    }

    pub fn minecart_spawner_spawn_delay(&self) -> i32 {
        self.minecart_kind()
            .and_then(|k| k.spawner)
            .map_or(20, |state| state.spawn_delay)
    }

    /// Vanilla `Shulker.getClientPeekAmount(partialTick)`, exposed for the renderer
    /// `ShulkerModel.setupAnim` lid open/close. Returns `0.0` (the closed/bind pose) when
    /// the entity is not a shulker or its lid is shut.
    pub fn shulker_peek_amount(&self, partial_tick: f32) -> f32 {
        self.shulker_kind()
            .and_then(|k| k.peek)
            .map_or(0.0, |state| state.peek_amount(partial_tick))
    }

    pub(crate) fn record_shulker_position_change(
        &mut self,
        old_position: EntityVec3,
        current_position: EntityVec3,
        is_passenger: bool,
    ) -> bool {
        if self.age_ticks == 0 || is_passenger {
            return false;
        }
        let old_block = entity_block_position(old_position);
        let current_block = entity_block_position(current_position);
        if old_block == current_block {
            return false;
        }
        let Some(shulker) = self.shulker_kind_mut() else {
            return false;
        };
        let state = shulker
            .teleport
            .get_or_insert(ShulkerTeleportAnimationState {
                old_attach_position: None,
                interpolation_ticks: 0,
            });
        if state.old_attach_position == Some(current_block) {
            return false;
        }
        state.old_attach_position = Some(old_block);
        state.interpolation_ticks = 6;
        true
    }

    pub(crate) fn shulker_started_riding(&mut self) {
        if let Some(slot) = self.shulker_kind_mut().map(|k| &mut k.teleport) {
            *slot = None;
        }
    }

    pub(crate) fn shulker_stopped_riding(&mut self, position: EntityVec3) {
        if let Some(slot) = self.shulker_kind_mut().map(|k| &mut k.teleport) {
            *slot = Some(ShulkerTeleportAnimationState {
                old_attach_position: Some(entity_block_position(position)),
                interpolation_ticks: 0,
            });
        }
    }

    pub(crate) fn shulker_render_offset(
        &self,
        current_position: EntityVec3,
        partial_tick: f32,
        scale: f32,
    ) -> Option<EntityVec3> {
        let state = self.shulker_kind().and_then(|k| k.teleport)?;
        let old = state.old_attach_position?;
        if state.interpolation_ticks == 0 {
            return None;
        }
        let current = entity_block_position(current_position);
        let mut progress = (f32::from(state.interpolation_ticks) - partial_tick) / 6.0;
        progress *= progress;
        let progress = f64::from(progress * scale);
        Some(EntityVec3 {
            x: -f64::from(current[0] - old[0]) * progress,
            y: -f64::from(current[1] - old[1]) * progress,
            z: -f64::from(current[2] - old[2]) * progress,
        })
    }

    /// Vanilla `Bee.getRollAmount(partialTick)` projected for the renderer
    /// `BeeModel.setupAnim` barrel-roll flip. Returns `0.0` when the entity is
    /// not a rolling bee.
    pub fn bee_roll_amount(&self, partial_tick: f32) -> f32 {
        self.bee_kind()
            .map_or(0.0, |state| state.roll_amount(partial_tick))
    }

    /// Vanilla `Panda.getSitAmount(partialTick)`, exposed for
    /// `PandaRenderState.sitAmount` and `PandaModel.setupAnim`. Returns `0.0`
    /// for a standing panda and every non-panda entity.
    pub fn panda_sit_amount(&self, partial_tick: f32) -> f32 {
        self.panda_kind()
            .map_or(0.0, |state| state.sit_amount(partial_tick))
    }

    /// Vanilla `Panda.getLieOnBackAmount(partialTick)`, exposed for
    /// `PandaRenderState.lieOnBackAmount` and `PandaModel.setupAnim`. Returns
    /// `0.0` for a panda not lying on its back and every non-panda entity.
    pub fn panda_lie_on_back_amount(&self, partial_tick: f32) -> f32 {
        self.panda_kind()
            .map_or(0.0, |state| state.lie_on_back_amount(partial_tick))
    }

    /// Vanilla `Panda.getRollAmount(partialTick)`, exposed for the adult panda
    /// model roll pose. The vanilla renderer zeroes this for baby pandas while
    /// still using [`Self::panda_roll_time`] for the whole-model tumble.
    pub fn panda_roll_amount(&self, partial_tick: f32) -> f32 {
        self.panda_kind()
            .map_or(0.0, |state| state.roll_amount(partial_tick))
    }

    /// Vanilla `PandaRenderState.rollTime = rollCounter > 0 ? rollCounter +
    /// partialTick : 0`, used by `PandaRenderer.setupRotations` for the
    /// whole-model tumble. Returns `0.0` outside an active roll.
    pub fn panda_roll_time(&self, partial_tick: f32) -> f32 {
        self.panda_kind()
            .map_or(0.0, |state| state.roll_time(partial_tick))
    }

    /// Vanilla `LivingEntity.getSwimAmount(partialTick)`: the 0..1 eased blend toward
    /// `isVisuallySwimming()`, used today by the drowned render path for the swim body
    /// pitch and limb overrides. Returns `0.0` for a living entity that is not swimming
    /// and for every non-living entity.
    pub fn living_swim_amount(&self, partial_tick: f32) -> f32 {
        self.living_swim
            .map_or(0.0, |state| state.swim_amount(partial_tick))
    }

    /// Vanilla `HumanoidRenderState.elytraRotX` =
    /// `LivingEntity.elytraAnimationState.getRotX(partialTick)`. A source row with
    /// no ticked state preserves the renderer's steady non-flying default.
    pub fn elytra_rot_x(&self, partial_tick: f32) -> f32 {
        self.elytra
            .map_or(ELYTRA_DEFAULT_X_ROT, |state| state.rot_x(partial_tick))
    }

    /// Vanilla `HumanoidRenderState.elytraRotY` =
    /// `LivingEntity.elytraAnimationState.getRotY(partialTick)`.
    pub fn elytra_rot_y(&self, partial_tick: f32) -> f32 {
        self.elytra
            .map_or(ELYTRA_DEFAULT_Y_ROT, |state| state.rot_y(partial_tick))
    }

    /// Vanilla `HumanoidRenderState.elytraRotZ` =
    /// `LivingEntity.elytraAnimationState.getRotZ(partialTick)`.
    pub fn elytra_rot_z(&self, partial_tick: f32) -> f32 {
        self.elytra
            .map_or(ELYTRA_DEFAULT_Z_ROT, |state| state.rot_z(partial_tick))
    }

    /// Vanilla `AvatarRenderer.extractCapeState`: the player cloak lag projected
    /// into `AvatarRenderState.capeFlap`, `capeLean`, and `capeLean2`.
    pub fn player_cape_state(&self, partial_tick: f32, y_body_rot: f32) -> (f32, f32, f32) {
        self.player_kind().map_or((0.0, 0.0, 0.0), |state| {
            state.cape_state(partial_tick, y_body_rot)
        })
    }

    /// Vanilla `ClientAvatarState` values copied into the active player
    /// camera's render state for `GameRenderer.bobView`.
    pub fn player_camera_view_bob(&self, partial_tick: f32) -> EntityCameraViewBobState {
        self.player_kind().map_or_else(Default::default, |state| {
            state.camera_view_bob(partial_tick)
        })
    }

    /// Vanilla `AvatarRenderState.fallFlyingTimeInTicks`.
    pub fn player_fall_flying_time(&self, partial_tick: f32) -> f32 {
        self.player_kind().map_or(partial_tick, |state| {
            state.fall_flying_ticks as f32 + partial_tick
        })
    }

    /// Vanilla `ClientAvatarState.deltaMovementOnPreviousTick`, used only for
    /// the avatar fall-flying yaw interpolation.
    pub fn player_previous_delta_movement(&self) -> EntityVec3 {
        self.player_kind().map_or(EntityVec3::default(), |state| {
            state.delta_movement_on_previous_tick
        })
    }

    /// The frog croak's elapsed seconds since `Pose.CROAKING` started (vanilla
    /// `croakAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for
    /// `FrogModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for a
    /// non-croaking frog and every other entity, so the renderer hides the
    /// `croaking_body` pouch and applies no `FROG_CROAK` keyframe; the renderer wraps
    /// a non-negative value by the 3.0s length before sampling.
    pub fn frog_croak_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_kind()
            .and_then(|k| k.croak)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The frog tongue's elapsed seconds since `Pose.USING_TONGUE` started (vanilla
    /// `tongueAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for
    /// `FrogModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for a frog that is not
    /// using its tongue and every other entity, so the renderer applies no `FROG_TONGUE` keyframe;
    /// the renderer wraps a non-negative value by the 0.5s length before sampling.
    pub fn frog_tongue_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_kind()
            .and_then(|k| k.tongue)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The frog jump's elapsed seconds since `Pose.LONG_JUMPING` started (vanilla
    /// `jumpAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected for
    /// `FrogModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for a
    /// non-jumping frog and every other entity, so the renderer applies no `FROG_JUMP`
    /// keyframe; a non-negative value is sampled against the 0.5s definition.
    pub fn frog_jump_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_kind()
            .and_then(|k| k.jump)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The frog swim-idle's elapsed seconds since it started (vanilla
    /// `swimIdleAnimationState`'s `getTimeInMillis`/`getElapsedSeconds`), projected
    /// for `FrogModel.setupAnim`. Unlike the pose-driven croak/jump, the swim-idle is
    /// driven each client tick from `isInWater() && !walkAnimation.isMoving()`.
    /// Returns `-1.0` (the stopped-animation sentinel) for a frog that is out of water
    /// or moving and for every other entity, so the renderer applies no
    /// `FROG_IDLE_WATER` keyframe; a non-negative value is sampled (looping) against
    /// the 3.0s definition.
    pub fn frog_swim_idle_seconds(&self, partial_tick: f32) -> f32 {
        self.frog_kind()
            .and_then(|k| k.swim_idle)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Camel.dashAnimationState` elapsed seconds (the looping `CAMEL_DASH` gallop), exposed
    /// for `CamelModel.setupAnim`. Returns `-1.0` (the stopped-animation sentinel) for every non-camel
    /// entity and a camel that is not dashing.
    pub fn camel_dash_seconds(&self, partial_tick: f32) -> f32 {
        self.camel_kind()
            .and_then(|k| k.dash)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Camel.idleAnimationState` elapsed seconds. `Camel.setupAnimationStates` restarts it
    /// every `random.nextInt(40) + 80` client ticks and never stops it, so a ticking camel always
    /// returns a non-negative sample while non-camels use the stopped-animation sentinel.
    pub fn camel_idle_seconds(&self, partial_tick: f32) -> f32 {
        self.camel_kind()
            .and_then(|k| k.idle)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.idleAnimationState` elapsed seconds. The client starts it only on the
    /// delayed `idleAnimationStartTick` while `COPPER_GOLEM_STATE == IDLE`; non-copper golems and
    /// interaction states return `-1.0` so the renderer applies no idle keyframe.
    pub fn copper_golem_idle_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_kind()
            .and_then(|k| k.idle)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionGetItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == GETTING_ITEM` and stopped by every other state.
    pub fn copper_golem_get_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_kind()
            .and_then(|k| k.get_item)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionGetNoItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == GETTING_NO_ITEM` and stopped by every other state.
    pub fn copper_golem_get_no_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_kind()
            .and_then(|k| k.get_no_item)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionDropItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == DROPPING_ITEM` and stopped by every other state.
    pub fn copper_golem_drop_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_kind()
            .and_then(|k| k.drop_item)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CopperGolem.interactionDropNoItemAnimationState` elapsed seconds, started while
    /// `COPPER_GOLEM_STATE == DROPPING_NO_ITEM` and stopped by every other state.
    pub fn copper_golem_drop_no_item_seconds(&self, partial_tick: f32) -> f32 {
        self.copper_golem_kind()
            .and_then(|k| k.drop_no_item)
            .and_then(|state| state.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `CamelRenderState.jumpCooldown` =
    /// `max(Camel.getJumpCooldown() - partialTicks, 0)`. The cooldown is seeded from the synced
    /// `DASH` rising edge and decremented each client tick, matching `Camel.tick`.
    pub fn camel_jump_cooldown(self, partial_tick: f32) -> f32 {
        self.camel_kind()
            .and_then(|k| k.dash_cooldown)
            .map(|state| state.jump_cooldown(partial_tick))
            .unwrap_or(0.0)
    }

    pub(crate) fn nautilus_jump_cooldown(self, partial_tick: f32) -> f32 {
        self.nautilus_kind()
            .and_then(|k| k.dash_cooldown)
            .map(|state| state.cooldown(partial_tick))
            .unwrap_or(0.0)
    }

    /// Vanilla `AllayRenderState.isDancing` (`Allay.isDancing()`): gates `AllayModel.setupAnim`'s dance
    /// branch (the body spin and head/body sway) against the plain head-look branch. `false` for every
    /// non-allay entity and an allay that is not dancing.
    pub fn allay_is_dancing(&self) -> bool {
        self.allay_kind()
            .and_then(|k| k.dance)
            .is_some_and(|state| state.is_dancing())
    }

    /// Vanilla `AllayRenderState.isSpinning` (`Allay.isSpinning()`): whether the allay is in the
    /// spinning phase of its dance (the body whirls `4π·spinningProgress`). `false` otherwise.
    pub fn allay_is_spinning(&self) -> bool {
        self.allay_kind()
            .and_then(|k| k.dance)
            .is_some_and(|state| state.is_dancing() && state.is_spinning())
    }

    /// Vanilla `AllayRenderState.spinningProgress` (`Allay.getSpinningProgress(partialTick)`): the
    /// `0..1` spin ramp blending the body spin in and out. `0.0` when the allay is not dancing.
    pub fn allay_spinning_progress(&self, partial_tick: f32) -> f32 {
        self.allay_kind()
            .and_then(|k| k.dance)
            .map_or(0.0, |state| state.spinning_progress(partial_tick))
    }

    /// Vanilla `AllayRenderState.holdingAnimationProgress`
    /// (`Allay.getHoldingItemAnimationProgress(partialTick)`): the `0..1` arm-raise
    /// blend driven by whether the synced main hand is non-empty.
    pub fn allay_holding_item_progress(&self, partial_tick: f32) -> f32 {
        self.allay_kind()
            .and_then(|k| k.holding_item)
            .map_or(0.0, |state| state.holding_item_progress(partial_tick))
    }

    /// Vanilla render-state `ticksUsingItem` (`getTicksUsingItem(partialTicks)`) reconstructed from the
    /// charge/use counter. Used by crossbow `CROSSBOW_CHARGE` poses and player spear kinetic use sway.
    /// `0.0` when the tracked charge/use bit is down.
    pub fn crossbow_charge_ticks_using_item(&self, partial_tick: f32) -> f32 {
        self.crossbow_charge
            .map_or(0.0, |state| state.ticks_using_item(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.playingDeadFactor` (`Axolotl.playingDeadAnimator.getFactor`): the
    /// `0..1` eased blend into `AdultAxolotlModel.setupPlayDeadAnimation`. `0.0` for an awake axolotl
    /// and every other entity.
    pub fn axolotl_playing_dead_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind()
            .map_or(0.0, |state| state.playing_dead_factor(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.inWaterFactor` (`Axolotl.inWaterAnimator.getFactor`): the `0..1`
    /// eased blend gating the swimming / water-hovering sub-animations. `0.0` for a grounded axolotl
    /// and every other entity.
    pub fn axolotl_in_water_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind()
            .map_or(0.0, |state| state.in_water_factor(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.onGroundFactor` (`Axolotl.onGroundAnimator.getFactor`): the `0..1`
    /// eased blend gating the ground-crawling / lay-still sub-animations. `0.0` for a swimming axolotl
    /// and every other entity.
    pub fn axolotl_on_ground_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind()
            .map_or(0.0, |state| state.on_ground_factor(partial_tick))
    }

    /// Vanilla `AxolotlRenderState.movingFactor` (`Axolotl.movingAnimator.getFactor`): the `0..1`
    /// eased blend separating the moving sub-animations (swim, crawl) from the still ones (hover,
    /// lay-still) and gating the mirror-leg copy. `0.0` for a still axolotl and every other entity.
    pub fn axolotl_moving_factor(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind()
            .map_or(0.0, |state| state.moving_factor(partial_tick))
    }

    /// Elapsed seconds for Vanilla's baby `swimAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_swim_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind().map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_swim,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `walkAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_walk_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind().map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_walk,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `idleUnderWaterAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_idle_underwater_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind().map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_idle_underwater,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `idleUnderWaterOnGroundAnimationState`, or `-1` while
    /// stopped.
    pub fn axolotl_baby_idle_underwater_on_ground_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind().map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_idle_underwater_on_ground,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `idleOnGroundAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_idle_on_ground_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind().map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_idle_on_ground,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// Elapsed seconds for Vanilla's baby `playDeadAnimationState`, or `-1` while stopped.
    pub fn axolotl_baby_play_dead_seconds(&self, partial_tick: f32) -> f32 {
        self.axolotl_kind().map_or(-1.0, |state| {
            AxolotlAnimationState::baby_animation_seconds(
                state.baby_play_dead,
                self.age_ticks,
                partial_tick,
            )
        })
    }

    /// The sniffer's active `Sniffer.State` animation (vanilla `Sniffer.onSyncedDataUpdated`'s
    /// one-shot `AnimationState`s) projected for `SnifferModel.setupAnim`: the `(state ordinal,
    /// elapsed seconds)` of the running triggered keyframe, or `(-1, -1.0)` when the sniffer is
    /// idling/searching or is any other entity. The renderer matches the id to pick the keyframe
    /// def (DIG/LONGSNIFF/STAND_UP/HAPPY/SNIFFSNIFF) and samples it at the elapsed seconds.
    pub fn sniffer_animation(&self, partial_tick: f32) -> (i32, f32) {
        self.sniffer_kind().map_or((-1, -1.0), |state| {
            state.animation(self.age_ticks, partial_tick)
        })
    }

    /// Vanilla `SnifferRenderState.isSearching` (`Sniffer.isSearching()`): whether the sniffer's
    /// synced `DATA_STATE` is `SEARCHING`, gating `SnifferModel.setupAnim`'s swap of the base walk for
    /// the looping `SNIFFER_SNIFF_SEARCH` search-walk. `false` for every other state and entity.
    pub fn sniffer_is_searching(&self) -> bool {
        self.sniffer_kind()
            .is_some_and(|state| state.is_searching())
    }

    /// Vanilla `Armadillo.shouldHideInShell()` projected for the renderer `isHidingInShell` shell-ball
    /// swap: `true` for the steady SCARED state and for the `inStateTicks`-gated ROLLING/UNROLLING
    /// windows (rolling hides after tick 5, unrolling un-hides at tick 26). `false` for an IDLE
    /// armadillo and every other entity (only the armadillo is given a roll animation state).
    pub fn armadillo_is_hiding_in_shell(&self) -> bool {
        self.armadillo_kind()
            .is_some_and(|state| state.is_hiding_in_shell(self.age_ticks))
    }

    /// The armadillo's `ARMADILLO_ROLL_UP` elapsed seconds (vanilla `rollUpAnimationState`, started
    /// on entry to ROLLING), projected for `ArmadilloModel.setupAnim`. Returns `-1.0` (the
    /// stopped-animation sentinel) when no roll-up is running and for every other entity; the
    /// renderer applies no `ARMADILLO_ROLL_UP` keyframe for a negative value.
    pub fn armadillo_roll_up_seconds(&self, partial_tick: f32) -> f32 {
        self.armadillo_kind()
            .and_then(|state| state.roll_up.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The armadillo's `ARMADILLO_ROLL_OUT` elapsed seconds (vanilla `rollOutAnimationState`, started
    /// on entry to UNROLLING), projected for `ArmadilloModel.setupAnim`. Returns `-1.0` when no
    /// roll-out is running and for every other entity.
    pub fn armadillo_roll_out_seconds(&self, partial_tick: f32) -> f32 {
        self.armadillo_kind()
            .and_then(|state| state.roll_out.elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// The armadillo's `ARMADILLO_PEEK` elapsed seconds (vanilla `peekAnimationState`), including the
    /// first SCARED setup tick's `fastForward(50, 1.0F)` baseline and entity event `64` restart. `-1.0`
    /// when no peek is running and for every other entity.
    pub fn armadillo_peek_seconds(&self, partial_tick: f32) -> f32 {
        self.armadillo_kind()
            .and_then(|state| state.peek_elapsed_seconds(self.age_ticks, partial_tick))
            .unwrap_or(-1.0)
    }

    /// Vanilla `Fox.getHeadRollAngle(partialTick)` projected for the renderer
    /// `FoxModel.setWalkingPose` head tilt. Returns `0.0` when the entity is not an
    /// interested fox.
    pub fn fox_head_roll_angle(&self, partial_tick: f32) -> f32 {
        self.fox_kind()
            .map_or(0.0, |state| state.head_roll_angle(partial_tick))
    }

    /// Vanilla `Fox.getCrouchAmount(partialTick)` projected for the renderer
    /// `FoxModel.setCrouchingPose` body drop. Returns `0.0` when the entity is not a
    /// crouching fox.
    pub fn fox_crouch_amount(&self, partial_tick: f32) -> f32 {
        self.fox_kind()
            .map_or(0.0, |state| state.crouch_amount(partial_tick))
    }

    /// Vanilla `Cat.getLieDownAmount(partialTick)` projected for
    /// `FelineRenderState.lieDownAmount`. Returns `0.0` for upright cats and every non-cat.
    pub fn feline_lie_down_amount(&self, partial_tick: f32) -> f32 {
        self.cat_kind()
            .map_or(0.0, |state| state.lie_down_amount(partial_tick))
    }

    /// Vanilla `Cat.getLieDownAmountTail(partialTick)` projected for the feline tail lie-down pose.
    /// Returns `0.0` for upright cats and every non-cat.
    pub fn feline_lie_down_amount_tail(&self, partial_tick: f32) -> f32 {
        self.cat_kind()
            .map_or(0.0, |state| state.lie_down_amount_tail(partial_tick))
    }

    /// Vanilla `Cat.getRelaxStateOneAmount(partialTick)` projected for the relaxed head pitch.
    /// Returns `0.0` for non-relaxing cats and every non-cat.
    pub fn feline_relax_state_one_amount(&self, partial_tick: f32) -> f32 {
        self.cat_kind()
            .map_or(0.0, |state| state.relax_state_one_amount(partial_tick))
    }

    /// Vanilla `WolfRenderState.wetShade` (`Wolf.getWetShade(partialTick)`): the base
    /// model tint multiplier used by `WolfRenderer.getModelTint`. Returns `1.0` when
    /// the wolf is dry, the `0.75` wet floor while submerged, and lerps back to white
    /// over the shake/drying timer. `1.0` for every non-wolf entity.
    pub fn wolf_wet_shade(&self, partial_tick: f32) -> f32 {
        self.wolf_kind()
            .and_then(|k| k.wet)
            .map_or(1.0, |state| state.wet_shade(partial_tick))
    }

    /// Vanilla `WolfRenderState.shakeAnim` (`Wolf.getShakeAnim(partialTick)`): the
    /// partial-lerped water-shake timer that `getBodyRollAngle(offset)` uses for
    /// the body / mane / tail roll. Returns `0.0` for a dry wolf and every non-wolf.
    pub fn wolf_shake_anim(&self, partial_tick: f32) -> f32 {
        self.wolf_kind()
            .and_then(|k| k.wet)
            .map_or(0.0, |state| state.shake_anim(partial_tick))
    }

    /// Vanilla `Wolf.die`: clear the client-only wet/shaking interpolation
    /// before delegating to `LivingEntity.die`.
    pub(crate) fn clear_wolf_wet_on_death(&mut self) {
        if let Some(slot) = self.wolf_kind_mut().map(|k| &mut k.wet) {
            *slot = None;
        }
    }

    /// Vanilla `WolfRenderState.headRollAngle` (`Wolf.getHeadRollAngle(partialTick)`):
    /// the lerped interested-angle accumulator scaled by `0.15π`. Returns `0.0`
    /// when the wolf is not begging/interested and every non-wolf entity.
    pub fn wolf_head_roll_angle(&self, partial_tick: f32) -> f32 {
        self.wolf_kind()
            .and_then(|k| k.head_roll)
            .map_or(0.0, |state| state.head_roll_angle(partial_tick))
    }

    /// Vanilla `ArrowRenderState.shake` =
    /// `AbstractArrow.shakeTime - partialTick`, consumed by `ArrowModel.setupAnim`
    /// for the impact wobble. `0.0` for arrows that are not currently shaking and
    /// every non-arrow entity.
    pub fn arrow_shake(&self, partial_tick: f32) -> f32 {
        self.arrow_kind()
            .map_or(0.0, |state| state.shake(partial_tick))
    }

    /// Vanilla `AbstractBoat.getRowingTime(side, partialTick)`: lerps the
    /// per-side paddle accumulator for a boat that still has an active paddle
    /// state and a controlling passenger. Returns `[left, right]`, or zeros for a
    /// boat at rest and every non-boat entity.
    pub fn boat_rowing_times(
        &self,
        partial_tick: f32,
        left_active: bool,
        right_active: bool,
    ) -> [f32; 2] {
        self.boat_kind()
            .and_then(|k| k.paddles)
            .map_or([0.0; 2], |state| {
                state.rowing_times(partial_tick, left_active, right_active)
            })
    }

    /// Vanilla `BoatRenderState.hurtTime/hurtDir/damageTime`: the client-decayed
    /// `VehicleEntity` damage-roll values, with `damageTime` clamped at zero after
    /// subtracting partial tick.
    pub fn boat_damage_state(&self, partial_tick: f32) -> (f32, i32, f32) {
        self.vehicle_damage()
            .map_or((0.0, 1, 0.0), |state| state.render_state(partial_tick))
    }

    /// Vanilla `BoatRenderState.bubbleAngle` (`AbstractBoat.getBubbleAngle(partialTick)`):
    /// the partial-lerped bubble-column wobble angle in degrees.
    pub fn boat_bubble_angle(&self, partial_tick: f32) -> f32 {
        self.boat_kind()
            .and_then(|k| k.bubble)
            .map_or(0.0, |state| state.angle(partial_tick))
    }

    /// Vanilla `WitherRenderState.xHeadRots/yHeadRots`: current side-head
    /// rotations copied from `WitherBoss.xRotHeads/yRotHeads`. The arrays are
    /// `[right_head, left_head]`; every non-wither keeps the zeroed default.
    pub fn wither_head_rotations(&self) -> ([f32; 2], [f32; 2]) {
        self.wither_kind()
            .map(|state| (state.x_head_rots, state.y_head_rots))
            .unwrap_or(([0.0; 2], [0.0; 2]))
    }
}
