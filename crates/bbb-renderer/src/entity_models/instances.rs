use super::catalog::*;
use super::SheepHeadEatPose;

/// Vanilla `LightCoordsUtil.FULL_BRIGHT` (`block 15 | sky 15`): the
/// `EntityRenderState`/`LivingEntityRenderState.lightCoords` default used until
/// the entity scene projects sampled block+sky light.
pub const ENTITY_FULL_BRIGHT_LIGHT_COORDS: u32 = 15_728_880;

/// Vanilla sleeping pose (`LivingEntityRenderer.setupRotations`/`submit` when
/// `state.hasPose(Pose.SLEEPING)`): the entity lies down in a bed. The renderer
/// skips the usual `180 - bodyRot` body yaw and instead applies `Ry(yaw_angle) *
/// Rz(getFlipDegrees) * Ry(270)`, plus a world-space bed head-offset translate
/// before the entity scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SleepingPose {
    /// Vanilla `setupRotations` sleeping `angle` (degrees): the bed-direction
    /// rotation `sleepDirectionToRotation(bedOrientation)`, or the (shaken) body
    /// yaw when the entity is not in a bed.
    pub yaw_angle: f32,
    /// Vanilla `submit` bed head-offset translate `[-stepX * headOffset, -stepZ *
    /// headOffset]` in world units, where `headOffset = eyeHeight(STANDING) - 0.1`.
    /// `[0, 0]` when the entity is not in a bed.
    pub bed_offset: [f32; 2],
}

