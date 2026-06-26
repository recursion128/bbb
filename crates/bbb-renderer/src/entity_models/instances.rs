use super::catalog::*;
use super::model_layers::{ELYTRA_DEFAULT_X_ROT, ELYTRA_DEFAULT_Y_ROT, ELYTRA_DEFAULT_Z_ROT};
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

/// Generates the [`EntityRenderState`] struct, its `defaults()`/`resting()`
/// constructors, and the single-field `with_*` builders on
/// [`EntityModelInstance`] from one per-field declaration, so adding an
/// animation field is a single line instead of edits at three sites (struct
/// field, `resting()` default, builder).
///
/// Each entry is `$(#[doc...])* (with_$name) $name: $ty = $default;` and gets a
/// `pub` struct field (with its doc comments forwarded verbatim), a default in
/// `defaults()`, and a `pub fn with_$name(self, $name: $ty) -> Self` setter
/// that assigns `self.render_state.$name`. The builder name is spelled out in
/// the leading parens (always `with_<name>`) because declarative macros cannot
/// synthesize the `with_` identifier without a proc-macro helper. Write empty
/// parens `()` instead to skip the setter for fields driven by `resting`'s
/// parameter or by a hand-written multi-field convenience builder (e.g.
/// `with_head_look`).
macro_rules! entity_render_state {
    (
        $(
            $(#[$meta:meta])*
            ($($with:ident)?) $name:ident : $ty:ty = $default:expr
        );* $(;)?
    ) => {
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
            $(
                $(#[$meta])*
                pub $name: $ty,
            )*
        }

        impl EntityRenderState {
            /// The resting per-field defaults shared by every entity before the
            /// scene projects per-frame poses and sampled light. [`resting`]
            /// overrides only `body_rot`.
            ///
            /// [`resting`]: Self::resting
            fn defaults() -> Self {
                Self {
                    $($name: $default,)*
                }
            }

            /// Builds the resting render state for an entity facing `body_rot`
            /// degrees: head aligned with the body (no look), no eat-grass head
            /// pose, an all-fours polar bear stance, and full-bright light.
            /// Per-frame animation poses and sampled light are layered on by the
            /// entity scene projection.
            fn resting(body_rot: f32) -> Self {
                Self {
                    body_rot,
                    ..Self::defaults()
                }
            }
        }

        impl EntityModelInstance {
            $(
                $(
                    /// Sets the matching [`EntityRenderState`] field (single-field
                    /// render-state builder; see the field for the vanilla reference).
                    pub fn $with(mut self, $name: $ty) -> Self {
                        self.render_state.$name = $name;
                        self
                    }
                )?
            )*
        }
    };
}

entity_render_state! {
    /// Vanilla `LivingEntityRenderState.bodyRot`: the interpolated body yaw, in
    /// degrees, that orients the model root transform. The entity scene folds the
    /// `LivingEntityRenderer.setupRotations` freezing body shake into this value.
    () body_rot: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.yRot`: the net head yaw in degrees
    /// (`Mth.wrapDegrees(headRot - bodyRot)`), i.e. the head turn relative to the
    /// body that models apply as `head.yRot = yRot * π/180`. `0.0` when the head
    /// is aligned with the body. The entity scene projects it from the canonical
    /// head/body yaw.
    () head_yaw: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.xRot`: the head pitch in degrees
    /// (`entity.getXRot`), applied as `head.xRot = xRot * π/180`. `0.0` when the
    /// head is level.
    () head_pitch: f32 = 0.0;
    /// Vanilla `ArrowRenderState.shake` (`AbstractArrow.shakeTime - partialTick`):
    /// the impact wobble amount that `ArrowModel.setupAnim` converts into a root
    /// z-rotation. `0.0` for arrows that are not shaking and every non-arrow entity.
    (with_arrow_shake) arrow_shake: f32 = 0.0;
    /// Per-frame sheep eat-grass head pose (`Sheep.getHeadEatPositionScale` /
    /// `getHeadEatAngleScale`). [`SheepHeadEatPose::NONE`] for every non-sheep
    /// entity and for a sheep that is not currently eating.
    (with_head_eat) head_eat: SheepHeadEatPose = SheepHeadEatPose::NONE;
    /// Per-frame polar bear standing-rear scale
    /// (`PolarBear.getStandingAnimationScale`, `0.0..=1.0`). `0.0` for every
    /// other entity and for a polar bear on all fours.
    (with_polar_bear_stand_scale) polar_bear_stand_scale: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.deathTime` (`entity.deathTime > 0 ?
    /// entity.deathTime + partialTick : 0`): the lerped death-animation counter
    /// that tips a dying living entity over in `LivingEntityRenderer.setupRotations`
    /// (`Axis.ZP.rotationDegrees(sqrt(min((deathTime-1)/20*1.6, 1)) *
    /// getFlipDegrees())`). `0.0` for every entity that is alive.
    (with_death_time) death_time: f32 = 0.0;
    /// Vanilla `EntityRenderState.lightCoords` (`LightCoordsUtil.pack(block,
    /// sky)`): the packed block+sky light sampled at the entity's light-probe
    /// block position. Defaults to [`ENTITY_FULL_BRIGHT_LIGHT_COORDS`]; the
    /// entity scene projects the sampled value with the on-fire override.
    (with_light_coords) light_coords: u32 = ENTITY_FULL_BRIGHT_LIGHT_COORDS;
    /// Vanilla `LivingEntityRenderState.hasRedOverlay` (`hurtTime > 0`): selects
    /// the red row of `OverlayTexture` so the entity flashes red when hurt.
    (with_has_red_overlay) has_red_overlay: bool = false;
    /// Vanilla `CreeperRenderer.getWhiteOverlayProgress` (`0.0..=1.0`): selects
    /// the white-flash column of `OverlayTexture` so a priming creeper flashes
    /// white. `0.0` for every entity that is not flashing white.
    (with_white_overlay_progress) white_overlay_progress: f32 = 0.0;
    /// Vanilla `CreeperRenderState.swelling` (`Creeper.getSwelling`, lerped): the raw
    /// fuse progress that `CreeperRenderer.scale` inflates the model by while a creeper
    /// primes to explode. `0.0` for every non-creeper entity and a creeper at rest, where
    /// the swell scale is the identity.
    (with_creeper_swelling) creeper_swelling: f32 = 0.0;
    /// Vanilla `CreeperRenderState.isPowered` (`Creeper.isPowered`, the synced `DATA_IS_POWERED`): a
    /// charged creeper (struck by lightning) wears the scrolling `CreeperPowerLayer` energy swirl over
    /// its inflated armor model. `false` for every other entity and an uncharged creeper.
    (with_creeper_powered) creeper_powered: bool = false;
    /// Vanilla `VillagerDataHolderRenderState.getVillagerData()`: the villager biome type,
    /// profession, and trade level that `VillagerProfessionLayer` uses to render the type robe,
    /// profession robe/hat, and level badge overlays. Defaults to plains/none/1 for non-villagers and
    /// for villager data that has not arrived yet.
    (with_villager_model_data) villager_model_data: VillagerModelData = VillagerModelData::DEFAULT;
    /// Vanilla `ShulkerRenderState.peekAmount` (`Shulker.getClientPeekAmount`, lerped): the
    /// client peek that `ShulkerModel.setupAnim` opens the lid by — `lid.y = 16 + sin((0.5 +
    /// peek)·π)·8` (plus an `ageInTicks` bob above `0.5`) and a `lid.yRot` twist above `0.3`.
    /// `0.0` (closed/bind pose, `lid.y = 24`) for every non-shulker and a shut shulker.
    (with_shulker_peek) shulker_peek: f32 = 0.0;
    /// Vanilla `ShulkerRenderState.attachFace`: the block face the shulker is attached to. The
    /// renderer rotates the whole shulker around `(0, 0.5, 0)` by
    /// `attachFace.getOpposite().getRotation()`. `Down` is the vanilla default and leaves the floor
    /// shulker in its normal rest orientation.
    (with_shulker_attach_face) shulker_attach_face: EntityAttachmentFace = EntityAttachmentFace::Down;
    /// Vanilla `WardenRenderState.tendrilAnimation` (`Warden.getTendrilAnimation`, lerped): the
    /// `0..=1` tendril pulse that `WardenModel.animateTendrils` swings the two antennae by —
    /// `leftTendril.xRot = tendrilAnimation · cos(ageInTicks · 2.25) · π · 0.1`, the right negated.
    /// `0.0` (bind pose, antennae still) for every non-warden and a warden at rest.
    (with_tendril_animation) tendril_animation: f32 = 0.0;
    /// Vanilla `WardenRenderState.heartAnimation` (`Warden.getHeartAnimation`, lerped): the `0..=1`
    /// heartbeat pulse that scales the warden heart emissive overlay's alpha. `0.0` (heart dark) for
    /// every non-warden and between a warden's heartbeats.
    (with_heart_animation) heart_animation: f32 = 0.0;
    /// Vanilla `Warden.roarAnimationState` elapsed seconds (`Pose.ROARING`-driven, the 4.2s
    /// `WARDEN_ROAR`): `WardenModel.setupAnim` applies `roarAnimation.apply(roarAnimationState,
    /// ageInTicks)`, which the renderer mirrors by sampling `WARDEN_ROAR` at these seconds when
    /// `>= 0`. `-1.0` (the stopped-animation sentinel) for every other entity and a non-roaring
    /// warden, so no roar keyframe is applied.
    (with_warden_roar_seconds) warden_roar_seconds: f32 = -1.0;
    /// Vanilla `Warden.sniffAnimationState` elapsed seconds (`Pose.SNIFFING`-driven, the 4.16s
    /// `WARDEN_SNIFF`), sampled by `WardenModel.setupAnim`. `-1.0` (stopped) for every other
    /// entity and a non-sniffing warden.
    (with_warden_sniff_seconds) warden_sniff_seconds: f32 = -1.0;
    /// Vanilla `Warden.attackAnimationState` elapsed seconds (entity event `4`, the 0.33333s
    /// `WARDEN_ATTACK`), sampled by `WardenModel.setupAnim`. `-1.0` (stopped) for every other
    /// entity and a non-attacking warden.
    (with_warden_attack_seconds) warden_attack_seconds: f32 = -1.0;
    /// Vanilla `Warden.sonicBoomAnimationState` elapsed seconds (entity event `62`, the 3.0s
    /// `WARDEN_SONIC_BOOM`), sampled by `WardenModel.setupAnim`. `-1.0` (stopped) for every other
    /// entity and a non-booming warden.
    (with_warden_sonic_boom_seconds) warden_sonic_boom_seconds: f32 = -1.0;
    /// Vanilla `Warden.emergeAnimationState` elapsed seconds (`Pose.EMERGING`-driven, the 6.68s
    /// `WARDEN_EMERGE` spawn rise), sampled by `WardenModel.setupAnim`. `-1.0` (stopped) for every
    /// other entity and a non-emerging warden.
    (with_warden_emerge_seconds) warden_emerge_seconds: f32 = -1.0;
    /// Vanilla `Warden.diggingAnimationState` elapsed seconds (`Pose.DIGGING`-driven, the 5.0s
    /// `WARDEN_DIG` despawn burrow), sampled by `WardenModel.setupAnim`. `-1.0` (stopped) for every
    /// other entity and a non-digging warden.
    (with_warden_dig_seconds) warden_dig_seconds: f32 = -1.0;
    /// Vanilla `Rabbit.hopAnimationState` elapsed seconds (entity event `1`-seeded, the 0.75s looping
    /// `RabbitAnimation.HOP`), sampled by `RabbitModel.setupAnim`. `-1.0` (stopped) for every other
    /// entity and a resting rabbit; a non-negative value is wrapped by the looping def length.
    (with_rabbit_hop_seconds) rabbit_hop_seconds: f32 = -1.0;
    /// Vanilla `Creaking.canMove()` (synced `CAN_MOVE`, default `true`): gates `CreakingModel`'s
    /// looping walk. A creaking frozen while observed turns to a statue. `true` for every other entity.
    (with_creaking_can_move) creaking_can_move: bool = true;
    /// Vanilla `Creaking.attackAnimationState` elapsed seconds (entity event `4`-seeded, the 0.7083s
    /// looping `CREAKING_ATTACK` lunge), sampled by `CreakingModel.setupAnim`. `-1.0` (stopped) for
    /// every other entity and a non-attacking creaking; a non-negative value is wrapped by the looping
    /// def length.
    (with_creaking_attack_seconds) creaking_attack_seconds: f32 = -1.0;
    /// Vanilla `Creaking.invulnerabilityAnimationState` elapsed seconds (entity event `66`-seeded, the
    /// 0.2917s `CREAKING_INVULNERABLE` stagger), sampled by `CreakingModel.setupAnim`. `-1.0` (stopped)
    /// for every other entity and a non-staggering creaking.
    (with_creaking_invulnerable_seconds) creaking_invulnerable_seconds: f32 = -1.0;
    /// Vanilla `Creaking.deathAnimationState` elapsed seconds (synced `isTearingDown()`-driven, the
    /// 2.25s `CREAKING_DEATH` collapse), sampled by `CreakingModel.setupAnim`. `-1.0` (stopped) for
    /// every other entity and a non-tearing-down creaking.
    (with_creaking_death_seconds) creaking_death_seconds: f32 = -1.0;
    /// Vanilla `LivingEntityRenderState.isAutoSpinAttack` riptide spin: when the
    /// entity is mid-trident-spin, `Some(ageInTicks)` (the lerped
    /// `ageInTicks + partialTick`) drives the `LivingEntityRenderer.setupRotations`
    /// branch `Axis.XP.rotationDegrees(-90 - xRot)` then
    /// `Axis.YP.rotationDegrees(ageInTicks * -75)`. `None` for every entity that is
    /// not spinning (the death tip-over takes precedence over this branch).
    (with_auto_spin_age_ticks) auto_spin_age_ticks: Option<f32> = None;
    /// Vanilla `LivingEntityRenderState.isUpsideDown` Dinnerbone/Grumm flip: when
    /// the entity is upside down, `Some(boundingBoxHeight)` drives the
    /// `LivingEntityRenderer.setupRotations` branch `translate(0, (bbHeight + 0.1) /
    /// entityScale, 0)` then `Axis.ZP.rotationDegrees(180)`. Carried as the world
    /// `boundingBoxHeight` because the post-yaw frame is already in world units (the
    /// model scale is applied innermost), so the `/ entityScale` is unnecessary.
    /// `None` for every entity that is not upside down (death and the riptide spin
    /// both take precedence over this branch).
    (with_upside_down_height) upside_down_height: Option<f32> = None;
    /// Vanilla `EntityRenderState.boundingBoxHeight` (`Entity.getBbHeight`), in world units. Drowned
    /// swim rotations use `bbHeight / 2 / entityScale` as their pivot after the standard living setup.
    /// `0.0` for callers that do not provide an AABB-derived height.
    (with_bounding_box_height) bounding_box_height: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: when sleeping in a
    /// bed, the renderer skips the `180 - bodyRot` yaw and lays the model down via
    /// [`SleepingPose`]. `None` for every entity that is not sleeping. Death and
    /// the riptide spin take precedence over this branch; this branch takes
    /// precedence over the upside-down flip.
    (with_sleeping) sleeping: Option<SleepingPose> = None;
    /// Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the `SCALE`
    /// attribute): the uniform model scale `LivingEntityRenderer.submit` applies as
    /// `poseStack.scale(scale, scale, scale)` before `setupRotations`. `1.0` for an
    /// entity at its default size.
    (with_scale) scale: f32 = 1.0;
    /// Vanilla `HumanoidRenderState.swimAmount` / `LivingEntity.getSwimAmount(partialTick)`: the
    /// eased 0..1 blend toward `isVisuallySwimming()`. The drowned consumes this for
    /// `DrownedRenderer.setupRotations` and the `DrownedModel.setupAnim` swim limb override. `0.0`
    /// for a living entity that is not visually swimming and every non-living entity.
    (with_swim_amount) swim_amount: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.walkAnimationPos`
    /// (`WalkAnimationState.position(partialTick)`): the lerped limb-swing position
    /// that models feed into the `cos(animationPos * 0.6662 ...)` leg/arm sway in
    /// `setupAnim`. `0.0` for a standing entity.
    () walk_animation_pos: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.walkAnimationSpeed`
    /// (`WalkAnimationState.speed(partialTick)`): the lerped limb-swing amplitude
    /// (`0.0..=1.0`) that scales the sway in `setupAnim`. `0.0` for a standing
    /// entity, leaving the model in its rest pose.
    () walk_animation_speed: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.wornHeadAnimationPos`: the animation position passed to
    /// `SkullBlockRenderer.submitSkull` for skull equipment. It equals the entity walk animation
    /// position for non-passenger entities and the vehicle walk animation position when riding a
    /// living entity.
    (with_worn_head_animation_pos) worn_head_animation_pos: f32 = 0.0;
    /// Vanilla `IllagerRenderState.isRiding` (`entity.isPassenger()`): an illager riding any vehicle
    /// uses the fixed seated arm/leg pose before its arm-pose branch runs. `false` for every
    /// non-passenger entity; currently consumed by `IllagerModel`.
    (with_is_riding) is_riding: bool = false;
    /// Vanilla `HumanoidRenderState.attackTime` (`LivingEntity.getAttackAnim(partialTick)`): the
    /// lerped `0..1` melee swing progress that `HumanoidModel.setupAttackAnimation` turns into the
    /// body twist + arm whack (the off arm tracked via [`attack_arm_off_hand`](Self::attack_arm_off_hand)).
    /// `0.0` for an entity that is not mid-swing, leaving the arms on their walk/idle pose.
    (with_attack_anim) attack_anim: f32 = 0.0;
    /// Vanilla `HumanoidRenderState.attackArm` (`LivingEntity.swingingArm`): whether the active swing
    /// is the off (left) hand. `false` for a main-hand swing (the common case) and every entity that
    /// is not mid-swing.
    (with_attack_arm_off_hand) attack_arm_off_hand: bool = false;
    /// Vanilla `EntityRenderState.ageInTicks` (`entity.tickCount + partialTick`): the
    /// lerped per-frame age that drives continuous idle animations (e.g. the
    /// `AbstractPiglinModel` ear flap). `0.0` until the entity scene projects it.
    (with_age_in_ticks) age_in_ticks: f32 = 0.0;
    /// Vanilla `Mob.isAggressive()` (`DATA_MOB_FLAGS_ID & 4`): deepens the held-out
    /// `animateZombieArms` arm drop for the zombie-model family (`-π / 1.5` aggressive vs
    /// `-π / 2.25` calm). `false` for every calm or non-zombie-family entity.
    (with_is_aggressive) is_aggressive: bool = false;
    /// Vanilla `SkeletonRenderState.isHoldingBow` (`getMainHandItem().is(Items.BOW)`): with
    /// [`is_aggressive`](Self::is_aggressive), `AbstractSkeletonRenderer.getArmPose` returns
    /// `BOW_AND_ARROW`, so `SkeletonModel` aims both arms forward along the head look. `false` for every
    /// non-skeleton entity and for a skeleton not holding a bow.
    (with_main_hand_holds_bow) main_hand_holds_bow: bool = false;
    /// Vanilla `HumanoidRenderState.swingAnimationType == STAB` (`ItemStack.getSwingAnimation().type()`):
    /// a held spear makes `HumanoidModel.setupAttackAnimation` run `SpearAnimations.thirdPersonAttackHand`
    /// (the lunge/retract stab) instead of the default `WHACK` arm chop. `false` for every entity not
    /// holding a spear — only `PlayerModel` consumes it (the default `WHACK` covers every other case).
    (with_main_hand_swing_is_stab) main_hand_swing_is_stab: bool = false;
    /// Vanilla `HumanoidModel.setupAnim` use-item arm pose `SPYGLASS`
    /// (`ItemStack.getUseAnimation() == SPYGLASS`): a player using a spyglass raises the holding arm to
    /// the eye (`xRot = clamp(head.xRot − 1.9198622 − crouch?π/12, −2.4, 3.3)`, `yRot = head.yRot ∓ π/12`)
    /// and that arm skips the idle bob. Applied to the [`use_item_off_hand`](Self::use_item_off_hand) arm.
    /// `false` for every entity not using a spyglass — only `PlayerModel` consumes it.
    (with_player_using_spyglass) player_using_spyglass: bool = false;
    /// Vanilla `HumanoidModel.setupAnim` use-item arm pose `TOOT_HORN`
    /// (`ItemStack.getUseAnimation() == TOOT_HORN`): a player tooting a goat horn raises the holding arm
    /// to the mouth (`xRot = clamp(head.xRot, −1.2, 1.2) − 1.4835298`, `yRot = head.yRot ± π/6`). Applied
    /// to the [`use_item_off_hand`](Self::use_item_off_hand) arm. `false` for every entity not tooting a
    /// horn — only `PlayerModel` consumes it.
    (with_player_tooting_horn) player_tooting_horn: bool = false;
    /// Vanilla `HumanoidModel.setupAnim` use-item arm pose `BRUSH`
    /// (`ItemStack.getUseAnimation() == BRUSH`): a player brushing lowers the holding arm to the block
    /// (`xRot = arm.xRot · 0.5 − π/5`, `yRot = 0`). Applied to the
    /// [`use_item_off_hand`](Self::use_item_off_hand) arm. `false` for every entity not brushing — only
    /// `PlayerModel` consumes it.
    (with_player_brushing) player_brushing: bool = false;
    /// Vanilla `AvatarRenderer.getArmPose` fallback `ITEM` arm pose
    /// (`HumanoidModel.poseRightArm` ITEM case): a player holding a generic main-hand item lowers and halves
    /// the arm swing (`xRot = arm.xRot · 0.5 − π/10`, `yRot = 0`). `false` for every entity not holding a
    /// plain item in the main hand — only `PlayerModel` consumes it, and only when not using an item and the
    /// item is not a spear (`SPEAR`) or charged crossbow (`CROSSBOW_HOLD`).
    (with_player_main_hand_item_pose) player_main_hand_item_pose: bool = false;
    /// Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM` arm pose
    /// (`HumanoidModel.poseLeftArm` ITEM case): a player holding a plain off-hand item lowers and halves the
    /// OFF (left) arm (`xRot = arm.xRot · 0.5 − π/10`, `yRot = 0`). `false` for every entity not holding a
    /// plain item in the off hand — only `PlayerModel` consumes it, and only when not using that hand and the
    /// item is not a spear (`SPEAR`) or charged crossbow (`CROSSBOW_HOLD`).
    (with_player_off_hand_item_pose) player_off_hand_item_pose: bool = false;
    /// Vanilla `AvatarRenderState.skin.cape()` as a downloaded profile texture. The actual cape layer also
    /// checks the `PlayerModelPart.CAPE` visibility bit carried by [`EntityModelKind::Player`] and emits only
    /// once the dynamic profile-texture atlas has the matching handle.
    (with_player_cape_texture) player_cape_texture: Option<EntityDynamicPlayerTexture> = None;
    /// Vanilla `AvatarRenderState.skin.elytra()` as a downloaded profile texture. Projected for
    /// `WingsLayer`; the cape layer does not consume it.
    (with_player_elytra_texture) player_elytra_texture: Option<EntityDynamicPlayerTexture> = None;
    /// First renderable WINGS equipment layer from `HumanoidRenderState.chestEquipment`'s equipment
    /// asset (vanilla `WingsLayer` -> `EquipmentLayerRenderer.renderLayers(WINGS, ..., order=0)`).
    /// The native side resolves the pack layer texture to a renderer atlas texture and preserves
    /// `use_player_texture` for the player-only profile elytra/cape override selection.
    (with_chest_wings_layer) chest_wings_layer: Option<EntityEquipmentLayerTexture> = None;
    /// Whether `HumanoidRenderState.chestEquipment` has an equipment asset with a WINGS layer. Vanilla
    /// player `CapeLayer` suppresses the cape when this is true because the wings layer owns the back
    /// slot.
    (with_chest_equipment_has_wings) chest_equipment_has_wings: bool = false;
    /// Whether `HumanoidRenderState.chestEquipment` has an equipment asset with a HUMANOID layer.
    /// Vanilla player `CapeLayer` nudges the cape away from humanoid chest equipment by
    /// `(0, -0.053125, 0.06875)`.
    (with_chest_equipment_has_humanoid) chest_equipment_has_humanoid: bool = false;
    /// Vanilla `AvatarRenderState.capeFlap`, produced by `AvatarRenderer.extractCapeState` from the
    /// world-projected cloak interpolation.
    (with_player_cape_flap) player_cape_flap: f32 = 0.0;
    /// Vanilla `AvatarRenderState.capeLean`, produced by `AvatarRenderer.extractCapeState`.
    (with_player_cape_lean) player_cape_lean: f32 = 0.0;
    /// Vanilla `AvatarRenderState.capeLean2`, produced by `AvatarRenderer.extractCapeState`.
    (with_player_cape_lean2) player_cape_lean2: f32 = 0.0;
    /// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` use-item arm pose `BLOCK` (`poseBlockingArm`):
    /// while a player raises a blocking item (`isUsingItem` + the using hand holds a non-consumable
    /// `BLOCKS_ATTACKS` item, normally the shield) the holding arm tucks the item forward along the head
    /// look — `xRot = arm.xRot · 0.5 − 0.9424779 + clamp(head.xRot, −4π/9, 0.43633232)`, `yRot = (right ?
    /// −π/6 : π/6) + clamp(head.yRot, −π/6, π/6)`.
    /// Applied to the [`use_item_off_hand`](Self::use_item_off_hand) arm. `false` for every entity not
    /// raising a blocking item — only `PlayerModel` consumes it.
    (with_player_blocking) player_blocking: bool = false;
    /// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` use-item arm pose `THROW_TRIDENT`: while a player
    /// charges a trident throw (`isUsingItem` + the using hand holds a trident) the holding arm raises the
    /// trident straight overhead — `xRot = arm.xRot · 0.5 − π`, `yRot = 0`. Applied to the
    /// [`use_item_off_hand`](Self::use_item_off_hand) arm. `false` for every entity not charging a trident —
    /// only `PlayerModel` consumes it (the drowned reaches the same pose via its own aggression path).
    (with_player_throwing_trident) player_throwing_trident: bool = false;
    /// Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` use-item arm pose `BOW_AND_ARROW`: while a player
    /// draws a bow, BOTH arms raise along the head look. The brace yaw offset mirrors when
    /// [`use_item_off_hand`](Self::use_item_off_hand) is true. The pose has `affectsOffhandPose`, so the
    /// opposite arm's fallback `ITEM` branch is suppressed in the projection. `false` for every entity not
    /// drawing a bow — only `PlayerModel` consumes it.
    (with_player_drawing_bow) player_drawing_bow: bool = false;
    /// Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` use-item arm pose `CROSSBOW_CHARGE`
    /// (`AnimationUtils.animateCrossbowCharge`): while a player draws an uncharged crossbow, the holding arm
    /// braces and the opposite arm pulls the string back over the draw, lerping by
    /// [`crossbow_charge_ticks`](Self::crossbow_charge_ticks) `/ 25`. The holding arm is selected by
    /// [`use_item_off_hand`](Self::use_item_off_hand). `false` for every entity not drawing a crossbow — only
    /// `PlayerModel` consumes it on this flag (the pillager/piglin route through their own
    /// `is_charging_crossbow`).
    (with_player_charging_crossbow) player_charging_crossbow: bool = false;
    /// Vanilla `AvatarRenderer.getArmPose` `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`): a player
    /// holding a charged main-hand crossbow while not mid-swing levels the crossbow along the head look (the
    /// right arm braces it, the left reaches the trigger). Reuses the same pose helper as the pillager.
    /// `false` for every entity not holding a charged crossbow — only `PlayerModel` consumes it on this flag
    /// (the pillager routes through its own `is_charging_crossbow` `false` path).
    (with_player_crossbow_hold) player_crossbow_hold: bool = false;
    /// Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` `CROSSBOW_HOLD`: a player holding a charged off-hand
    /// crossbow while not mid-swing levels the mirrored crossbow pose (`holdingInRightArm = false`) unless a
    /// main-hand two-handed/affecting pose suppresses `poseLeftArm`. `false` for every entity not holding a
    /// charged off-hand crossbow — only `PlayerModel` consumes it.
    (with_player_crossbow_hold_off_hand) player_crossbow_hold_off_hand: bool = false;
    /// Vanilla `LivingEntity.getUsedItemHand()` off-hand bit: which arm the use-item pose
    /// ([`player_using_spyglass`](Self::player_using_spyglass)) applies to. `false` (main / right arm) when
    /// not using an off-hand item.
    (with_use_item_off_hand) use_item_off_hand: bool = false;
    /// Vanilla `Pillager.isHolding(Items.CROSSBOW)`: with [`is_charging_crossbow`](Self::is_charging_crossbow)
    /// `false`, `Pillager.getArmPose` returns `CROSSBOW_HOLD`, so `IllagerModel` levels the crossbow
    /// (`AnimationUtils.animateCrossbowHold`) along the head look. `false` for every non-pillager entity
    /// and for a pillager whose main hand is not a crossbow.
    (with_main_hand_holds_crossbow) main_hand_holds_crossbow: bool = false;
    /// Vanilla `DrownedRenderer.getArmPose` `THROW_TRIDENT`: a drowned holding a trident in its main hand
    /// while aggressive raises the arm overhead to throw (`DrownedModel.setupAnim` sets the main arm
    /// `xRot = xRot*0.5 - π`, `yRot = 0` after the held-out zombie arms). `false` for every other entity,
    /// a calm drowned, and a drowned not holding a trident.
    (with_drowned_throw_trident) drowned_throw_trident: bool = false;
    /// Vanilla `Pillager.isChargingCrossbow()` (the synced `IS_CHARGING_CROSSBOW` boolean, id 17):
    /// `Pillager.getArmPose` returns `CROSSBOW_CHARGE` instead of `CROSSBOW_HOLD` while drawing, so the
    /// pillager pulls the crossbow back ([`crossbow_charge_ticks`](Self::crossbow_charge_ticks)
    /// drives the draw). `false` for every non-pillager entity.
    (with_is_charging_crossbow) is_charging_crossbow: bool = false;
    /// Vanilla `{Illager,Piglin}RenderState.ticksUsingItem` for the `CROSSBOW_CHARGE` draw — the
    /// reconstructed `getTicksUsingItem()`. `AnimationUtils.animateCrossbowCharge` lerps the pulling arm
    /// from rest to full draw over `ticksUsingItem / maxChargeDuration`. Shared by the pillager
    /// (`IllagerModel`) and the regular piglin (`PiglinModel`). `0.0` for anything not mid-draw.
    (with_crossbow_charge_ticks) crossbow_charge_ticks: f32 = 0.0;
    /// Vanilla `EndermanRenderState.carriedBlock` non-empty: the enderman is holding a
    /// block, so `EndermanModel.setupAnim` poses both arms forward (`xRot = -0.5`, `zRot =
    /// ±0.05`). `false` for every other entity.
    (with_enderman_carrying) enderman_carrying: bool = false;
    /// Vanilla `EndermanRenderState.isCreepy`: the enderman is staring at a player, so
    /// `EndermanModel.setupAnim` drops the head (`y -= 5`) and raises the hat (`y += 5`)
    /// into the open-mouth screech pose. `false` for every other entity.
    (with_enderman_creepy) enderman_creepy: bool = false;
    /// Vanilla `BatRenderState.isResting`: the bat is hanging at rest, so `BatModel.setupAnim`
    /// applies the `BatAnimation.BAT_RESTING` upside-down pose (and a head look) instead of
    /// the flying flap. `false` for every other entity (and for a flying bat).
    (with_bat_resting) bat_resting: bool = false;
    /// Vanilla `BeeRenderState.hasStinger` (`!Bee.hasStung()`): whether the bee still carries
    /// its stinger cube, which `BeeModel.setupAnim` toggles via `stinger.visible`. `true` for
    /// every other entity and for a bee that has not stung; `false` only for a bee that has
    /// lost its stinger.
    (with_bee_has_stinger) bee_has_stinger: bool = true;
    /// Vanilla `BeeRenderState.isAngry` (`Bee.isAngry()`): an angry bee skips
    /// `BeeModel.bobUpAndDown`, so its body, front/back legs and antennae hold still (the wing
    /// flap continues). `false` for every other entity and for a calm bee.
    (with_bee_angry) bee_angry: bool = false;
    /// Vanilla `BeeRenderState.rollAmount` (`Bee.getRollAmount(partialTick)`): a rolling bee tips
    /// onto its back, which `BeeModel.setupAnim` applies last as `bone.xRot =
    /// rotLerpRad(rollAmount, bone.xRot, 3.0915928)`. `0.0` (upright) for every other entity and
    /// for an upright bee.
    (with_bee_roll_amount) bee_roll_amount: f32 = 0.0;
    /// Vanilla `Camel.sitAnimationState` elapsed seconds (the 2.0 s `CAMEL_SIT`, non-looping),
    /// driven by `Camel.setupAnimationStates()` while the camel is visually sitting AND inside the
    /// 40-tick sit-down window (`isVisuallySittingDown()`). `CamelModel.setupAnim` applies
    /// `sitAnimation.apply(...)` ADDITIVELY onto the walk pose; the renderer samples `CAMEL_SIT` at
    /// these seconds when `>= 0` (clamping past 2.0 s to the seated final frame). `-1.0` (the
    /// stopped-animation sentinel) for every other entity and for a standing camel, so no keyframe
    /// is applied. Projected purely from the synced `LAST_POSE_CHANGE_TICK` + game time.
    (with_camel_sit_seconds) camel_sit_seconds: f32 = -1.0;
    /// Vanilla `Camel.sitPoseAnimationState` elapsed seconds (the 1.0 s `CAMEL_SIT_POSE`,
    /// non-looping), driven by `Camel.setupAnimationStates()` while the camel is visually sitting but
    /// past the sit-down window (the steady seated hold). Its `AnimationState` starts when the
    /// 40-tick sit-down window ends, so the projected elapsed is `getPoseTime - 40`.
    /// `CamelModel.setupAnim` applies it ADDITIVELY; `-1.0` for every other entity and a camel that
    /// is not holding the seated pose.
    (with_camel_sit_pose_seconds) camel_sit_pose_seconds: f32 = -1.0;
    /// Vanilla `Camel.sitUpAnimationState` elapsed seconds (the 2.6 s `CAMEL_STANDUP`, non-looping),
    /// driven by `Camel.setupAnimationStates()` while the camel is NOT visually sitting but still in
    /// the stand-up pose transition (`isInPoseTransition() && getPoseTime() >= 0`). Its elapsed is
    /// `getPoseTime`. `CamelModel.setupAnim` applies `standupAnimation.apply(...)` ADDITIVELY;
    /// `-1.0` for every other entity and a camel that is not standing up. (`idle` stays deferred;
    /// see `docs/unsupported-features.md`.)
    (with_camel_standup_seconds) camel_standup_seconds: f32 = -1.0;
    /// Vanilla `Camel.dashAnimationState` elapsed seconds (the looping 0.5 s `CAMEL_DASH`), driven by
    /// `Camel.setupAnimationStates()`'s `dashAnimationState.animateWhen(isDashing(), tickCount)` — the
    /// synced `DASH` boolean rising edge starts it. `CamelModel.setupAnim` applies
    /// `dashAnimation.apply(...)` ADDITIVELY (last, over the walk pose). `-1.0` (the stopped-animation
    /// sentinel) for every other entity and a camel that is not dashing.
    (with_camel_dash_seconds) camel_dash_seconds: f32 = -1.0;
    /// Vanilla frog croak timing (`FrogRenderState.croakAnimationState` driven by the synced
    /// `Pose.CROAKING`): the elapsed seconds since the croak started, projected for
    /// `FrogModel.setupAnim`, which shows the `croaking_body` pouch (`croakAnimationState.isStarted`)
    /// and samples the triggered `FrogAnimation.FROG_CROAK` POSITION/SCALE pouch animation. `-1.0`
    /// (the sentinel for a stopped `croakAnimationState`) for every other entity and for a
    /// non-croaking frog, so the pouch stays hidden and no keyframe is applied.
    (with_frog_croak_seconds) frog_croak_seconds: f32 = -1.0;
    /// Vanilla frog tongue timing (`FrogRenderState.tongueAnimationState` driven by the synced
    /// `Pose.USING_TONGUE`): the elapsed seconds since the tongue lash started, projected for
    /// `FrogModel.setupAnim`, which dips the head and z-scales the `tongue` part forward by sampling
    /// the triggered `FrogAnimation.FROG_TONGUE` ROTATION/SCALE animation. `-1.0` (the sentinel for a
    /// stopped `tongueAnimationState`) for every other entity and a frog not using its tongue, so no
    /// keyframe is applied.
    (with_frog_tongue_seconds) frog_tongue_seconds: f32 = -1.0;
    /// Vanilla frog jump timing (`FrogRenderState.jumpAnimationState` driven by the synced
    /// `Pose.LONG_JUMPING`): the elapsed seconds since the long-jump started, projected for
    /// `FrogModel.setupAnim`, which samples the triggered `FrogAnimation.FROG_JUMP` POSITION/ROTATION
    /// pose onto the body, arms, and legs. `-1.0` (the sentinel for a stopped `jumpAnimationState`)
    /// for every other entity and for a non-jumping frog, so no keyframe is applied.
    (with_frog_jump_seconds) frog_jump_seconds: f32 = -1.0;
    /// Vanilla frog swim-idle timing (`FrogRenderState.swimIdleAnimationState`, driven each client
    /// tick by `Frog.tick`'s `animateWhen(isInWater() && !walkAnimation.isMoving(), tickCount)`): the
    /// elapsed seconds since the in-water idle started, projected for `FrogModel.setupAnim`, which
    /// applies the looping `FrogAnimation.FROG_IDLE_WATER` ROTATION/POSITION pose onto the body, arms,
    /// and legs (last, after the walk/swim). `-1.0` (the sentinel for a stopped
    /// `swimIdleAnimationState`) for every other entity and for a frog that is dry or moving, so no
    /// keyframe is applied.
    (with_frog_swim_idle_seconds) frog_swim_idle_seconds: f32 = -1.0;
    /// Vanilla sniffer animation selector (`Sniffer.onSyncedDataUpdated`'s one-shot `AnimationState`s
    /// driven by the synced `DATA_STATE`): the active `Sniffer.State` ordinal whose triggered keyframe
    /// is playing (`FEELING_HAPPY=1`/`SCENTING=2`/`SNIFFING=3`/`DIGGING=5`/`RISING=6`), which
    /// `SnifferModel.setupAnim` matches to pick and apply the keyframe def. `-1` (no triggered
    /// animation) for every other entity and for an idling/searching sniffer.
    (with_sniffer_animation_id) sniffer_animation_id: i32 = -1;
    /// Vanilla sniffer animation timing: the elapsed seconds since the active `Sniffer.State`
    /// animation started (paired with [`Self::sniffer_animation_id`]), sampled by
    /// `SnifferModel.setupAnim`. `-1.0` (the stopped-animation sentinel) for every other entity and
    /// for an idling/searching sniffer.
    (with_sniffer_animation_seconds) sniffer_animation_seconds: f32 = -1.0;
    /// Vanilla `SnifferRenderState.isSearching` (`Sniffer.isSearching()`, synced `DATA_STATE` ==
    /// `SEARCHING`): swaps the base `SNIFFER_WALK` for the looping `SNIFFER_SNIFF_SEARCH` search-walk
    /// in `SnifferModel.setupAnim`. `false` for every non-searching sniffer and every other entity.
    (with_sniffer_is_searching) sniffer_is_searching: bool = false;
    /// Vanilla `ArmadilloRenderState.isHidingInShell` (`Armadillo.shouldHideInShell()`): the synced
    /// `ARMADILLO_STATE` gated on the client `inStateTicks` — `true` for the steady SCARED ball and
    /// for the ROLLING/UNROLLING transition windows. `ArmadilloModel.setupAnim` renders the shell ball
    /// (body/tail/hind legs hidden) when set. `false` (unrolled) for every other entity.
    (with_armadillo_is_hiding_in_shell) armadillo_is_hiding_in_shell: bool = false;
    /// Vanilla armadillo roll-up timing (`Armadillo.rollUpAnimationState`, started on entry to
    /// ROLLING): the elapsed seconds since the curl-in began, which `ArmadilloModel.setupAnim` samples
    /// from `ARMADILLO_ROLL_UP` onto the body/legs/head. `-1.0` (the stopped-animation sentinel) for
    /// every other entity and for an armadillo that is not rolling up.
    (with_armadillo_roll_up_seconds) armadillo_roll_up_seconds: f32 = -1.0;
    /// Vanilla armadillo roll-out timing (`Armadillo.rollOutAnimationState`, started on entry to
    /// UNROLLING): the elapsed seconds since the un-curl began, sampled from `ARMADILLO_ROLL_OUT`.
    /// `-1.0` for every other entity and for an armadillo that is not unrolling.
    (with_armadillo_roll_out_seconds) armadillo_roll_out_seconds: f32 = -1.0;
    /// Vanilla armadillo peek timing (`Armadillo.peekAnimationState`): elapsed seconds for
    /// `ARMADILLO_PEEK`, including the first SCARED setup tick's `fastForward(50, 1.0F)` baseline
    /// and entity event `64` restart. `-1.0` for every other entity and while no peek is running.
    (with_armadillo_peek_seconds) armadillo_peek_seconds: f32 = -1.0;
    /// Vanilla `FoxRenderState.headRollAngle` (`Fox.getHeadRollAngle(partialTick)`): an interested
    /// fox tilts its head, which `FoxModel.setWalkingPose` applies as `head.zRot = headRollAngle`.
    /// `0.0` (level) for every other entity and for a fox that is not interested.
    (with_fox_head_roll_angle) fox_head_roll_angle: f32 = 0.0;
    /// Vanilla `FoxRenderState.crouchAmount` (`Fox.getCrouchAmount(partialTick)`): a stalking fox
    /// lowers its body, which `FoxModel.setCrouchingPose` applies as `head.y += crouchAmount ·
    /// ageScale` (plus the adult `body.y += crouchAmount` / baby `+ crouchAmount/6` drop and the
    /// pounce `body.y -= crouchAmount/2`). `0.0` for every other entity and for an upright fox.
    (with_fox_crouch_amount) fox_crouch_amount: f32 = 0.0;
    /// Vanilla `FoxRenderState.isCrouching` (`Fox.isCrouching()`, the synced `DATA_FLAGS_ID & 4`): a
    /// stalking fox, whose `FoxModel.setupAnim` runs `setCrouchingPose` (the first pose branch, taken
    /// over sleeping/sitting). `false` for every other entity and for an upright fox.
    (with_fox_is_crouching) fox_is_crouching: bool = false;
    /// Vanilla `FoxRenderState.isSleeping` (`Fox.isSleeping()`, the synced `DATA_FLAGS_ID & 32`): a
    /// sleeping fox, whose `FoxModel.setSleepingPose` hides all four legs and whose `setupAnim`
    /// overrides the head pose with a wobble. `false` for every other entity and for an awake fox.
    (with_fox_is_sleeping) fox_is_sleeping: bool = false;
    /// Vanilla `FoxRenderState.isSitting` (`Fox.isSitting()`, the synced `DATA_FLAGS_ID & 1`): a
    /// perched fox, whose `FoxModel.setSittingPose` folds it down. `false` for every other entity and
    /// for a standing fox.
    (with_fox_is_sitting) fox_is_sitting: bool = false;
    /// Vanilla `FoxRenderState.isPouncing` (`Fox.isPouncing()`, the synced `DATA_FLAGS_ID & 16`): a
    /// pouncing fox, whose `FoxModel.setPouncingPose` (adult only) drops the body/head by
    /// `crouchAmount/2`. `false` for every other entity and for a fox that is not pouncing. The
    /// `FoxRenderer.setupRotations` body-pitch flip is a deferred renderer concern.
    (with_fox_is_pouncing) fox_is_pouncing: bool = false;
    /// Vanilla `FoxRenderState.isFaceplanted` (`Fox.isFaceplanted()`, the synced `DATA_FLAGS_ID &
    /// 64`): a face-planted fox, whose `FoxModel.setupAnim` twitches all four legs on `ageInTicks`.
    /// `false` for every other entity and for an upright fox. The `FoxRenderer.setupRotations`
    /// body-pitch flip is a deferred renderer concern.
    (with_fox_is_faceplanted) fox_is_faceplanted: bool = false;
    /// Vanilla `WitchRenderState.isHoldingItem`: true when the witch's main hand is non-empty. The model
    /// pins the nose to the drinking pose (`nose.setPos(0, 1, -1.5)`, `xRot = -0.9`) before item layers
    /// read the nose transform. `false` for every other entity and for an empty-handed witch.
    (with_witch_holding_item) witch_holding_item: bool = false;
    /// Vanilla `WitchRenderState.isHoldingPotion`: true only when the main-hand item is
    /// `Items.POTION`; `WitchItemLayer` then attaches the rendered stack to the nose instead of the
    /// crossed arms. `false` for non-potion held items and every other entity.
    (with_witch_holding_potion) witch_holding_potion: bool = false;
    /// Vanilla `CopperGolemModel.setupAnim`: when either hand item state is non-empty, the golem samples
    /// the walk-with-item branch and clamps both arms into the resting held-item pose before
    /// `ItemInHandLayer` reads the hand transform. `false` for empty-handed copper golems and every other
    /// entity. The walk-with-item keyframe itself remains deferred.
    (with_copper_golem_holding_item) copper_golem_holding_item: bool = false;
    /// Vanilla `LivingEntityRenderState.wornHeadType`: a supported skull block item in the HEAD slot,
    /// rendered by `CustomHeadLayer` through `SkullBlockRenderer` instead of the generic item-model path.
    /// Static mob skulls, profileless default player heads, profiled player-head default skin
    /// fallbacks, and the specialized dragon/piglin skull models are projected; live player-skin
    /// downloads remain deferred.
    (with_custom_head_skull) custom_head_skull: Option<EntityCustomHeadSkull> = None;
    /// Vanilla `VexRenderState.isCharging` (`Vex.isCharging`, the synced `DATA_FLAGS_ID & 1`):
    /// the vex is charging an attack, so `VexModel.setupAnim` levels the body (`xRot = 0`) and
    /// `setArmsCharging` raises both arms or only the item-bearing arms. `false` for every other
    /// entity and for an idle vex.
    (with_vex_charging) vex_charging: bool = false;
    /// Vanilla `ArmedEntityRenderState.rightHandItemState` non-empty for Vex: when charging with an item
    /// in the RIGHT hand, `VexModel.setArmsCharging` pitches only that arm to `xRot = π*7/6` while
    /// empty hands keep their pre-charging rest roll. `false` for idle vexes, empty right hands, and every
    /// non-vex entity. Native currently projects Vex main hand as the default RIGHT arm.
    (with_vex_right_hand_item_non_empty) vex_right_hand_item_non_empty: bool = false;
    /// Vanilla `ArmedEntityRenderState.leftHandItemState` non-empty for Vex: when charging with an item in
    /// the LEFT hand, `VexModel.setArmsCharging` pitches only that arm to `xRot = π*7/6` while empty hands
    /// keep their pre-charging rest roll. `false` for idle vexes, empty left hands, and every non-vex entity.
    /// Native currently projects Vex offhand as the default LEFT arm.
    (with_vex_left_hand_item_non_empty) vex_left_hand_item_non_empty: bool = false;
    /// Vanilla `WitherRenderState.invulnerableTicks` (`WitherBoss.getInvulnerableTicks`, the synced
    /// `DATA_ID_INV` spawn countdown, lerped `invulnerableTicks - partialTicks`): the wither's
    /// spawn-charge progress. `WitherBossRenderer.scale` shrinks the model by
    /// `invulnerableTicks / 220 * 0.5` off its base `2.0` scale ([`wither_model_root_transform`]),
    /// and `getTextureLocation` swaps to `wither_invulnerable.png` (flickering every 5 ticks once
    /// `<= 80`). `0.0` for every other entity and for a fully-spawned wither.
    (with_wither_invulnerable_ticks) wither_invulnerable_ticks: f32 = 0.0;
    /// Vanilla `WitherRenderState.xHeadRots`: side-head pitch rotations in degrees,
    /// copied from `WitherBoss.xRotHeads` as `[right_head, left_head]`. `WitherBossModel`
    /// applies these directly as `head.xRot`. `[0.0; 2]` for every non-wither.
    (with_wither_x_head_rots) wither_x_head_rots: [f32; 2] = [0.0; 2];
    /// Vanilla `WitherRenderState.yHeadRots`: side-head absolute yaw rotations in
    /// degrees, copied from `WitherBoss.yRotHeads` as `[right_head, left_head]`.
    /// `WitherBossModel` applies `yHeadRot - bodyRot`. `[0.0; 2]` for every non-wither.
    (with_wither_y_head_rots) wither_y_head_rots: [f32; 2] = [0.0; 2];
    /// Vanilla `WitherRenderState.isPowered` (`WitherBoss.isPowered() = getHealth() <=
    /// getMaxHealth() / 2`): the wither is at or below half health, so the `WitherArmorLayer` energy
    /// swirl (the same `EnergySwirlLayer` as the charged creeper) glows over the inflated
    /// `WITHER_ARMOR` body. `false` for every other entity and for a healthy wither.
    (with_wither_powered) wither_powered: bool = false;
    /// Vanilla `HumanoidArmorLayer` worn armor, one material per equipment slot (head / chest / legs /
    /// feet), projected from the entity's `SetEquipment` items. `Some(material)` drapes that slot's
    /// inflated `HumanoidArmorModel` piece (helmet / chestplate / leggings / boots) over the host
    /// humanoid pose, textured by the material's equipment-asset texture; `None` leaves the slot bare.
    (with_head_armor) head_armor: Option<EntityArmorMaterial> = None;
    (with_chest_armor) chest_armor: Option<EntityArmorMaterial> = None;
    (with_legs_armor) legs_armor: Option<EntityArmorMaterial> = None;
    (with_feet_armor) feet_armor: Option<EntityArmorMaterial> = None;
    /// Vanilla `DyedItemColor.getOrDefault` per worn armor slot: the worn item's `dyed_color`
    /// component (a packed RGB), paired with the slot's [`Self::head_armor`] material. Only leather is
    /// dyeable, so `armor_layer_tint` applies this as the leather layer's tint (forced opaque) when
    /// `Some`, falling back to the default `DyedItemColor.LEATHER_COLOR` brown when `None`; every other
    /// material ignores it and renders white. `None` for an undyed / non-leather slot.
    (with_head_armor_dye) head_armor_dye: Option<u32> = None;
    (with_chest_armor_dye) chest_armor_dye: Option<u32> = None;
    (with_legs_armor_dye) legs_armor_dye: Option<u32> = None;
    (with_feet_armor_dye) feet_armor_dye: Option<u32> = None;
    /// Vanilla `PigRenderState.saddle`: the item in `EquipmentSlot.SADDLE`. A non-empty saddle item
    /// renders the adult `PigModel.createBodyLayer(CubeDeformation(0.5F))` equipment layer over the
    /// pig body. Babies do not render this layer because vanilla supplies no baby saddle model.
    (with_pig_saddle) pig_saddle: bool = false;
    /// Vanilla `EquineRenderState.saddle`: the item in `EquipmentSlot.SADDLE` for horse, donkey, mule,
    /// skeleton-horse, and zombie-horse renderers. A non-empty saddle item renders the adult
    /// `EquineSaddleModel` equipment layer; babies do not render it because vanilla supplies no baby
    /// saddle model.
    (with_equine_saddle) equine_saddle: bool = false;
    /// Vanilla `EquineRenderState.isRidden`: the equine saddle layer shows its two bridle line parts
    /// only while the entity is a vehicle. `false` for unridden equines and every non-equine entity.
    (with_equine_saddle_ridden) equine_saddle_ridden: bool = false;
    /// Vanilla `EquineRenderState.bodyArmorItem`: an adult horse / zombie horse body equipment item
    /// whose equipment asset provides a `HORSE_BODY` layer. Leather horse armor is dyeable and carries
    /// its optional dye through [`Self::equine_body_armor_dye`]. Babies skip it because vanilla
    /// supplies no baby armor model; skeleton horses are excluded by the vanilla armor-wearer tag.
    (with_equine_body_armor) equine_body_armor: Option<EntityArmorMaterial> = None;
    /// Vanilla `DyedItemColor.getOrDefault` for `EquineRenderState.bodyArmorItem`: the packed RGB dye
    /// component on leather horse armor. Non-leather horse armor ignores it and renders white; undyed
    /// leather falls back to `DyedItemColor.LEATHER_COLOR`.
    (with_equine_body_armor_dye) equine_body_armor_dye: Option<u32> = None;
    /// Vanilla `WolfRenderState.bodyArmorItem`: an adult wolf body equipment item whose equipment
    /// asset provides a `WOLF_BODY` layer. The renderer maps `ArmadilloScute` to the wolf armor
    /// equipment textures and skips babies.
    (with_wolf_body_armor) wolf_body_armor: Option<EntityArmorMaterial> = None;
    /// Vanilla `DyedItemColor.getOrDefault` for the wolf body armor item: a packed RGB dye. The
    /// `armadillo_scute_overlay` wolf-body layer renders only when this is present.
    (with_wolf_body_armor_dye) wolf_body_armor_dye: Option<u32> = None;
    /// Vanilla `Crackiness.WOLF_ARMOR.byDamage(bodyArmorItem)`: low / medium / high damage cracks over
    /// the adult wolf armor model. `None` draws no crack texture.
    (with_wolf_body_armor_crackiness) wolf_body_armor_crackiness: Option<WolfArmorCrackiness> = None;
    /// Vanilla `StriderRenderState.isRidden`: a strider with passengers zeroes body pitch/yaw in
    /// `StriderModel.setupAnim`. `false` for unridden striders and every non-strider entity.
    (with_strider_ridden) strider_ridden: bool = false;
    /// Vanilla `StriderRenderState.saddle`: the item in `EquipmentSlot.SADDLE`. A non-empty saddle item
    /// renders `AdultStriderModel(ModelLayers.STRIDER_SADDLE)` over the base strider. Babies skip the
    /// layer because vanilla supplies no baby saddle model.
    (with_strider_saddle) strider_saddle: bool = false;
    /// Vanilla `CamelRenderState.saddle`: the item in `EquipmentSlot.SADDLE` for camel and camel husk.
    /// A non-empty saddle item renders `CamelSaddleModel`; baby camels skip it because vanilla supplies
    /// no baby saddle model.
    (with_camel_saddle) camel_saddle: bool = false;
    /// Vanilla `CamelRenderState.isRidden`: the camel saddle layer shows its `reins` part only while
    /// the camel is a vehicle. `false` for unridden camels and every non-camel entity.
    (with_camel_saddle_ridden) camel_saddle_ridden: bool = false;
    /// Vanilla `NautilusRenderState.saddle`: the item in `EquipmentSlot.SADDLE` for living nautilus
    /// and zombie nautilus. A non-empty saddle item renders `NautilusSaddleModel`; baby living
    /// nautilus skip it because vanilla supplies no baby saddle model.
    (with_nautilus_saddle) nautilus_saddle: bool = false;
    /// Vanilla `NautilusRenderState.bodyArmorItem`: an adult living nautilus / zombie nautilus body
    /// equipment item whose equipment asset provides a `NAUTILUS_BODY` layer. Baby living nautilus
    /// skip it because vanilla supplies no baby armor model.
    (with_nautilus_body_armor) nautilus_body_armor: Option<EntityArmorMaterial> = None;
    /// Vanilla `SnowGolemRenderState.headBlock`: a carved pumpkin block model on the snow golem head
    /// while the synced `SnowGolem.DATA_PUMPKIN_ID` flag is set. The block layer is baked by native
    /// into the shared block-item model pass because it samples the terrain atlas.
    (with_snow_golem_pumpkin) snow_golem_pumpkin: bool = false;
    /// Vanilla `LlamaRenderState.bodyItem`: an adult llama/trader-llama body equipment item whose
    /// equipment asset is one of the 16 carpet colors. Baby llamas ignore body items for the decor
    /// layer; trader llamas render their built-in decor when this is `None`.
    (with_llama_body_decor) llama_body_decor: Option<EntityDyeColor> = None;
    /// Vanilla `IllagerRenderState.armPose == SPELLCASTING` (`SpellcasterIllager.isCastingSpell()`,
    /// the synced `DATA_SPELL_CASTING_ID` byte > 0): a casting evoker/illusioner, whose
    /// `IllagerModel.setupAnim` hides the crossed `arms` part and raises the two separate arms
    /// (`zRot = ±3π/4`, `xRot = cos(ageInTicks · 0.6662) · 0.25`). `false` for every other entity
    /// and for an idle illager (which shows the static CROSSED arms).
    (with_illager_spellcasting) illager_spellcasting: bool = false;
    /// Vanilla `Raider.isCelebrating()` (the synced `IS_CELEBRATING` boolean, id 16): a celebrating
    /// evoker/vindicator whose `getArmPose` returns `CELEBRATING` (when not casting / not aggressive),
    /// so `IllagerModel.setupAnim` raises the two separate arms into the victory dance (`zRot` raised,
    /// `xRot = cos(ageInTicks · 0.6662) · 0.05`). `false` for every other entity and for an idle illager
    /// (which shows the static CROSSED arms).
    (with_illager_celebrating) illager_celebrating: bool = false;
    /// Vanilla `IllagerModel.setupAnim` `ATTACKING` branch checks whether the rendered main-hand item is
    /// empty: empty-handed attacking illagers use `AnimationUtils.animateZombieArms`, while armed ones use
    /// `AnimationUtils.swingWeaponDown`. `false` preserves the usual armed vindicator path unless native
    /// has projected an actually empty main hand.
    (with_illager_main_hand_empty) illager_main_hand_empty: bool = false;
    /// Vanilla `Piglin.isDancing()` (the synced `DATA_IS_DANCING` boolean): a piglin dancing by a soul
    /// campfire, whose `PiglinModel.setupAnim` returns the `DANCING` arm pose — the ears sway, the head
    /// and body bob, and both arms raise overhead (`zRot = ±(70° + cos·10°)`) wagging with `ageInTicks`.
    /// `false` for every other entity (the piglin brute and zombified piglin never dance) and an idle
    /// piglin.
    (with_piglin_dancing) piglin_dancing: bool = false;
    /// Vanilla `Piglin.getArmPose` `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`, holding in the
    /// right arm): a regular piglin holding a charged crossbow (not dancing, not mid-draw) levels it along
    /// the head look — the right (holding) arm raises the crossbow and the left (shooting) arm reaches the
    /// trigger. `false` for every other entity, a charging/dancing piglin, and an empty-handed piglin.
    (with_piglin_crossbow_hold) piglin_crossbow_hold: bool = false;
    /// Vanilla `Piglin.getArmPose` `CROSSBOW_CHARGE` (`AnimationUtils.animateCrossbowCharge`, holding in
    /// the right arm): a regular piglin drawing its crossbow (the synced `IS_CHARGING_CROSSBOW` boolean,
    /// id 18) pulls it back — the right (holding) arm fixes at `yRot = -0.8` and the left (pulling) arm
    /// lerps from rest to full draw over [`crossbow_charge_ticks`](Self::crossbow_charge_ticks) `/ 25`.
    /// Higher priority than `CROSSBOW_HOLD`, lower than `ATTACKING`/`ADMIRING`/`DANCING`. `false` for every
    /// other entity and a piglin not mid-draw.
    (with_piglin_crossbow_charge) piglin_crossbow_charge: bool = false;
    /// Vanilla `PiglinModel` `ATTACKING_WITH_MELEE_WEAPON` (`PiglinModel.holdWeaponHigh` +
    /// `AnimationUtils.swingWeaponDown`, mainArm = RIGHT): a piglin or piglin brute that is aggressive and
    /// holds a melee weapon (a main-hand item carrying the `minecraft:tool` data component) raises the
    /// weapon overhead at rest ([`attack_anim`](Self::attack_anim) `== 0`) and chops it down across the
    /// swing (`attack_anim > 0`). `false` for every other entity, a dancing piglin, and a piglin holding a
    /// crossbow or no melee weapon. The zombified piglin instead does the deferred `animateZombieArms`.
    (with_piglin_attacking_with_melee) piglin_attacking_with_melee: bool = false;
    /// Vanilla `Piglin.getArmPose` `ADMIRING_ITEM` (`PiglinModel.setupAnim`): a regular piglin holding a
    /// piglin-loved item in its OFFHAND tilts its head down to look at it (`head.xRot = 0.5`) and lifts the
    /// off (left) arm up to show it (`leftArm.xRot = -0.9, yRot = 0.5`). `false` for every other entity, a
    /// dancing piglin, and a piglin with no loved offhand item.
    (with_piglin_admiring) piglin_admiring: bool = false;
    /// Vanilla `PandaRenderState.isUnhappy` (`Panda.getUnhappyCounter() > 0`): an unhappy panda whose
    /// `PandaModel.setupAnim` shakes its head (`yRot = zRot = 0.35·sin(0.6·ageInTicks)`) and paddles its
    /// front legs (`xRot = ∓0.75·sin(0.3·ageInTicks)`). `false` for every other entity and a content panda.
    (with_panda_unhappy) panda_unhappy: bool = false;
    /// Vanilla `PandaRenderState.isSneezing` (`Panda.isSneezing()`, the `DATA_ID_FLAGS` bit): a sneezing
    /// panda whose `PandaModel.setupAnim` dips its head ([`panda_sneeze_time`] driving the ramp). `false`
    /// for every other entity and a panda not mid-sneeze.
    (with_panda_sneezing) panda_sneezing: bool = false;
    /// Vanilla `PandaRenderState.sneezeTime` (`Panda.getSneezeCounter()`, the 0..20 ramp): drives the
    /// sneeze head dip (`head.xRot` falls to `-π/4` over ticks 0..14, then holds). `0` when not sneezing.
    (with_panda_sneeze_time) panda_sneeze_time: i32 = 0;
    /// Vanilla `PandaRenderState.isEating` (`Panda.isEating()`, synced `EAT_COUNTER > 0`): while a sitting
    /// panda holds a main-hand item, `PandaHoldsItemLayer` bobs the GROUND-context item toward the mouth.
    /// `false` for every other entity and a panda whose eat counter is zero.
    (with_panda_eating) panda_eating: bool = false;
    /// Vanilla `PandaRenderState.isScared` (`Panda.isWorried() && level.isThundering()`): scared sitting
    /// pandas suppress `PandaHoldsItemLayer` and use the scared sitting pose. `false` for every other
    /// entity, a non-worried panda, or a non-thundering level.
    (with_panda_scared) panda_scared: bool = false;
    /// Vanilla `PandaRenderState.isSitting` (`DATA_ID_FLAGS` bit `0x08`): the held item layer renders only
    /// while the panda is sitting and not scared. `false` for every other entity and a standing panda.
    (with_panda_sitting) panda_sitting: bool = false;
    /// Vanilla `PandaRenderState.sitAmount` (`Panda.getSitAmount(partialTick)`): the 0..1 eased sitting
    /// amount that `PandaRenderer.setupRotations` and `PandaModel.setupAnim` both consume.
    (with_panda_sit_amount) panda_sit_amount: f32 = 0.0;
    /// Vanilla `PandaRenderState.lieOnBackAmount` (`Panda.getLieOnBackAmount(partialTick)`): the eased
    /// blend for the lying-on-back root transform and limb/head pose.
    (with_panda_lie_on_back_amount) panda_lie_on_back_amount: f32 = 0.0;
    /// Vanilla `PandaRenderState.rollAmount` (`Panda.getRollAmount(partialTick)`, forced to `0.0` for
    /// baby pandas): the adult panda model rolling-pose blend.
    (with_panda_roll_amount) panda_roll_amount: f32 = 0.0;
    /// Vanilla `PandaRenderState.rollTime` (`rollCounter + partialTick` while rolling): drives the whole
    /// panda model tumble in `PandaRenderer.setupRotations`, including baby pandas.
    (with_panda_roll_time) panda_roll_time: f32 = 0.0;
    /// Vanilla `GoatRenderState.rammingXHeadRot` (`Goat.getRammingXHeadRot()`): the head-down tilt
    /// (radians) of a ramming goat, `lowerHeadTick/20 · (baby ? 52.5° : 30°)`. `GoatModel.setupAnim` SETs
    /// `head.xRot` to it while non-zero, overwriting the head-look pitch. `0.0` for a resting goat and
    /// every other entity.
    (with_goat_ramming_x_head_rot) goat_ramming_x_head_rot: f32 = 0.0;
    /// Vanilla `IronGolemRenderState.attackTicksRemaining` (the partial-lerped `attackAnimationTick`):
    /// while `> 0`, `IronGolemModel.setupAnim` raises both arms into the two-fisted smash
    /// (`xRot = -2 + 1.5·triangleWave(tick, 10)`), overriding the walk swing. `0.0` for a golem not
    /// attacking and every other entity.
    (with_iron_golem_attack_ticks_remaining) iron_golem_attack_ticks_remaining: f32 = 0.0;
    /// Vanilla `IronGolemRenderState.offerFlowerTick`: while `> 0` (and not attacking), the golem holds
    /// its right arm out with a poppy (`xRot = -0.8 + 0.025·triangleWave(tick, 70)`, left arm flat).
    /// `0` for a golem not offering and every other entity. The poppy block layer uses this same state
    /// to attach `Blocks.POPPY.defaultBlockState()` to the posed right arm.
    (with_iron_golem_offer_flower_tick) iron_golem_offer_flower_tick: i32 = 0;
    /// Vanilla `RavagerRenderState.stunnedTicksRemaining` (partial-lerped `stunnedTick`): while `> 0`,
    /// `RavagerModel.setupAnim` tilts the neck and shakes the head side-to-side (`neck.x =
    /// sin(stunned/40·10)·3`) with the jaw slightly open. `0.0` for a ravager not stunned and every other
    /// entity.
    (with_ravager_stunned_ticks_remaining) ravager_stunned_ticks_remaining: f32 = 0.0;
    /// Vanilla `RavagerRenderState.attackTicksRemaining` (partial-lerped `attackTick`): while `> 0`, the
    /// neck lunges forward (`neck.z = -6.5 + headPos`) and the mouth snaps open for the bite. `0.0` for a
    /// ravager not attacking and every other entity.
    (with_ravager_attack_ticks_remaining) ravager_attack_ticks_remaining: f32 = 0.0;
    /// Vanilla `RavagerRenderState.roarAnimation` (the `0..1` roar ramp): while `> 0` (and not stunned /
    /// attacking), the mouth gapes for the roar (`mouth.xRot = π/2·sin(roar·π/4)`). `0.0` for a ravager
    /// not roaring and every other entity.
    (with_ravager_roar_animation) ravager_roar_animation: f32 = 0.0;
    /// Vanilla `HoglinRenderState.attackAnimationRemainingTicks` (the RAW `0..10` headbutt timer): while
    /// active, `HoglinModel.setupAnim`'s `animateHeadbutt` raises the head from its rest down-tilt
    /// (`head.xRot = lerp(1 - |10 - 2·tick|/10, 0.87266463, -π/9)`, the baby also lifting `head.y`). `0`
    /// for a hoglin / zoglin not mid-headbutt and every other entity.
    (with_hoglin_attack_animation_tick) hoglin_attack_animation_tick: i32 = 0;
    /// Vanilla `LivingEntityRenderState.isCrouching` (`Pose.CROUCHING`): a sneaking player,
    /// whose `HumanoidModel.setupAnim` leans the body forward, drops the head, tucks the legs
    /// back and tilts the arms. `false` for every other entity and for a standing player.
    (with_is_crouching) is_crouching: bool = false;
    /// Vanilla `HumanoidRenderState.elytraRotX` from `LivingEntity.elytraAnimationState`. Defaults to
    /// `ElytraAnimationState`'s steady non-flying target (`PI/12`) until the native world projects the
    /// animated per-entity state.
    (with_elytra_rot_x) elytra_rot_x: f32 = ELYTRA_DEFAULT_X_ROT;
    /// Vanilla `HumanoidRenderState.elytraRotY`; steady non-flying target is `0`.
    (with_elytra_rot_y) elytra_rot_y: f32 = ELYTRA_DEFAULT_Y_ROT;
    /// Vanilla `HumanoidRenderState.elytraRotZ`; steady non-flying target is `-PI/12` and the right wing
    /// mirrors this value.
    (with_elytra_rot_z) elytra_rot_z: f32 = ELYTRA_DEFAULT_Z_ROT;
    /// Vanilla `LivingEntityRenderer.isBodyVisible`: a normally-invisible entity
    /// (Invisibility effect / `setInvisible`) draws no body and no layers for a
    /// non-spectator, non-glowing client. Both render paths skip the whole model
    /// when set. (The spectator-translucent and glowing-outline cases stay deferred.)
    (with_invisible) invisible: bool = false;
    /// Vanilla `WolfRenderState.tailAngle` (`Wolf.getTailAngle()`): the wolf tail's
    /// `xRot`. An angry wolf returns `1.5393804`; a tame wolf droops its tail with
    /// damage, `(0.55 - (maxHealth - health) / maxHealth * 0.4) * π` (tame `maxHealth`
    /// is the constant `40`); an untamed wolf returns the `π/5` default. Defaults to the
    /// `π/5` rest droop, matching the wolf tail layer's base pose, so a non-wolf or
    /// wild wolf is unaffected.
    (with_wolf_tail_angle) wolf_tail_angle: f32 = std::f32::consts::PI / 5.0;
    /// Vanilla `WolfRenderState.isSitting` (`Wolf.isInSittingPose()`): a sitting wolf
    /// folds its legs and tilts its body (`WolfModel.setSittingPose`) instead of swinging
    /// its legs. `false` for a standing wolf and every non-wolf entity.
    (with_wolf_sitting) wolf_sitting: bool = false;
    /// Vanilla `WolfRenderState.wetShade` (`Wolf.getWetShade(partialTick)`): the
    /// grayscale model tint multiplier `WolfRenderer.getModelTint` applies to the
    /// base wolf model only. `1.0` is dry/white; wet wolves start at `0.75` and
    /// brighten as the shake/drying timer advances.
    (with_wolf_wet_shade) wolf_wet_shade: f32 = 1.0;
    /// Vanilla `WolfRenderState.shakeAnim` (`Wolf.getShakeAnim(partialTick)`): the
    /// partial-lerped water-shake timer used by `getBodyRollAngle(offset)` to roll
    /// the body, adult mane, and tail while drying off. `0.0` for a dry wolf.
    (with_wolf_shake_anim) wolf_shake_anim: f32 = 0.0;
    /// Vanilla `WolfRenderState.headRollAngle` (`Wolf.getHeadRollAngle(partialTick)`): the
    /// interested/begging head tilt, added to the head water-shake roll in adult and baby wolf
    /// models. `0.0` for a level-headed wolf.
    (with_wolf_head_roll_angle) wolf_head_roll_angle: f32 = 0.0;
    /// Vanilla `ParrotRenderState.pose == SITTING` (`Parrot.isInSittingPose()`, the
    /// `TamableAnimal.DATA_FLAGS_ID` sitting bit): a perched parrot, whose
    /// `ParrotModel.prepare(SITTING)` raises every part `y += 1.9`, folds the legs
    /// (`xRot += π/2`), pitches the tail (`xRot += π/6`), and tucks the wings (`zRot = ±0.0873`).
    /// `false` for a standing parrot and every non-parrot entity.
    (with_parrot_sitting) parrot_sitting: bool = false;
    /// Vanilla `TurtleRenderState.hasEgg` (`!isBaby() && Turtle.hasEgg()`, the synced `HAS_EGG`
    /// boolean): a gravid adult turtle, whose `AdultTurtleModel.setupAnim` shows the `egg_belly`
    /// overlay cube and drops the whole model `root.y--` by one unit. `false` for a turtle
    /// without an egg, every baby turtle, and every non-turtle entity.
    (with_turtle_has_egg) turtle_has_egg: bool = false;
    /// Vanilla `TurtleRenderState.isLayingEgg` (the synced `Turtle.LAYING_EGG` boolean): a
    /// nesting turtle, whose shared `TurtleModel.setupAnim` quadruples the front legs' land yaw
    /// frequency (`layEgg = 4`) and doubles their amplitude (`layEggAmplitude = 2`) to mime
    /// digging. `false` for a turtle that is not laying and every non-turtle entity. Applies to
    /// adults and babies alike (the amplitude lives in the base model).
    (with_turtle_laying_egg) turtle_laying_egg: bool = false;
    /// Vanilla `EndCrystalRenderState.showsBottom` (the synced `EndCrystal.DATA_SHOW_BOTTOM`
    /// boolean, default `true`): `EndCrystalModel.setupAnim` sets `base.visible = showsBottom`, so
    /// the bottom slab is drawn when `true` and hidden when `false` (e.g. the four end-spike
    /// crystals that heal the dragon). Defaults `true` (vanilla default) for every non-crystal
    /// entity, where it is unused.
    (with_end_crystal_shows_bottom) end_crystal_shows_bottom: bool = true;
    /// Vanilla `EndCrystalRenderState.beamOffset`: target block center relative to the crystal position
    /// when `EndCrystal.DATA_BEAM_TARGET` is present. `None` for crystals without a beam target and
    /// every non-crystal entity.
    (with_end_crystal_beam) end_crystal_beam: Option<EndCrystalBeamRenderState> = None;
    /// Vanilla `EnderDragonRenderState.beamOffset`: nearest end-crystal position (including its
    /// `EndCrystalRenderer.getY` bob) relative to the dragon position. `None` for dragons without a
    /// tracked healing crystal and every non-dragon entity.
    (with_ender_dragon_beam) ender_dragon_beam: Option<EnderDragonBeamRenderState> = None;
    /// Vanilla `SquidRenderState.tentacleAngle` (`Mth.lerp(partialTick,
    /// oldTentacleAngle, tentacleAngle)`): the `xRot` `SquidModel.setupAnim` applies to
    /// all eight tentacles. `0.0` for a floating squid at rest and every non-squid
    /// entity.
    (with_squid_tentacle_angle) squid_tentacle_angle: f32 = 0.0;
    /// Vanilla `SquidRenderState.xBodyRot` (`Mth.lerp(partialTick, xBodyRotO,
    /// xBodyRot)`, degrees): the squid swim pitch `SquidRenderer.setupRotations` applies
    /// as `Axis.XP.rotationDegrees(xBodyRot)` after the body yaw. Tracks the movement
    /// direction while swimming and drifts toward `-90` while idle. `0.0` at rest and
    /// for every non-squid entity.
    () squid_x_body_rot: f32 = 0.0;
    /// Vanilla `SquidRenderState.zBodyRot` (`Mth.lerp(partialTick, zBodyRotO,
    /// zBodyRot)`, degrees): the squid swim roll `SquidRenderer.setupRotations` applies
    /// as `Axis.YP.rotationDegrees(zBodyRot)` after the pitch. Accumulates while
    /// swimming. `0.0` at rest and for every non-squid entity.
    () squid_z_body_rot: f32 = 0.0;
    /// Vanilla `GuardianRenderState.tailAnimation` (`Mth.lerp(partialTick,
    /// clientSideTailAnimationO, clientSideTailAnimation)`): the tail-sway phase
    /// `GuardianModel.setupAnim` feeds to the three tail segments' `yRot`
    /// (`sin(swim) * π * {0.05, 0.1, 0.15}`). `0.0` (tail at bind) for an
    /// unticked/out-of-water guardian and every non-guardian entity.
    (with_guardian_tail_animation) guardian_tail_animation: f32 = 0.0;
    /// Vanilla `GuardianRenderState.spikesAnimation` (`Guardian.getSpikesAnimation` lerped):
    /// `GuardianModel.setupAnim` turns it into `withdrawal = (1 - it) · 0.55`, the per-spike inset
    /// (spikes retract while swimming, extend while idle). `1.0` (withdrawal `0`, fully extended) for
    /// an unticked guardian and every non-guardian entity.
    (with_guardian_spikes_animation) guardian_spikes_animation: f32 = 1.0;
    /// Vanilla `Breeze`'s pose-driven action one-shots (elapsed seconds since each started, sampled by
    /// `BreezeModel.setupAnim` over the projected pose), applied additively to the body/head/rods. The
    /// non-looping actions clamp past their length to the final frame. `-1.0` (stopped) for a breeze
    /// not in that action and every non-breeze entity.
    (with_breeze_shoot_seconds) breeze_shoot_seconds: f32 = -1.0;
    (with_breeze_slide_seconds) breeze_slide_seconds: f32 = -1.0;
    (with_breeze_slide_back_seconds) breeze_slide_back_seconds: f32 = -1.0;
    (with_breeze_inhale_seconds) breeze_inhale_seconds: f32 = -1.0;
    (with_breeze_long_jump_seconds) breeze_long_jump_seconds: f32 = -1.0;
    /// Vanilla `GuardianRenderer` attack beam (`GuardianRenderState.attackTargetPosition` present): a
    /// guardian firing its beam, carrying the world-space eye→target vector, eye height, lerped
    /// `clientSideAttackTime`, and attack scale. `None` (no beam) for a guardian with no active attack
    /// target and every other entity.
    (with_guardian_beam) guardian_beam: Option<GuardianBeamRenderState> = None;
    /// Vanilla `ChickenRenderState.flap` (`Mth.lerp(partialTick, oFlap, flap)`): the
    /// wing-flap phase `ChickenModel.setupAnim` feeds to `flapAngle = (sin(flap) +
    /// 1) * flapSpeed`, written to `rightWing.zRot` / `-leftWing.zRot`. `0.0` for a
    /// still chicken and every non-chicken entity.
    (with_chicken_flap) chicken_flap: f32 = 0.0;
    /// Vanilla `ChickenRenderState.flapSpeed` (`Mth.lerp(partialTick, oFlapSpeed,
    /// flapSpeed)`): the wing-flap amplitude `ChickenModel.setupAnim` multiplies the
    /// flap phase by. `0.0` (wings held) for a grounded/still chicken and every
    /// non-chicken entity.
    (with_chicken_flap_speed) chicken_flap_speed: f32 = 0.0;
    /// Vanilla `SlimeRenderState.squish` (`Mth.lerp(partialTick, oSquish, squish)`):
    /// the squish amount `SlimeRenderer.scale` turns into the slime/magma-cube body's
    /// non-uniform stretch (`ss = squish / (size * 0.5 + 1)`, `w = 1 / (ss + 1)`,
    /// scale `[w, 1/w, w] * size`). `0.0` (undeformed cube) for a resting slime/magma
    /// cube and every other entity.
    (with_slime_squish) slime_squish: f32 = 0.0;
    /// Vanilla `EvokerFangsRenderState.biteProgress` (`EvokerFangs.getAnimationProgress`):
    /// the `0..1` attack ramp `EvokerFangsModel.setupAnim` turns into the jaw snap
    /// (`upper/lower_jaw.zRot`), the rise out of the ground (`base.y`), and the final
    /// vanish (`root.y` + `root` scale → 0). The renderer also hides the fang entirely
    /// while this is `0` (vanilla `EvokerFangsRenderer` skips rendering at
    /// `biteProgress == 0`). `0.0` (hidden) for an un-attacked fang and every other
    /// entity.
    (with_evoker_fangs_bite_progress) evoker_fangs_bite_progress: f32 = 0.0;
    /// Vanilla `AllayModel.setupAnim`: `true` while the allay's synced `DATA_DANCING`
    /// flag is set, gating the dance pose (head tilt + body sway/spin) over the normal
    /// head-look. `false` for a non-dancing allay and every other entity.
    (with_allay_dancing) allay_dancing: bool = false;
    /// Vanilla `AllayModel`: `true` during the spin sub-window of the dance
    /// (`danceAnimation % 55 < 15`), selecting the `4π * progress` body spin
    /// (`root.yRot`). `false` otherwise and for every non-allay entity.
    (with_allay_spinning) allay_spinning: bool = false;
    /// Vanilla `AllayModel`: the `0..1` lerped spin blend (`spinningAnimation / 15`)
    /// that cross-fades the body sway (`root.zRot`/`head` tilt scaled by `1 - progress`)
    /// into the spin. `0.0` for a non-spinning allay and every other entity.
    (with_allay_spinning_progress) allay_spinning_progress: f32 = 0.0;
    /// Vanilla `AxolotlRenderState.playingDeadFactor` (`Axolotl.playingDeadAnimator.getFactor`): the
    /// `0..1` eased blend `AdultAxolotlModel.setupPlayDeadAnimation` scales its limp-on-its-side pose
    /// by. `0.0` (awake) for every non-axolotl entity.
    (with_axolotl_playing_dead_factor) axolotl_playing_dead_factor: f32 = 0.0;
    /// Vanilla `AxolotlRenderState.inWaterFactor` (`Axolotl.inWaterAnimator.getFactor`): the `0..1`
    /// eased blend gating `setupSwimmingAnimation` / `setupWaterHoveringAnimation`. `0.0` (out of
    /// water) for every non-axolotl entity.
    (with_axolotl_in_water_factor) axolotl_in_water_factor: f32 = 0.0;
    /// Vanilla `AxolotlRenderState.onGroundFactor` (`Axolotl.onGroundAnimator.getFactor`): the `0..1`
    /// eased blend gating `setupGroundCrawlingAnimation` / `setupLayStillOnGroundAnimation`. `0.0`
    /// for every non-axolotl entity.
    (with_axolotl_on_ground_factor) axolotl_on_ground_factor: f32 = 0.0;
    /// Vanilla `AxolotlRenderState.movingFactor` (`Axolotl.movingAnimator.getFactor`): the `0..1`
    /// eased blend separating the moving sub-animations (swim, crawl) from the still ones (hover,
    /// lay-still) and gating `applyMirrorLegRotations`. `0.0` (still) for every non-axolotl entity.
    (with_axolotl_moving_factor) axolotl_moving_factor: f32 = 0.0;
    /// Vanilla `ParrotRenderState.flapAngle` (`ParrotRenderer.extractRenderState`:
    /// `(Mth.sin(lerp(oFlap, flap)) + 1) * lerp(oFlapSpeed, flapSpeed)`): the combined
    /// wing-flap angle `ParrotModel.setupAnim` writes to the wings (`leftWing.zRot =
    /// -0.0873 - flapAngle`, `rightWing.zRot = 0.0873 + flapAngle`) and the
    /// body/head/tail/wing/leg bob (`y += flapAngle * 0.3`) in the STANDING/FLYING
    /// branches. `0.0` (wings held) for a grounded/still parrot and every non-parrot
    /// entity.
    (with_parrot_flap_angle) parrot_flap_angle: f32 = 0.0;
    /// Vanilla `LivingEntityRenderState.isInWater` (`entity.isInWaterOrBubble()`): a fish
    /// out of water thrashes harder and flops onto its side. `CodModel.setupAnim` scales
    /// its tail sway by `1.0` in water / `1.5` out, and `CodRenderer.setupRotations` adds
    /// the beached `RotZ(90)` flop when `false`. `false` (the Java default) for every
    /// entity until the entity scene projects `entity.isInWater()`.
    (with_in_water) in_water: bool = false;
    /// Vanilla `Entity.onGround()`: combined with [`in_water`](Self::in_water) to drive the
    /// vanilla `TurtleRenderer` `isOnLand = !isInWater && onGround` walk/swim leg branch.
    /// `false` (the Java default) for every entity until the entity scene projects it.
    (with_on_ground) on_ground: bool = false;
    /// Vanilla `DolphinRenderState.isMoving` (`getDeltaMovement().horizontalDistanceSqr() >
    /// 1e-7`): drives the `DolphinModel.setupAnim` swim body tilt / tail wave. `false` for a
    /// stationary entity until the entity scene projects it.
    (with_is_moving) is_moving: bool = false;
}

impl EntityRenderState {
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

    /// Sets the head-look projection (vanilla `LivingEntityRenderState.yRot` /
    /// `.xRot`, both in degrees): the net head yaw relative to the body and the
    /// head pitch. Consumed by model families with a head part (currently the
    /// sheep `QuadrupedModel`).
    pub fn with_head_look(mut self, head_yaw: f32, head_pitch: f32) -> Self {
        self.render_state.head_yaw = head_yaw;
        self.render_state.head_pitch = head_pitch;
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

    pub fn with_squid_body_tilt(mut self, x_body_rot: f32, z_body_rot: f32) -> Self {
        self.render_state.squid_x_body_rot = x_body_rot;
        self.render_state.squid_z_body_rot = z_body_rot;
        self
    }

    pub fn cod(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Cod, position, y_rot)
    }

    pub fn salmon(entity_id: i32, position: [f32; 3], y_rot: f32, size: SalmonModelSize) -> Self {
        Self::new(entity_id, EntityModelKind::Salmon { size }, position, y_rot)
    }

    pub fn tropical_fish(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        shape: TropicalFishModelShape,
        base_color: EntityDyeColor,
        pattern: TropicalFishPattern,
        pattern_color: EntityDyeColor,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::TropicalFish {
                shape,
                base_color,
                pattern,
                pattern_color,
            },
            position,
            y_rot,
        )
    }

    pub fn squid(entity_id: i32, position: [f32; 3], y_rot: f32, glow: bool, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Squid { glow, baby },
            position,
            y_rot,
        )
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
        Self::player_with_skin(
            entity_id,
            position,
            y_rot,
            EntityPlayerSkin::default_for_model(slim),
            parts,
        )
    }

    pub fn player_with_skin(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        skin: EntityPlayerSkin,
        parts: PlayerModelPartVisibility,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Player { skin, parts },
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

    pub fn ghast(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Ghast { charging: false },
            position,
            y_rot,
        )
    }

    pub fn happy_ghast(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::HappyGhast, position, y_rot)
    }

    pub fn minecart(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Minecart, position, y_rot)
    }

    pub fn blaze(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Blaze, position, y_rot)
    }

    pub fn endermite(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Endermite, position, y_rot)
    }

    pub fn silverfish(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Silverfish, position, y_rot)
    }

    pub fn vex(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Vex { charging: false },
            position,
            y_rot,
        )
    }

    pub fn allay(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Allay, position, y_rot)
    }

    pub fn strider(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool, cold: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Strider { baby, cold },
            position,
            y_rot,
        )
    }

    pub fn turtle(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(entity_id, EntityModelKind::Turtle { baby }, position, y_rot)
    }

    pub fn bat(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Bat, position, y_rot)
    }

    pub fn bee(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Bee {
                baby,
                angry: false,
                has_nectar: false,
            },
            position,
            y_rot,
        )
    }

    pub fn breeze(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Breeze, position, y_rot)
    }

    pub fn dolphin(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Dolphin { baby },
            position,
            y_rot,
        )
    }

    pub fn guardian(entity_id: i32, position: [f32; 3], y_rot: f32, elder: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Guardian { elder },
            position,
            y_rot,
        )
    }

    pub fn frog(entity_id: i32, position: [f32; 3], y_rot: f32, variant: FrogModelVariant) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Frog { variant },
            position,
            y_rot,
        )
    }

    pub fn creaking(entity_id: i32, position: [f32; 3], y_rot: f32, eyes_glowing: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Creaking { eyes_glowing },
            position,
            y_rot,
        )
    }

    pub fn sniffer(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Sniffer, position, y_rot)
    }

    pub fn warden(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Warden, position, y_rot)
    }

    pub fn armadillo(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        rolled_up: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Armadillo { baby, rolled_up },
            position,
            y_rot,
        )
    }

    pub fn axolotl(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: AxolotlModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Axolotl { baby, variant },
            position,
            y_rot,
        )
    }

    pub fn tadpole(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Tadpole, position, y_rot)
    }

    pub fn parrot(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        variant: ParrotModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Parrot { variant },
            position,
            y_rot,
        )
    }

    pub fn shulker(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        color: Option<EntityDyeColor>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Shulker { color },
            position,
            y_rot,
        )
    }

    pub fn wither(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Wither, position, y_rot)
    }

    pub fn giant(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Giant, position, y_rot)
    }

    pub fn end_crystal(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::EndCrystal, position, y_rot)
    }

    pub fn evoker_fangs(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::EvokerFangs, position, y_rot)
    }

    pub fn leash_knot(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::LeashKnot, position, y_rot)
    }

    pub fn arrow(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        texture: ArrowModelTexture,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Arrow { texture },
            position,
            y_rot,
        )
    }

    pub fn trident(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::Trident, position, y_rot)
    }

    pub fn wither_skull(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::wither_skull_with_dangerous(entity_id, position, y_rot, false)
    }

    pub fn wither_skull_with_dangerous(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        dangerous: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::WitherSkull { dangerous },
            position,
            y_rot,
        )
    }

    pub fn llama_spit(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::LlamaSpit, position, y_rot)
    }

    pub fn shulker_bullet(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::ShulkerBullet, position, y_rot)
    }

    pub fn wind_charge(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::WindCharge, position, y_rot)
    }

    pub fn ender_dragon(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::EnderDragon, position, y_rot)
    }

    pub fn no_render(entity_id: i32, position: [f32; 3], y_rot: f32) -> Self {
        Self::new(entity_id, EntityModelKind::NoRender, position, y_rot)
    }

    pub fn phantom(entity_id: i32, position: [f32; 3], y_rot: f32, size: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Phantom { size },
            position,
            y_rot,
        )
    }

    pub fn pufferfish(entity_id: i32, position: [f32; 3], y_rot: f32, puff_state: i32) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Pufferfish { puff_state },
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

    pub fn mooshroom(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Mooshroom {
                baby,
                variant: MooshroomVariant::Red,
            },
            position,
            y_rot,
        )
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
                jeb,
                age_ticks,
            },
            position,
            y_rot,
        )
        .with_invisible(invisible)
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
                collar_color: tame.then_some(collar_color).flatten(),
                variant: WolfModelVariant::Pale,
            },
            position,
            y_rot,
        )
        .with_invisible(invisible)
    }

    pub fn horse(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::horse_with_variant(entity_id, position, y_rot, baby, HorseColorVariant::White)
    }

    pub fn horse_with_variant(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: HorseColorVariant,
    ) -> Self {
        Self::horse_full(
            entity_id,
            position,
            y_rot,
            baby,
            variant,
            HorseMarkings::None,
        )
    }

    pub fn horse_full(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: HorseColorVariant,
        markings: HorseMarkings,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Horse {
                baby,
                variant,
                markings,
            },
            position,
            y_rot,
        )
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

    pub fn rabbit(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: RabbitModelVariant,
        toast: bool,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Rabbit {
                baby,
                variant,
                toast,
            },
            position,
            y_rot,
        )
    }

    pub fn panda(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: PandaModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Panda { baby, variant },
            position,
            y_rot,
        )
    }

    pub fn feline(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        cat: bool,
        baby: bool,
        cat_variant: CatModelVariant,
        collar: Option<EntityDyeColor>,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Feline {
                cat,
                baby,
                cat_variant,
                collar,
            },
            position,
            y_rot,
        )
    }

    pub fn fox(
        entity_id: i32,
        position: [f32; 3],
        y_rot: f32,
        baby: bool,
        variant: FoxModelVariant,
    ) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Fox { baby, variant },
            position,
            y_rot,
        )
    }

    pub fn nautilus(entity_id: i32, position: [f32; 3], y_rot: f32, baby: bool) -> Self {
        Self::new(
            entity_id,
            EntityModelKind::Nautilus { baby },
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
        Self::new(
            entity_id,
            EntityModelKind::IronGolem {
                crackiness: IronGolemCrackiness::None,
            },
            position,
            y_rot,
        )
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
                collar_color: None,
                variant: WolfModelVariant::Pale,
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
                arrow_shake: 0.0,
                head_eat: SheepHeadEatPose::NONE,
                polar_bear_stand_scale: 0.0,
                death_time: 0.0,
                light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
                has_red_overlay: false,
                white_overlay_progress: 0.0,
                creeper_swelling: 0.0,
                creeper_powered: false,
                villager_model_data: VillagerModelData::DEFAULT,
                shulker_peek: 0.0,
                shulker_attach_face: EntityAttachmentFace::Down,
                tendril_animation: 0.0,
                heart_animation: 0.0,
                warden_roar_seconds: -1.0,
                warden_sniff_seconds: -1.0,
                warden_attack_seconds: -1.0,
                warden_sonic_boom_seconds: -1.0,
                warden_emerge_seconds: -1.0,
                warden_dig_seconds: -1.0,
                rabbit_hop_seconds: -1.0,
                creaking_can_move: true,
                creaking_attack_seconds: -1.0,
                creaking_invulnerable_seconds: -1.0,
                creaking_death_seconds: -1.0,
                auto_spin_age_ticks: None,
                upside_down_height: None,
                bounding_box_height: 0.0,
                sleeping: None,
                scale: 1.0,
                swim_amount: 0.0,
                walk_animation_pos: 0.0,
                walk_animation_speed: 0.0,
                worn_head_animation_pos: 0.0,
                is_riding: false,
                attack_anim: 0.0,
                attack_arm_off_hand: false,
                age_in_ticks: 0.0,
                is_aggressive: false,
                main_hand_holds_bow: false,
                main_hand_swing_is_stab: false,
                player_using_spyglass: false,
                player_tooting_horn: false,
                player_brushing: false,
                player_main_hand_item_pose: false,
                player_off_hand_item_pose: false,
                player_cape_texture: None,
                player_elytra_texture: None,
                chest_wings_layer: None,
                chest_equipment_has_wings: false,
                chest_equipment_has_humanoid: false,
                player_cape_flap: 0.0,
                player_cape_lean: 0.0,
                player_cape_lean2: 0.0,
                player_blocking: false,
                player_throwing_trident: false,
                player_drawing_bow: false,
                player_charging_crossbow: false,
                player_crossbow_hold: false,
                player_crossbow_hold_off_hand: false,
                use_item_off_hand: false,
                main_hand_holds_crossbow: false,
                drowned_throw_trident: false,
                is_charging_crossbow: false,
                crossbow_charge_ticks: 0.0,
                enderman_carrying: false,
                enderman_creepy: false,
                bat_resting: false,
                bee_has_stinger: true,
                bee_angry: false,
                bee_roll_amount: 0.0,
                camel_sit_seconds: -1.0,
                camel_sit_pose_seconds: -1.0,
                camel_standup_seconds: -1.0,
                camel_dash_seconds: -1.0,
                frog_croak_seconds: -1.0,
                frog_tongue_seconds: -1.0,
                frog_jump_seconds: -1.0,
                frog_swim_idle_seconds: -1.0,
                sniffer_animation_id: -1,
                sniffer_animation_seconds: -1.0,
                sniffer_is_searching: false,
                armadillo_is_hiding_in_shell: false,
                armadillo_roll_up_seconds: -1.0,
                armadillo_roll_out_seconds: -1.0,
                armadillo_peek_seconds: -1.0,
                fox_head_roll_angle: 0.0,
                fox_crouch_amount: 0.0,
                fox_is_crouching: false,
                fox_is_sleeping: false,
                fox_is_sitting: false,
                fox_is_pouncing: false,
                fox_is_faceplanted: false,
                witch_holding_item: false,
                witch_holding_potion: false,
                copper_golem_holding_item: false,
                custom_head_skull: None,
                vex_charging: false,
                vex_right_hand_item_non_empty: false,
                vex_left_hand_item_non_empty: false,
                wither_invulnerable_ticks: 0.0,
                wither_x_head_rots: [0.0; 2],
                wither_y_head_rots: [0.0; 2],
                wither_powered: false,
                head_armor: None,
                chest_armor: None,
                legs_armor: None,
                feet_armor: None,
                head_armor_dye: None,
                chest_armor_dye: None,
                legs_armor_dye: None,
                feet_armor_dye: None,
                pig_saddle: false,
                equine_saddle: false,
                equine_saddle_ridden: false,
                equine_body_armor: None,
                equine_body_armor_dye: None,
                wolf_body_armor: None,
                wolf_body_armor_dye: None,
                wolf_body_armor_crackiness: None,
                strider_ridden: false,
                strider_saddle: false,
                camel_saddle: false,
                camel_saddle_ridden: false,
                nautilus_saddle: false,
                nautilus_body_armor: None,
                snow_golem_pumpkin: false,
                llama_body_decor: None,
                illager_spellcasting: false,
                illager_celebrating: false,
                illager_main_hand_empty: false,
                piglin_dancing: false,
                piglin_crossbow_hold: false,
                piglin_crossbow_charge: false,
                piglin_attacking_with_melee: false,
                piglin_admiring: false,
                panda_unhappy: false,
                panda_sneezing: false,
                panda_sneeze_time: 0,
                panda_eating: false,
                panda_scared: false,
                panda_sitting: false,
                panda_sit_amount: 0.0,
                panda_lie_on_back_amount: 0.0,
                panda_roll_amount: 0.0,
                panda_roll_time: 0.0,
                goat_ramming_x_head_rot: 0.0,
                iron_golem_attack_ticks_remaining: 0.0,
                iron_golem_offer_flower_tick: 0,
                ravager_stunned_ticks_remaining: 0.0,
                ravager_attack_ticks_remaining: 0.0,
                ravager_roar_animation: 0.0,
                hoglin_attack_animation_tick: 0,
                is_crouching: false,
                elytra_rot_x: ELYTRA_DEFAULT_X_ROT,
                elytra_rot_y: ELYTRA_DEFAULT_Y_ROT,
                elytra_rot_z: ELYTRA_DEFAULT_Z_ROT,
                invisible: false,
                wolf_tail_angle: std::f32::consts::PI / 5.0,
                wolf_sitting: false,
                wolf_wet_shade: 1.0,
                wolf_shake_anim: 0.0,
                wolf_head_roll_angle: 0.0,
                parrot_sitting: false,
                turtle_has_egg: false,
                turtle_laying_egg: false,
                end_crystal_shows_bottom: true,
                end_crystal_beam: None,
                ender_dragon_beam: None,
                squid_tentacle_angle: 0.0,
                squid_x_body_rot: 0.0,
                squid_z_body_rot: 0.0,
                guardian_tail_animation: 0.0,
                guardian_spikes_animation: 1.0,
                breeze_shoot_seconds: -1.0,
                breeze_slide_seconds: -1.0,
                breeze_slide_back_seconds: -1.0,
                breeze_inhale_seconds: -1.0,
                breeze_long_jump_seconds: -1.0,
                guardian_beam: None,
                chicken_flap: 0.0,
                chicken_flap_speed: 0.0,
                slime_squish: 0.0,
                evoker_fangs_bite_progress: 0.0,
                allay_dancing: false,
                allay_spinning: false,
                allay_spinning_progress: 0.0,
                axolotl_playing_dead_factor: 0.0,
                axolotl_in_water_factor: 0.0,
                axolotl_on_ground_factor: 0.0,
                axolotl_moving_factor: 0.0,
                parrot_flap_angle: 0.0,
                in_water: false,
                on_ground: false,
                is_moving: false,
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