/// Per-frame projection of the vanilla `LivingEntityRenderState` (and its
/// `EntityRenderState` base) fields that the renderer entity pass consumes.
///
/// Vanilla renders entities from a render-state snapshot extracted once per
/// frame in `EntityRenderer.extractRenderState`, not from the live entity. This
/// struct is the matching projection and the single landing spot for the
/// per-frame rotation, pose, and animation values shared across model families.
/// Pipeline work added later (block+sky `lightCoords`, hurt/white
/// `OverlayTexture`, `walkAnimationPos`/`walkAnimationSpeed` limb-swing, head
/// `yRot`/`xRot` look, `ageScale`) extends this one structure instead of growing
/// ad hoc fields on [`EntityModelInstance`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityRenderState {
    /// Vanilla `LivingEntityRenderState.bodyRot`: the interpolated body yaw, in
    /// degrees, that orients the model root transform. The entity scene folds the
    /// `LivingEntityRenderer.setupRotations` freezing body shake into this value.
    pub body_rot: f32,
    /// Vanilla `LivingEntityRenderState.yRot`: the net head yaw in degrees
    /// (`Mth.wrapDegrees(headRot - bodyRot)`), i.e. the head turn relative to the
    /// body that models apply as `head.yRot = yRot * π/180`. `0.0` when the head
    /// is aligned with the body. The entity scene projects it from the canonical
    /// head/body yaw.
    pub head_yaw: f32,
    /// Vanilla `LivingEntityRenderState.xRot`: the head pitch in degrees
    /// (`entity.getXRot`), applied as `head.xRot = xRot * π/180`. `0.0` when the
    /// head is level.
    pub head_pitch: f32,
    /// Per-frame sheep eat-grass head pose (`Sheep.getHeadEatPositionScale` /
    /// `getHeadEatAngleScale`). [`SheepHeadEatPose::NONE`] for every non-sheep
    /// entity and for a sheep that is not currently eating.
    pub head_eat: SheepHeadEatPose,
    /// Per-frame polar bear standing-rear scale
    /// (`PolarBear.getStandingAnimationScale`, `0.0..=1.0`). `0.0` for every
    /// other entity and for a polar bear on all fours.
    pub polar_bear_stand_scale: f32,
    /// Vanilla `LivingEntityRenderState.deathTime` (`entity.deathTime > 0 ?
    /// entity.deathTime + partialTick : 0`): the lerped death-animation counter
    /// that tips a dying living entity over in `LivingEntityRenderer.setupRotations`
    /// (`Axis.ZP.rotationDegrees(sqrt(min((deathTime-1)/20*1.6, 1)) *
    /// getFlipDegrees())`). `0.0` for every entity that is alive.
    pub death_time: f32,
    /// Vanilla `EntityRenderState.lightCoords` (`LightCoordsUtil.pack(block,
    /// sky)`): the packed block+sky light sampled at the entity's light-probe
    /// block position. Defaults to [`ENTITY_FULL_BRIGHT_LIGHT_COORDS`]; the
    /// entity scene projects the sampled value with the on-fire override.
    pub light_coords: u32,
    /// Vanilla `LivingEntityRenderState.hasRedOverlay` (`hurtTime > 0`): selects
    /// the red row of `OverlayTexture` so the entity flashes red when hurt.
    pub has_red_overlay: bool,
    /// Vanilla `CreeperRenderer.getWhiteOverlayProgress` (`0.0..=1.0`): selects
    /// the white-flash column of `OverlayTexture` so a priming creeper flashes
    /// white. `0.0` for every entity that is not flashing white.
    pub white_overlay_progress: f32,
    /// Vanilla `LivingEntityRenderState.isAutoSpinAttack` riptide spin: when the
    /// entity is mid-trident-spin, `Some(ageInTicks)` (the lerped
    /// `ageInTicks + partialTick`) drives the `LivingEntityRenderer.setupRotations`
    /// branch `Axis.XP.rotationDegrees(-90 - xRot)` then
    /// `Axis.YP.rotationDegrees(ageInTicks * -75)`. `None` for every entity that is
    /// not spinning (the death tip-over takes precedence over this branch).
    pub auto_spin_age_ticks: Option<f32>,
    /// Vanilla `LivingEntityRenderState.isUpsideDown` Dinnerbone/Grumm flip: when
    /// the entity is upside down, `Some(boundingBoxHeight)` drives the
    /// `LivingEntityRenderer.setupRotations` branch `translate(0, (bbHeight + 0.1) /
    /// entityScale, 0)` then `Axis.ZP.rotationDegrees(180)`. Carried as the world
    /// `boundingBoxHeight` because the post-yaw frame is already in world units (the
    /// model scale is applied innermost), so the `/ entityScale` is unnecessary.
    /// `None` for every entity that is not upside down (death and the riptide spin
    /// both take precedence over this branch).
    pub upside_down_height: Option<f32>,
    /// Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: when sleeping in a
    /// bed, the renderer skips the `180 - bodyRot` yaw and lays the model down via
    /// [`SleepingPose`]. `None` for every entity that is not sleeping. Death and
    /// the riptide spin take precedence over this branch; this branch takes
    /// precedence over the upside-down flip.
    pub sleeping: Option<SleepingPose>,
    /// Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the `SCALE`
    /// attribute): the uniform model scale `LivingEntityRenderer.submit` applies as
    /// `poseStack.scale(scale, scale, scale)` before `setupRotations`. `1.0` for an
    /// entity at its default size.
    pub scale: f32,
    /// Vanilla `LivingEntityRenderState.walkAnimationPos`
    /// (`WalkAnimationState.position(partialTick)`): the lerped limb-swing position
    /// that models feed into the `cos(animationPos * 0.6662 ...)` leg/arm sway in
    /// `setupAnim`. `0.0` for a standing entity.
    pub walk_animation_pos: f32,
    /// Vanilla `LivingEntityRenderState.walkAnimationSpeed`
    /// (`WalkAnimationState.speed(partialTick)`): the lerped limb-swing amplitude
    /// (`0.0..=1.0`) that scales the sway in `setupAnim`. `0.0` for a standing
    /// entity, leaving the model in its rest pose.
    pub walk_animation_speed: f32,
    /// Vanilla `EntityRenderState.ageInTicks` (`entity.tickCount + partialTick`): the
    /// lerped per-frame age that drives continuous idle animations (e.g. the
    /// `AbstractPiglinModel` ear flap). `0.0` until the entity scene projects it.
    pub age_in_ticks: f32,
}

impl EntityRenderState {
    /// Builds the resting render state for an entity facing `body_rot` degrees:
    /// head aligned with the body (no look), no eat-grass head pose, an all-fours
    /// polar bear stance, and full-bright light. Per-frame animation poses and
    /// sampled light are layered on by the entity scene projection.
    fn resting(body_rot: f32) -> Self {
        Self {
            body_rot,
            head_yaw: 0.0,
            head_pitch: 0.0,
            head_eat: SheepHeadEatPose::NONE,
            polar_bear_stand_scale: 0.0,
            death_time: 0.0,
            light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
            has_red_overlay: false,
            white_overlay_progress: 0.0,
            auto_spin_age_ticks: None,
            upside_down_height: None,
            sleeping: None,
            scale: 1.0,
            walk_animation_pos: 0.0,
            walk_animation_speed: 0.0,
            age_in_ticks: 0.0,
        }
    }

    /// Projects the packed light coords into the renderer per-vertex lightmap
    /// input `[block, sky]`, each normalized to `0.0..=1.0`, mirroring the
    /// terrain mesh's `[block/15, sky/15]` shader light.
    pub(in crate::entity_models) fn shader_light(&self) -> [f32; 2] {
        let block = (self.light_coords >> 4) & 0xF;
        let sky = (self.light_coords >> 20) & 0xF;
        [block as f32 / 15.0, sky as f32 / 15.0]
    }

    /// Projects the entity overlay into the renderer per-vertex overlay coords
    /// `[u, v]` (vanilla `OverlayTexture.pack` channels). `u` is the white-flash
    /// column `OverlayTexture.u(whiteOverlayProgress)` = `(int)(progress * 15)`,
    /// `v` is `RED_OVERLAY_V` (`3`) when hurt and `WHITE_OVERLAY_V` (`10`, no red
    /// overlay) otherwise.
    pub(in crate::entity_models) fn overlay_coords(&self) -> [f32; 2] {
        let u = (self.white_overlay_progress.clamp(0.0, 1.0) * 15.0).floor();
        [u, if self.has_red_overlay { 3.0 } else { 10.0 }]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityModelInstance {
    pub entity_id: i32,
    pub kind: EntityModelKind,
    pub position: [f32; 3],
    /// Per-frame render-state projection (body rotation + animation poses)
    /// consumed by the renderer entity pass.
    pub render_state: EntityRenderState,
}

impl EntityModelInstance {
    pub fn new(entity_id: i32, kind: EntityModelKind, position: [f32; 3], y_rot: f32) -> Self {
        Self {
            entity_id,
            kind,
            position,
            render_state: EntityRenderState::resting(y_rot),
        }
    }

    pub fn with_head_eat(mut self, head_eat: SheepHeadEatPose) -> Self {
        self.render_state.head_eat = head_eat;
        self
    }

    /// Sets the head-look projection (vanilla `LivingEntityRenderState.yRot` /
    /// `.xRot`, both in degrees): the net head yaw relative to the body and the
    /// head pitch. Consumed by model families with a head part (currently the
    /// sheep `QuadrupedModel`).
    pub fn with_head_look(mut self, head_yaw: f32, head_pitch: f32) -> Self {
        self.render_state.head_yaw = head_yaw;
        self.render_state.head_pitch = head_pitch;
        self
    }

    pub fn with_polar_bear_stand_scale(mut self, polar_bear_stand_scale: f32) -> Self {
        self.render_state.polar_bear_stand_scale = polar_bear_stand_scale;
        self
    }

    /// Sets the death-animation counter (vanilla `LivingEntityRenderState.deathTime`,
    /// the lerped `entity.deathTime + partialTick`). Drives the
    /// `LivingEntityRenderer.setupRotations` tip-over flip for a dying entity.
    pub fn with_death_time(mut self, death_time: f32) -> Self {
        self.render_state.death_time = death_time;
        self
    }

    pub fn with_light_coords(mut self, light_coords: u32) -> Self {
        self.render_state.light_coords = light_coords;
        self
    }

    pub fn with_has_red_overlay(mut self, has_red_overlay: bool) -> Self {
        self.render_state.has_red_overlay = has_red_overlay;
        self
    }

    pub fn with_white_overlay_progress(mut self, white_overlay_progress: f32) -> Self {
        self.render_state.white_overlay_progress = white_overlay_progress;
        self
    }

    /// Sets the riptide auto-spin projection (vanilla
    /// `LivingEntityRenderState.isAutoSpinAttack` plus the lerped `ageInTicks`).
    /// Drives the `LivingEntityRenderer.setupRotations` trident-spin branch.
    pub fn with_auto_spin_age_ticks(mut self, auto_spin_age_ticks: Option<f32>) -> Self {
        self.render_state.auto_spin_age_ticks = auto_spin_age_ticks;
        self
    }

    /// Sets the Dinnerbone/Grumm upside-down projection (vanilla
    /// `LivingEntityRenderState.isUpsideDown` plus `boundingBoxHeight`). Drives the
    /// `LivingEntityRenderer.setupRotations` upside-down branch.
    pub fn with_upside_down_height(mut self, upside_down_height: Option<f32>) -> Self {
        self.render_state.upside_down_height = upside_down_height;
        self
    }

    /// Sets the sleeping-in-bed projection (vanilla
    /// `LivingEntityRenderState.hasPose(Pose.SLEEPING)`). Drives the
    /// `LivingEntityRenderer.setupRotations`/`submit` sleeping branch.
    pub fn with_sleeping(mut self, sleeping: Option<SleepingPose>) -> Self {
        self.render_state.sleeping = sleeping;
        self
    }

    /// Sets the uniform model scale (vanilla `LivingEntityRenderState.scale`,
    /// `LivingEntity.getScale`). Drives the `LivingEntityRenderer.submit`
    /// `poseStack.scale` applied before `setupRotations`.
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.render_state.scale = scale;
        self
    }

    /// Sets the limb-swing projection (vanilla `LivingEntityRenderState.walkAnimationPos`
    /// / `.walkAnimationSpeed`). Consumed by model families with a walk cycle
    /// (currently the `QuadrupedModel` legs) to sway the limbs in `setupAnim`.
    pub fn with_walk_animation(
        mut self,
        walk_animation_pos: f32,
        walk_animation_speed: f32,
    ) -> Self {
        self.render_state.walk_animation_pos = walk_animation_pos;
        self.render_state.walk_animation_speed = walk_animation_speed;
        self
    }

    pub fn with_age_in_ticks(mut self, age_in_ticks: f32) -> Self {
        self.render_state.age_in_ticks = age_in_ticks;
        self
    }

    pub fn chicken(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::chicken_variant(
            entity_id,
            position,
            y_rot,
            ChickenModelVariant::Temperate,
            baby,
        )
    }

    pub fn chicken_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: ChickenModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Chicken { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn pig(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: PigModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Pig { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn player(entity_id: i32, position: [f32; 3], y_rot: f32, slim: bool) -> Self {
        Self::player_with_parts(
            entity_id,
            position,
            y_rot,
            slim,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
    }

    pub fn player_with_parts(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        slim: bool,
        parts: PlayerModelPartVisibility,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Player { slim, parts },
            position,
            y_rot,
        )
    }

    pub fn humanoid(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: HumanoidModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Humanoid { family, baby },
            position,
            y_rot,
        )
    }

    pub fn armor_stand(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        small: bool,
        show_arms: bool,
        show_base_plate: bool,
        pose: ArmorStandModelPose,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ArmorStand {
                small,
                show_arms,
                show_base_plate,
                pose,
            },
            position,
            y_rot,
        )
    }

    pub fn slime(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(entity_id, EntityModelKind::Slime { size }, position, y_rot)
    }

    pub fn magma_cube(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::MagmaCube { size },
            position,
            y_rot,
        )
    }

    pub fn zombie(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Zombie { baby }, position, y_rot)
    }

    pub fn zombie_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: ZombieVariantModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::ZombieVariant { family, baby },
            position,
            y_rot,
        )
    }

    pub fn piglin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: PiglinModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Piglin { family, baby },
            position,
            y_rot,
        )
    }

    pub fn hoglin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: HoglinModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Hoglin { family, baby },
            position,
            y_rot,
        )
    }

    pub fn ravager(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Ravager, position, y_rot)
    }

    pub fn boat(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: BoatModelFamily,
        chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Boat { family, chest },
            position,
            y_rot,
        )
    }

    pub fn skeleton(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Skeleton, position, y_rot)
    }

    pub fn skeleton_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: SkeletonModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::SkeletonVariant { family },
            position,
            y_rot,
        )
    }

    pub fn cow(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::cow_variant(entity_id, position, y_rot, CowModelVariant::Temperate, baby)
    }

    pub fn cow_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: CowModelVariant,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Cow { variant, baby },
            position,
            y_rot,
        )
    }

    pub fn sheep(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::sheep_wool(
            entity_id,
            position,
            y_rot,
            baby,
            false,
            SheepWoolColor::White,
        )
    }

    pub fn sheep_wool(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
    ) -> Self {
        Self::sheep_render_state(
            entity_id, position, y_rot, baby, sheared, wool_color, false, false, 0.0,
        )
    }

    pub fn sheep_render_state(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
        invisible: bool,
        jeb: bool,
        age_ticks: f32,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Sheep {
                baby,
                sheared,
                wool_color,
                invisible,
                jeb,
                age_ticks,
            },
            position,
            y_rot,
        )
    }

    #[cfg(test)]
    pub fn sheep_eating(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        sheared: bool,
        wool_color: SheepWoolColor,
        eat_animation_tick: i32,
        partial_tick: f32,
    ) -> Self {
        Self::sheep_wool(entity_id, position, y_rot, baby, sheared, wool_color).with_head_eat(
            SheepHeadEatPose::from_eat_tick(eat_animation_tick, partial_tick),
        )
    }

    pub fn villager(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Villager { baby },
            position,
            y_rot,
        )
    }

    pub fn wandering_trader(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::WanderingTrader, position, y_rot)
    }

    pub fn wolf(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::wolf_state(entity_id, position, y_rot, baby, false, false, false, None)
    }

    pub fn wolf_state(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        tame: bool,
        angry: bool,
        invisible: bool,
        collar_color: Option<EntityDyeColor>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Wolf {
                baby,
                tame,
                angry,
                invisible,
                collar_color: tame.then_some(collar_color).flatten(),
            },
            position,
            y_rot,
        )
    }

    pub fn horse(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Horse { baby }, position, y_rot)
    }

    pub fn donkey(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: DonkeyModelFamily,
        baby: bool,
        has_chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Donkey {
                family,
                baby,
                has_chest,
            },
            position,
            y_rot,
        )
    }

    pub fn undead_horse(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: UndeadHorseModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::UndeadHorse { family, baby },
            position,
            y_rot,
        )
    }

    pub fn camel(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: CamelModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Camel { family, baby },
            position,
            y_rot,
        )
    }

    pub fn llama(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: LlamaModelFamily,
        variant: LlamaVariant,
        baby: bool,
        has_chest: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Llama {
                family,
                variant,
                baby,
                has_chest,
            },
            position,
            y_rot,
        )
    }

    pub fn goat(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        left_horn: bool,
        right_horn: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Goat {
                baby,
                left_horn,
                right_horn,
            },
            position,
            y_rot,
        )
    }

    pub fn polar_bear(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::PolarBear { baby },
            position,
            y_rot,
        )
    }

    #[cfg(test)]
    pub fn polar_bear_standing(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        stand_scale: f32,
    ) -> Self {
        Self::polar_bear(entity_id, position, y_rot, baby).with_polar_bear_stand_scale(stand_scale)
    }

    pub fn spider(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Spider, position, y_rot)
    }

    pub fn cave_spider(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::CaveSpider, position, y_rot)
    }

    pub fn enderman(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Enderman, position, y_rot)
    }

    pub fn iron_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::IronGolem, position, y_rot)
    }

    pub fn snow_golem(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::SnowGolem, position, y_rot)
    }

    pub fn witch(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Witch, position, y_rot)
    }

    pub fn illager(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: IllagerModelFamily,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Illager { family },
            position,
            y_rot,
        )
    }

    pub fn quadruped(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        family: QuadrupedModelFamily,
        baby: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Quadruped { family, baby },
            position,
            y_rot,
        )
    }

    pub fn placeholder(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        name: &'static str,
        width: f32,
        height: f32,
        depth: f32,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Placeholder {
                name,
                bounds: EntityModelBounds {
                    width,
                    height,
                    depth,
                },
            },
            position,
            y_rot,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_model_instance_constructors_project_render_state() {
        assert_eq!(
            EntityModelInstance::chicken(10, [1.0, 2.0, 3.0], 45.0, true),
            EntityModelInstance::new(
                10,
                EntityModelKind::Chicken {
                    variant: ChickenModelVariant::Temperate,
                    baby: true,
                },
                [1.0, 2.0, 3.0],
                45.0,
            )
        );

        let wild = EntityModelInstance::wolf_state(
            11,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            false,
            false,
            false,
            Some(EntityDyeColor::Blue),
        );
        assert_eq!(
            wild.kind,
            EntityModelKind::Wolf {
                baby: false,
                tame: false,
                angry: false,
                invisible: false,
                collar_color: None,
            }
        );

        let placeholder = EntityModelInstance::placeholder(
            12,
            [4.0, 5.0, 6.0],
            90.0,
            "custom_bounds",
            1.0,
            2.0,
            3.0,
        );
        assert_eq!(placeholder.entity_id, 12);
        assert_eq!(placeholder.position, [4.0, 5.0, 6.0]);
        assert_eq!(placeholder.render_state.body_rot, 90.0);
        assert_eq!(
            placeholder.kind,
            EntityModelKind::Placeholder {
                name: "custom_bounds",
                bounds: EntityModelBounds {
                    width: 1.0,
                    height: 2.0,
                    depth: 3.0,
                },
            }
        );
    }

    #[test]
    fn new_projects_resting_render_state() {
        let instance = EntityModelInstance::new(
            7,
            EntityModelKind::Quadruped {
                family: QuadrupedModelFamily::Pig,
                baby: false,
            },
            [0.0, 0.0, 0.0],
            123.0,
        );
        assert_eq!(
            instance.render_state,
            EntityRenderState {
                body_rot: 123.0,
                head_yaw: 0.0,
                head_pitch: 0.0,
                head_eat: SheepHeadEatPose::NONE,
                polar_bear_stand_scale: 0.0,
                death_time: 0.0,
                light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
                has_red_overlay: false,
                white_overlay_progress: 0.0,
                auto_spin_age_ticks: None,
                upside_down_height: None,
                sleeping: None,
                scale: 1.0,
                walk_animation_pos: 0.0,
                walk_animation_speed: 0.0,
                age_in_ticks: 0.0,
            }
        );
    }

    #[test]
    fn overlay_coords_select_vanilla_red_row_when_hurt() {
        let calm = EntityModelInstance::zombie(1, [0.0, 0.0, 0.0], 0.0, false);
        // NO_WHITE_U = 0, WHITE_OVERLAY_V = 10 (no overlay).
        assert_eq!(calm.render_state.overlay_coords(), [0.0, 10.0]);

        let hurt = calm.with_has_red_overlay(true);
        // NO_WHITE_U = 0, RED_OVERLAY_V = 3.
        assert_eq!(hurt.render_state.overlay_coords(), [0.0, 3.0]);

        // White swelling overlay drives the u column: u = (int)(progress * 15).
        let swelling = calm.with_white_overlay_progress(0.8);
        assert_eq!(swelling.render_state.overlay_coords(), [12.0, 10.0]);
        // Red overlay still wins the v row when both are active.
        let both = swelling.with_has_red_overlay(true);
        assert_eq!(both.render_state.overlay_coords(), [12.0, 3.0]);
    }

    #[test]
    fn shader_light_normalizes_packed_block_and_sky() {
        // Full bright packs block 15, sky 15 -> [1.0, 1.0].
        let bright = EntityModelInstance::sheep(1, [0.0, 0.0, 0.0], 0.0, false);
        assert_eq!(bright.render_state.shader_light(), [1.0, 1.0]);

        // pack(block 7, sky 0) = 7 << 4 = 112; pack(block 0, sky 15) = 15 << 20.
        let block_only = bright.with_light_coords(7 << 4);
        assert_eq!(block_only.render_state.shader_light(), [7.0 / 15.0, 0.0]);
        let sky_only =
            EntityModelInstance::sheep(2, [0.0, 0.0, 0.0], 0.0, false).with_light_coords(15 << 20);
        assert_eq!(sky_only.render_state.shader_light(), [0.0, 1.0]);
    }

    #[test]
    fn builders_set_only_their_render_state_field() {
        let base = EntityModelInstance::sheep(1, [0.0, 0.0, 0.0], 45.0, false);
        assert_eq!(base.render_state.body_rot, 45.0);
        assert_eq!(base.render_state.head_yaw, 0.0);
        assert_eq!(base.render_state.head_pitch, 0.0);
        assert_eq!(base.render_state.head_eat, SheepHeadEatPose::NONE);
        assert_eq!(base.render_state.polar_bear_stand_scale, 0.0);

        let eating = base.with_head_eat(SheepHeadEatPose::from_eat_tick(40, 0.5));
        assert_eq!(eating.render_state.body_rot, 45.0);
        assert_eq!(
            eating.render_state.head_eat,
            SheepHeadEatPose::from_eat_tick(40, 0.5)
        );
        assert_eq!(eating.render_state.polar_bear_stand_scale, 0.0);
        // The eat builder leaves the head-look projection untouched.
        assert_eq!(eating.render_state.head_yaw, 0.0);
        assert_eq!(eating.render_state.head_pitch, 0.0);

        let looking = base.with_head_look(30.0, -12.5);
        assert_eq!(looking.render_state.head_yaw, 30.0);
        assert_eq!(looking.render_state.head_pitch, -12.5);
        // The look builder leaves body rotation and the eat pose untouched.
        assert_eq!(looking.render_state.body_rot, 45.0);
        assert_eq!(looking.render_state.head_eat, SheepHeadEatPose::NONE);

        let bear = EntityModelInstance::polar_bear(2, [0.0, 0.0, 0.0], 0.0, false)
            .with_polar_bear_stand_scale(0.5);
        assert_eq!(bear.render_state.head_eat, SheepHeadEatPose::NONE);
        assert_eq!(bear.render_state.polar_bear_stand_scale, 0.5);
    }
}
