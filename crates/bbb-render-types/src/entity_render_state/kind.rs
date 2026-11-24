//! Kind-specific entity render state: one variant per Vanilla render-state family, filled once
//! by the world projection and read by that family's renderer.

use crate::{EntityArmorTrim, EntityDynamicPlayerTexture, EntityEquipmentAssetId};

use super::{
    EndCrystalBeamRenderState, EnderDragonAnimationRenderState, EnderDragonBeamRenderState,
    EntityAttachmentFace, GuardianBeamRenderState, GuardianEyeTrackingRenderState,
    ParrotModelVariant, SheepHeadEatPose, SpearKineticWeapon, VillagerModelData,
    WolfArmorCrackiness,
};

/// Declares [`EntityKindRenderState`] with, per variant, a getter returning the variant's state
/// (its defaults for every other kind) and a `_mut` accessor that switches to the variant.
macro_rules! entity_kind_render_states {
    ($($variant:ident($state:ident) => $get:ident, $get_mut:ident;)*) => {
        /// The kind-specific part of [`super::EntityRenderState`].
        #[derive(Debug, Clone, Copy, Default, PartialEq)]
        #[allow(clippy::large_enum_variant)]
        pub enum EntityKindRenderState {
            #[default]
            Other,
            $($variant($state),)*
        }

        impl EntityKindRenderState {
            $(
                pub fn $get(&self) -> &$state {
                    match self {
                        Self::$variant(state) => state,
                        _ => &$state::DEFAULT,
                    }
                }

                pub fn $get_mut(&mut self) -> &mut $state {
                    if !matches!(self, Self::$variant(_)) {
                        *self = Self::$variant($state::DEFAULT);
                    }
                    match self {
                        Self::$variant(state) => state,
                        _ => unreachable!(),
                    }
                }
            )*
        }
    };
}

render_state_struct! {
    /// Vanilla `AbstractBoatRenderState` / `MinecartRenderState` hurt wobble (`hurtTime`,
    /// `hurtDir`, `damageTime`), shared by boats and minecarts.
    pub struct VehicleHurtRenderState {
        pub time: f32 = 0.0,
        pub dir: i32 = 1,
        pub damage_time: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct ArrowRenderState {
        /// Vanilla `ArrowRenderState.shake` (`AbstractArrow.shakeTime - partialTick`).
        pub shake: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct TridentRenderState {
        /// Vanilla `ThrownTridentRenderState.isFoil`.
        pub foil: bool = false,
    }
}

render_state_struct! {
    pub struct FishingHookRenderState {
        /// Vanilla `FishingHookRenderState.lineOriginOffset`.
        pub line_origin_offset: Option<[f32; 3]> = None,
    }
}

render_state_struct! {
    pub struct SheepRenderState {
        /// Per-frame sheep eat-grass head pose (`Sheep.getHeadEatPositionScale` /
        /// `getHeadEatAngleScale`). [`SheepHeadEatPose::NONE`] for every non-sheep entity and for a
        /// sheep that is not currently eating.
        pub head_eat: SheepHeadEatPose = SheepHeadEatPose::NONE,
    }
}

render_state_struct! {
    pub struct PolarBearRenderState {
        /// Per-frame polar bear standing-rear scale (`PolarBear.getStandingAnimationScale`,
        /// `0.0..=1.0`).
        pub stand_scale: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct CreeperRenderState {
        /// Vanilla `CreeperRenderState.swelling` (`Creeper.getSwelling`, lerped).
        pub swelling: f32 = 0.0,
        /// Vanilla `CreeperRenderState.isPowered` (`Creeper.isPowered`, the synced
        /// `DATA_IS_POWERED`).
        pub powered: bool = false,
    }
}

render_state_struct! {
    pub struct VillagerRenderState {
        /// Vanilla `VillagerDataHolderRenderState.getVillagerData()`.
        pub model_data: VillagerModelData = VillagerModelData::DEFAULT,
        /// Vanilla `VillagerRenderState.isUnhappy` (`AbstractVillager.getUnhappyCounter() > 0`).
        pub unhappy: bool = false,
    }
}

render_state_struct! {
    pub struct ShulkerRenderState {
        /// Vanilla `ShulkerRenderState.peekAmount` (`Shulker.getClientPeekAmount`, lerped).
        pub peek: f32 = 0.0,
        /// Vanilla `ShulkerRenderState.attachFace`.
        pub attach_face: EntityAttachmentFace = EntityAttachmentFace::Down,
    }
}

render_state_struct! {
    pub struct WardenRenderState {
        /// Vanilla `WardenRenderState.tendrilAnimation` (`Warden.getTendrilAnimation`, lerped).
        pub tendril_animation: f32 = 0.0,
        /// Vanilla `WardenRenderState.heartAnimation` (`Warden.getHeartAnimation`, lerped).
        pub heart_animation: f32 = 0.0,
        /// Vanilla `Warden.roarAnimationState` elapsed seconds (`Pose.ROARING`-driven, the 4.2s
        /// `WARDEN_ROAR`).
        pub roar_seconds: f32 = -1.0,
        /// Vanilla `Warden.sniffAnimationState` elapsed seconds (`Pose.SNIFFING`-driven, the 4.16s
        /// `WARDEN_SNIFF`), sampled by `WardenModel.setupAnim`.
        pub sniff_seconds: f32 = -1.0,
        /// Vanilla `Warden.attackAnimationState` elapsed seconds (entity event `4`, the 0.33333s
        /// `WARDEN_ATTACK`), sampled by `WardenModel.setupAnim`.
        pub attack_seconds: f32 = -1.0,
        /// Vanilla `Warden.sonicBoomAnimationState` elapsed seconds (entity event `62`, the 3.0s
        /// `WARDEN_SONIC_BOOM`), sampled by `WardenModel.setupAnim`.
        pub sonic_boom_seconds: f32 = -1.0,
        /// Vanilla `Warden.emergeAnimationState` elapsed seconds (`Pose.EMERGING`-driven, the 6.68s
        /// `WARDEN_EMERGE` spawn rise), sampled by `WardenModel.setupAnim`.
        pub emerge_seconds: f32 = -1.0,
        /// Vanilla `Warden.diggingAnimationState` elapsed seconds (`Pose.DIGGING`-driven, the 5.0s
        /// `WARDEN_DIG` despawn burrow), sampled by `WardenModel.setupAnim`.
        pub dig_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct RabbitRenderState {
        /// Vanilla `Rabbit.hopAnimationState` elapsed seconds (entity event `1`-seeded, the 0.75s
        /// looping `RabbitAnimation.HOP`), sampled by `RabbitModel.setupAnim`.
        pub hop_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct CreakingRenderState {
        /// Vanilla `Creaking.canMove()` (synced `CAN_MOVE`, default `true`).
        pub can_move: bool = true,
        /// Vanilla `Creaking.attackAnimationState` elapsed seconds (entity event `4`-seeded, the
        /// 0.7083s looping `CREAKING_ATTACK` lunge), sampled by `CreakingModel.setupAnim`.
        pub attack_seconds: f32 = -1.0,
        /// Vanilla `Creaking.invulnerabilityAnimationState` elapsed seconds (entity event
        /// `66`-seeded, the 0.2917s `CREAKING_INVULNERABLE` stagger), sampled by
        /// `CreakingModel.setupAnim`.
        pub invulnerable_seconds: f32 = -1.0,
        /// Vanilla `Creaking.deathAnimationState` elapsed seconds (synced `isTearingDown()`-driven,
        /// the 2.25s `CREAKING_DEATH` collapse), sampled by `CreakingModel.setupAnim`.
        pub death_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct BoatRenderState {
        pub hurt: VehicleHurtRenderState = VehicleHurtRenderState::DEFAULT,
        /// Vanilla `BoatRenderState.rowingTimeLeft`.
        pub rowing_time_left: f32 = 0.0,
        /// Vanilla `BoatRenderState.rowingTimeRight`.
        pub rowing_time_right: f32 = 0.0,
        /// Vanilla `BoatRenderState.bubbleAngle`.
        pub bubble_angle: f32 = 0.0,
        /// Vanilla `BoatRenderState.isUnderWater`.
        pub underwater: bool = false,
    }
}

render_state_struct! {
    pub struct MinecartRenderState {
        pub hurt: VehicleHurtRenderState = VehicleHurtRenderState::DEFAULT,
        /// Vanilla `MinecartRenderState.isNewRender`.
        pub new_render: bool = false,
        /// Vanilla `MinecartRenderState.posOnRail`, set by old-render carts when
        /// `OldMinecartBehavior.getPos` resolves the current rail block.
        pub pos_on_rail: Option<[f32; 3]> = None,
        /// Vanilla `MinecartRenderState.frontPos`, from `OldMinecartBehavior.getPosOffs(...,
        /// 0.3F)`.
        pub front_pos: Option<[f32; 3]> = None,
        /// Vanilla `MinecartRenderState.backPos`, from `OldMinecartBehavior.getPosOffs(...,
        /// -0.3F)`.
        pub back_pos: Option<[f32; 3]> = None,
        /// `posOnRail` / `(frontPos.y + backPos.y) * 0.5` offset relative to `entity.position`,
        /// subtracted in `f64` before casting to `f32` so large world coordinates do not jitter.
        pub rail_offset: Option<[f32; 3]> = None,
        /// `backPos - frontPos` subtracted in `f64` before casting to `f32`.
        pub rail_direction: Option<[f32; 3]> = None,
        /// Vanilla `MinecartTntRenderState.fuseRemainingInTicks`.
        pub tnt_fuse_remaining_in_ticks: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct PlayerRenderState {
        /// Vanilla `HumanoidModel.ArmPose.SPEAR` while using a spear.
        pub using_spear: Option<SpearKineticWeapon> = None,
        /// Vanilla `AvatarRenderer.getArmPose` `SPEAR` for a non-using main-hand spear.
        pub main_hand_spear_pose: bool = false,
        /// Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` `SPEAR` for a non-using off-hand spear.
        pub off_hand_spear_pose: bool = false,
        /// Vanilla `HumanoidModel.setupAnim` use-item arm pose `SPYGLASS`
        /// (`ItemStack.getUseAnimation() == SPYGLASS`).
        pub using_spyglass: bool = false,
        /// Vanilla `HumanoidModel.setupAnim` use-item arm pose `TOOT_HORN`
        /// (`ItemStack.getUseAnimation() == TOOT_HORN`).
        pub tooting_horn: bool = false,
        /// Vanilla `HumanoidModel.setupAnim` use-item arm pose `BRUSH`
        /// (`ItemStack.getUseAnimation() == BRUSH`).
        pub brushing: bool = false,
        /// Vanilla `AvatarRenderer.getArmPose` fallback `ITEM` arm pose
        /// (`HumanoidModel.poseRightArm` ITEM case).
        pub main_hand_item_pose: bool = false,
        /// Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM` arm pose
        /// (`HumanoidModel.poseLeftArm` ITEM case).
        pub off_hand_item_pose: bool = false,
        /// Vanilla `AvatarRenderState.skin.cape()` as a downloaded profile texture.
        pub cape_texture: Option<EntityDynamicPlayerTexture> = None,
        /// Vanilla `AvatarRenderState.skin.elytra()` as a downloaded profile texture.
        pub elytra_texture: Option<EntityDynamicPlayerTexture> = None,
        /// Vanilla `AvatarRenderState.parrotOnLeftShoulder`, projected from
        /// `Player.DATA_SHOULDER_PARROT_LEFT`.
        pub left_shoulder_parrot: Option<ParrotModelVariant> = None,
        /// Vanilla `AvatarRenderState.parrotOnRightShoulder`, projected from
        /// `Player.DATA_SHOULDER_PARROT_RIGHT`.
        pub right_shoulder_parrot: Option<ParrotModelVariant> = None,
        /// Vanilla `AvatarRenderState.arrowCount`.
        pub arrow_count: i32 = 0,
        /// Vanilla `AvatarRenderState.stingerCount`.
        pub stinger_count: i32 = 0,
        /// Vanilla `AvatarRenderState.showExtraEars`.
        pub show_extra_ears: bool = false,
        /// Vanilla `AvatarRenderState.capeFlap`, produced by `AvatarRenderer.extractCapeState` from
        /// the world-projected cloak interpolation.
        pub cape_flap: f32 = 0.0,
        /// Vanilla `AvatarRenderState.capeLean`, produced by `AvatarRenderer.extractCapeState`.
        pub cape_lean: f32 = 0.0,
        /// Vanilla `AvatarRenderState.capeLean2`, produced by `AvatarRenderer.extractCapeState`.
        pub cape_lean2: f32 = 0.0,
        /// Vanilla `AvatarRenderState.fallFlyingTimeInTicks`, including the render partial tick,
        /// used by `AvatarRenderer.setupRotations`' pitch ramp.
        pub fall_flying_time_in_ticks: f32 = 0.0,
        /// Vanilla `AvatarRenderState.shouldApplyFlyingYRot` flight-data threshold.
        pub should_apply_flying_y_rot: bool = false,
        /// Vanilla `AvatarRenderState.flyingYRot`, the signed horizontal movement/look angle in
        /// radians.
        pub flying_y_rot: f32 = 0.0,
        /// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` use-item arm pose `BLOCK`
        /// (`poseBlockingArm`).
        pub blocking: bool = false,
        /// Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` use-item arm pose `THROW_TRIDENT`.
        pub throwing_trident: bool = false,
        /// Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` use-item arm pose `BOW_AND_ARROW`.
        pub drawing_bow: bool = false,
        /// Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` use-item arm pose `CROSSBOW_CHARGE`
        /// (`AnimationUtils.animateCrossbowCharge`).
        pub charging_crossbow: bool = false,
        /// Vanilla `AvatarRenderer.getArmPose` `CROSSBOW_HOLD`
        /// (`AnimationUtils.animateCrossbowHold`).
        pub crossbow_hold: bool = false,
        /// Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` `CROSSBOW_HOLD`.
        pub crossbow_hold_off_hand: bool = false,
    }
}

render_state_struct! {
    pub struct IllagerRenderState {
        /// Vanilla `Pillager.isHolding(Items.CROSSBOW)`.
        pub holds_crossbow: bool = false,
        /// Vanilla `Pillager.isChargingCrossbow()` (the synced `IS_CHARGING_CROSSBOW` boolean, id
        /// 17).
        pub is_charging_crossbow: bool = false,
        /// Vanilla `IllagerRenderState.armPose == SPELLCASTING`
        /// (`SpellcasterIllager.isCastingSpell()`, the synced `DATA_SPELL_CASTING_ID` byte > 0).
        pub spellcasting: bool = false,
        /// Vanilla `Raider.isCelebrating()` (the synced `IS_CELEBRATING` boolean, id 16).
        pub celebrating: bool = false,
        /// Vanilla `IllagerModel.setupAnim` `ATTACKING` branch checks whether the rendered
        /// main-hand item is empty.
        pub main_hand_empty: bool = false,
        /// Vanilla `IllusionerRenderState.illusionOffsets`, the four render-only body offsets that
        /// `IllusionerRenderer.submit` loops over while `state.isInvisible`.
        pub clone_offsets: [[f32; 3]; 4] = [[0.0; 3]; 4],
    }
}

render_state_struct! {
    pub struct DrownedRenderState {
        /// Vanilla `DrownedRenderer.getArmPose` `THROW_TRIDENT`.
        pub throw_trident: bool = false,
    }
}

render_state_struct! {
    pub struct EndermanRenderState {
        /// Vanilla `EndermanRenderState.carriedBlock` non-empty.
        pub carrying: bool = false,
        /// Vanilla `EndermanRenderState.isCreepy`.
        pub creepy: bool = false,
    }
}

render_state_struct! {
    pub struct BatRenderState {
        /// Vanilla `BatRenderState.isResting`.
        pub resting: bool = false,
        /// Vanilla `BatRenderState.flyAnimationState`.
        pub flying_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct BeeRenderState {
        /// Vanilla `BeeRenderState.hasStinger` (`!Bee.hasStung()`).
        pub has_stinger: bool = true,
        /// Vanilla `BeeRenderState.isAngry` (`Bee.isAngry()`).
        pub angry: bool = false,
        /// Vanilla `BeeRenderState.rollAmount` (`Bee.getRollAmount(partialTick)`).
        pub roll_amount: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct CamelRenderState {
        /// Vanilla `Camel.sitAnimationState` elapsed seconds (the 2.0 s `CAMEL_SIT`, non-looping),
        /// driven by `Camel.setupAnimationStates()` while the camel is visually sitting AND inside
        /// the 40-tick sit-down window (`isVisuallySittingDown()`).
        pub sit_seconds: f32 = -1.0,
        /// Vanilla `Camel.sitPoseAnimationState` elapsed seconds (the 1.0 s `CAMEL_SIT_POSE`,
        /// non-looping), driven by `Camel.setupAnimationStates()` while the camel is visually
        /// sitting but past the sit-down window (the steady seated hold).
        pub sit_pose_seconds: f32 = -1.0,
        /// Vanilla `Camel.sitUpAnimationState` elapsed seconds (the 2.6 s `CAMEL_STANDUP`,
        /// non-looping), driven by `Camel.setupAnimationStates()` while the camel is NOT visually
        /// sitting but still in the stand-up pose transition (`isInPoseTransition() &&
        /// getPoseTime() >= 0`).
        pub standup_seconds: f32 = -1.0,
        /// Vanilla `Camel.dashAnimationState` elapsed seconds (the looping 0.5 s `CAMEL_DASH`),
        /// driven by `Camel.setupAnimationStates()`'s `dashAnimationState.animateWhen(isDashing(),
        /// tickCount)` — the synced `DASH` boolean rising edge starts it.
        pub dash_seconds: f32 = -1.0,
        /// Vanilla `Camel.idleAnimationState` elapsed seconds (the 4.0 s non-looping `CAMEL_IDLE`),
        /// restarted by `Camel.setupAnimationStates()` every `random.nextInt(40) + 80` client
        /// ticks.
        pub idle_seconds: f32 = -1.0,
        /// Vanilla `CamelRenderState.jumpCooldown`.
        pub jump_cooldown: f32 = 0.0,
        /// Vanilla `CamelRenderState.saddle`.
        pub saddle_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub saddle_dye: Option<u32> = None,
        /// Vanilla armor-trim atlas sprite for the family-specific `CAMEL*_SADDLE` equipment model.
        pub saddle_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for the camel saddle item.
        pub saddle_foil: bool = false,
        /// Vanilla `CamelRenderState.isRidden`.
        pub saddle_ridden: bool = false,
    }
}

render_state_struct! {
    pub struct FrogRenderState {
        /// Vanilla frog croak timing (`FrogRenderState.croakAnimationState` driven by the synced
        /// `Pose.CROAKING`).
        pub croak_seconds: f32 = -1.0,
        /// Vanilla frog tongue timing (`FrogRenderState.tongueAnimationState` driven by the synced
        /// `Pose.USING_TONGUE`).
        pub tongue_seconds: f32 = -1.0,
        /// Vanilla frog jump timing (`FrogRenderState.jumpAnimationState` driven by the synced
        /// `Pose.LONG_JUMPING`).
        pub jump_seconds: f32 = -1.0,
        /// Vanilla frog swim-idle timing (`FrogRenderState.swimIdleAnimationState`, driven each
        /// client tick by `Frog.tick`'s `animateWhen(isInWater() && !walkAnimation.isMoving(),
        /// tickCount)`).
        pub swim_idle_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct SnifferRenderState {
        /// Vanilla sniffer animation selector (`Sniffer.onSyncedDataUpdated`'s one-shot
        /// `AnimationState`s driven by the synced `DATA_STATE`).
        pub animation_id: i32 = -1,
        /// Vanilla sniffer animation timing: the elapsed seconds since the active `Sniffer.State`
        /// animation started (paired with `sniffer_animation_id`), sampled by
        /// `SnifferModel.setupAnim`.
        pub animation_seconds: f32 = -1.0,
        /// Vanilla `SnifferRenderState.isSearching` (`Sniffer.isSearching()`, synced `DATA_STATE`
        /// == `SEARCHING`).
        pub is_searching: bool = false,
    }
}

render_state_struct! {
    pub struct ArmadilloRenderState {
        /// Vanilla `ArmadilloRenderState.isHidingInShell` (`Armadillo.shouldHideInShell()`).
        pub is_hiding_in_shell: bool = false,
        /// Vanilla armadillo roll-up timing (`Armadillo.rollUpAnimationState`, started on entry to
        /// ROLLING).
        pub roll_up_seconds: f32 = -1.0,
        /// Vanilla armadillo roll-out timing (`Armadillo.rollOutAnimationState`, started on entry
        /// to UNROLLING).
        pub roll_out_seconds: f32 = -1.0,
        /// Vanilla armadillo peek timing (`Armadillo.peekAnimationState`).
        pub peek_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct FoxRenderState {
        /// Vanilla `FoxRenderState.headRollAngle` (`Fox.getHeadRollAngle(partialTick)`).
        pub head_roll_angle: f32 = 0.0,
        /// Vanilla `FoxRenderState.crouchAmount` (`Fox.getCrouchAmount(partialTick)`).
        pub crouch_amount: f32 = 0.0,
        /// Vanilla `FoxRenderState.isCrouching` (`Fox.isCrouching()`, the synced `DATA_FLAGS_ID &
        /// 4`).
        pub is_crouching: bool = false,
        /// Vanilla `FoxRenderState.isSleeping` (`Fox.isSleeping()`, the synced `DATA_FLAGS_ID &
        /// 32`).
        pub is_sleeping: bool = false,
        /// Vanilla `FoxRenderState.isSitting` (`Fox.isSitting()`, the synced `DATA_FLAGS_ID & 1`).
        pub is_sitting: bool = false,
        /// Vanilla `FoxRenderState.isPouncing` (`Fox.isPouncing()`, the synced `DATA_FLAGS_ID &
        /// 16`).
        pub is_pouncing: bool = false,
        /// Vanilla `FoxRenderState.isFaceplanted` (`Fox.isFaceplanted()`, the synced `DATA_FLAGS_ID
        /// & 64`).
        pub is_faceplanted: bool = false,
    }
}

render_state_struct! {
    pub struct FelineRenderState {
        /// Vanilla `FelineRenderState.isCrouching` (`Entity.isCrouching()`, pose `CROUCHING`).
        pub is_crouching: bool = false,
        /// Vanilla `FelineRenderState.isSprinting` (`Entity.isSprinting()`, shared flags bit 3).
        pub is_sprinting: bool = false,
        /// Vanilla `CatRenderer.extractRenderState` copies `Cat.isInSittingPose()` into
        /// `FelineRenderState.isSitting`; `OcelotRenderer` leaves it false.
        pub is_sitting: bool = false,
        /// Vanilla `FelineRenderState.lieDownAmount`, copied by `CatRenderer.extractRenderState`
        /// from `Cat.getLieDownAmount(partialTick)`.
        pub lie_down_amount: f32 = 0.0,
        /// Vanilla `FelineRenderState.lieDownAmountTail`, copied by
        /// `CatRenderer.extractRenderState` from `Cat.getLieDownAmountTail(partialTick)` for the
        /// lying tail pose.
        pub lie_down_amount_tail: f32 = 0.0,
        /// Vanilla `FelineRenderState.relaxStateOneAmount`, copied by
        /// `CatRenderer.extractRenderState` from `Cat.getRelaxStateOneAmount(partialTick)` for the
        /// relaxed head pitch.
        pub relax_state_one_amount: f32 = 0.0,
        /// Vanilla `CatRenderState.isLyingOnTopOfSleepingPlayer`, copied from
        /// `Cat.isLyingOnTopOfSleepingPlayer()`.
        pub is_lying_on_top_of_sleeping_player: bool = false,
    }
}

render_state_struct! {
    pub struct WitchRenderState {
        /// Vanilla `WitchRenderState.isHoldingItem`.
        pub holding_item: bool = false,
        /// Vanilla `WitchRenderState.isHoldingPotion`.
        pub holding_potion: bool = false,
    }
}

render_state_struct! {
    pub struct CopperGolemRenderState {
        /// Vanilla `CopperGolemModel.setupAnim`.
        pub holding_item: bool = false,
        /// Vanilla `CopperGolemRenderState.idleAnimationState` elapsed seconds for the 3.5s
        /// non-looping `COPPER_GOLEM_IDLE` head-spin/body-turn keyframe.
        pub idle_seconds: f32 = -1.0,
        /// Vanilla `CopperGolemRenderState.interactionGetItem` elapsed seconds for
        /// `COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET`.
        pub get_item_seconds: f32 = -1.0,
        /// Vanilla `CopperGolemRenderState.interactionGetNoItem` elapsed seconds for
        /// `COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET`.
        pub get_no_item_seconds: f32 = -1.0,
        /// Vanilla `CopperGolemRenderState.interactionDropItem` elapsed seconds for
        /// `COPPER_GOLEM_CHEST_INTERACTION_ITEM_DROP`.
        pub drop_item_seconds: f32 = -1.0,
        /// Vanilla `CopperGolemRenderState.interactionDropNoItem` elapsed seconds for
        /// `COPPER_GOLEM_CHEST_INTERACTION_ITEM_NODROP`.
        pub drop_no_item_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct VexRenderState {
        /// Vanilla `VexRenderState.isCharging` (`Vex.isCharging`, the synced `DATA_FLAGS_ID & 1`).
        pub charging: bool = false,
        /// Vanilla `ArmedEntityRenderState.rightHandItemState` non-empty for Vex.
        pub right_hand_item_non_empty: bool = false,
        /// Vanilla `ArmedEntityRenderState.leftHandItemState` non-empty for Vex.
        pub left_hand_item_non_empty: bool = false,
    }
}

render_state_struct! {
    pub struct WitherRenderState {
        /// Vanilla `WitherRenderState.invulnerableTicks` (`WitherBoss.getInvulnerableTicks`, the
        /// synced `DATA_ID_INV` spawn countdown, lerped `invulnerableTicks - partialTicks`).
        pub invulnerable_ticks: f32 = 0.0,
        /// Vanilla `WitherRenderState.xHeadRots`.
        pub x_head_rots: [f32; 2] = [0.0; 2],
        /// Vanilla `WitherRenderState.yHeadRots`.
        pub y_head_rots: [f32; 2] = [0.0; 2],
        /// Vanilla `WitherRenderState.isPowered` (`WitherBoss.isPowered() = getHealth() <=
        /// getMaxHealth() / 2`).
        pub powered: bool = false,
    }
}

render_state_struct! {
    pub struct PigRenderState {
        /// Effective asset of the pig's `EquipmentSlot.SADDLE` stack.
        pub saddle_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub saddle_dye: Option<u32> = None,
        /// Vanilla armor-trim atlas sprite for the `PIG_SADDLE` equipment model.
        pub saddle_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for the saddle item.
        pub saddle_foil: bool = false,
    }
}

render_state_struct! {
    pub struct EquineRenderState {
        /// Effective asset of `EquipmentSlot.SADDLE` for horse, donkey, mule, skeleton-horse, and
        /// zombie-horse renderers.
        pub saddle_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub saddle_dye: Option<u32> = None,
        /// Vanilla armor-trim atlas sprite for the family-specific equine saddle equipment model.
        pub saddle_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for the equine saddle item.
        pub saddle_foil: bool = false,
        /// Vanilla `EquineRenderState.isRidden`.
        pub saddle_ridden: bool = false,
        /// Vanilla `EquineRenderState.animateTail`.
        pub animate_tail: bool = false,
        /// Vanilla `EquineRenderState.eatAnimation` (`AbstractHorse.getEatAnim(partialTick)`).
        pub eat_animation: f32 = 0.0,
        /// Vanilla `EquineRenderState.standAnimation` (`AbstractHorse.getStandAnim(partialTick)`).
        pub stand_animation: f32 = 0.0,
        /// Vanilla `EquineRenderState.feedingAnimation`
        /// (`AbstractHorse.getMouthAnim(partialTick)`).
        pub feeding_animation: f32 = 0.0,
        /// Vanilla `EquineRenderState.bodyArmorItem`.
        pub body_equipment_asset: Option<EntityEquipmentAssetId> = None,
        /// Vanilla armor trim over the adult `HORSE_BODY` equipment model, resolved through the
        /// shared runtime trim atlas.
        pub body_armor_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `DyedItemColor.getOrDefault` for `EquineRenderState.bodyArmorItem`.
        pub body_armor_dye: Option<u32> = None,
        /// Vanilla `ItemStack.hasFoil()` for horse body armor.
        pub body_armor_foil: bool = false,
    }
}

render_state_struct! {
    pub struct WolfRenderState {
        /// Vanilla `WolfRenderState.bodyArmorItem`.
        pub body_equipment_asset: Option<EntityEquipmentAssetId> = None,
        /// Vanilla armor trim over the adult `WOLF_BODY` equipment model.
        pub body_armor_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `DyedItemColor.getOrDefault` for the wolf body armor item.
        pub body_armor_dye: Option<u32> = None,
        /// Vanilla `Crackiness.WOLF_ARMOR.byDamage(bodyArmorItem)`.
        pub body_armor_crackiness: WolfArmorCrackiness = WolfArmorCrackiness::None,
        /// Vanilla `ItemStack.hasFoil()` for the wolf body armor item.
        pub body_armor_foil: bool = false,
        /// Vanilla `WolfRenderState.tailAngle` (`Wolf.getTailAngle()`).
        pub tail_angle: f32 = std::f32::consts::PI / 5.0,
        /// Vanilla `WolfRenderState.isSitting` (`Wolf.isInSittingPose()`).
        pub sitting: bool = false,
        /// Vanilla `WolfRenderState.wetShade` (`Wolf.getWetShade(partialTick)`).
        pub wet_shade: f32 = 1.0,
        /// Vanilla `WolfRenderState.shakeAnim` (`Wolf.getShakeAnim(partialTick)`).
        pub shake_anim: f32 = 0.0,
        /// Vanilla `WolfRenderState.headRollAngle` (`Wolf.getHeadRollAngle(partialTick)`).
        pub head_roll_angle: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct StriderRenderState {
        /// Vanilla `StriderRenderState.isRidden`.
        pub ridden: bool = false,
        /// Vanilla `StriderRenderState.saddle`.
        pub saddle_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub saddle_dye: Option<u32> = None,
        /// Vanilla armor-trim atlas sprite for the `STRIDER_SADDLE` equipment model.
        pub saddle_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for the strider saddle item.
        pub saddle_foil: bool = false,
    }
}

render_state_struct! {
    pub struct NautilusRenderState {
        /// Vanilla `NautilusRenderState.saddle`.
        pub saddle_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub saddle_dye: Option<u32> = None,
        /// Vanilla armor-trim atlas sprite for the adult `NAUTILUS_SADDLE` equipment model.
        pub saddle_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for the nautilus saddle item.
        pub saddle_foil: bool = false,
        /// Vanilla `NautilusRenderState.bodyArmorItem`.
        pub body_equipment_asset: Option<EntityEquipmentAssetId> = None,
        /// Vanilla `NautilusRenderState.bodyArmorItem.isEmpty()`, independent of whether the
        /// current resource generation recognizes or can render the stack's equipment asset.
        pub body_item_present: bool = false,
        pub body_armor_dye: Option<u32> = None,
        /// Vanilla armor trim over the adult `NAUTILUS_BODY` equipment model, resolved through the
        /// shared runtime trim atlas.
        pub body_armor_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for nautilus body armor.
        pub body_armor_foil: bool = false,
    }
}

render_state_struct! {
    pub struct HappyGhastRenderState {
        /// Any non-empty happy-ghast BODY item, independently of whether its equipment asset is
        /// recognized.
        pub body_item: bool = false,
        /// Effective BODY-stack equipment asset consumed by `HAPPY_GHAST_BODY`.
        pub body_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub harness_dye: Option<u32> = None,
        /// Armor trim rendered over the happy-ghast harness model.
        pub harness_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for the recognized harness.
        pub harness_foil: bool = false,
        /// Whether the BODY item belongs to the synchronized `minecraft:harnesses` item tag.
        pub body_item_is_harness: bool = false,
        /// Vanilla synchronized `HappyGhastRenderState.isLeashHolder`; the ropes layer additionally
        /// requires BODY-item membership in the synchronized harness tag.
        pub is_leash_holder: bool = false,
        /// True while a happy ghast has passengers; lowers the harness goggles into their ridden
        /// pose.
        pub ridden: bool = false,
    }
}

render_state_struct! {
    pub struct SnowGolemRenderState {
        /// Vanilla `SnowGolemRenderState.headBlock`.
        pub pumpkin: bool = false,
    }
}

render_state_struct! {
    pub struct LlamaRenderState {
        /// Vanilla `LlamaRenderState.bodyItem`.
        pub body_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub body_decor_dye: Option<u32> = None,
        /// Vanilla armor trim over an adult carpet item's `LLAMA_BODY` equipment model.
        pub body_decor_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for an adult llama carpet item.
        pub body_decor_foil: bool = false,
    }
}

render_state_struct! {
    pub struct PiglinRenderState {
        /// Vanilla `Piglin.isDancing()` (the synced `DATA_IS_DANCING` boolean).
        pub dancing: bool = false,
        /// Vanilla `Piglin.getArmPose` `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`,
        /// holding in the right arm).
        pub crossbow_hold: bool = false,
        /// Vanilla `Piglin.getArmPose` `CROSSBOW_CHARGE` (`AnimationUtils.animateCrossbowCharge`,
        /// holding in the right arm).
        pub crossbow_charge: bool = false,
        /// Vanilla `PiglinModel` `ATTACKING_WITH_MELEE_WEAPON` (`PiglinModel.holdWeaponHigh` +
        /// `AnimationUtils.swingWeaponDown`, mainArm = RIGHT).
        pub attacking_with_melee: bool = false,
        /// Vanilla `Piglin.getArmPose` `ADMIRING_ITEM` (`PiglinModel.setupAnim`).
        pub admiring: bool = false,
    }
}

render_state_struct! {
    pub struct PandaRenderState {
        /// Vanilla `PandaRenderState.isUnhappy` (`Panda.getUnhappyCounter() > 0`).
        pub unhappy: bool = false,
        /// Vanilla `PandaRenderState.isSneezing` (`Panda.isSneezing()`, the `DATA_ID_FLAGS` bit).
        pub sneezing: bool = false,
        /// Vanilla `PandaRenderState.sneezeTime` (`Panda.getSneezeCounter()`, the 0..20 ramp).
        pub sneeze_time: i32 = 0,
        /// Vanilla `PandaRenderState.isEating` (`Panda.isEating()`, synced `EAT_COUNTER > 0`).
        pub eating: bool = false,
        /// Vanilla `PandaRenderState.isScared` (`Panda.isWorried() && level.isThundering()`).
        pub scared: bool = false,
        /// Vanilla `PandaRenderState.isSitting` (`DATA_ID_FLAGS` bit `0x08`).
        pub sitting: bool = false,
        /// Vanilla `PandaRenderState.sitAmount` (`Panda.getSitAmount(partialTick)`).
        pub sit_amount: f32 = 0.0,
        /// Vanilla `PandaRenderState.lieOnBackAmount` (`Panda.getLieOnBackAmount(partialTick)`).
        pub lie_on_back_amount: f32 = 0.0,
        /// Vanilla `PandaRenderState.rollAmount` (`Panda.getRollAmount(partialTick)`, forced to
        /// `0.0` for baby pandas).
        pub roll_amount: f32 = 0.0,
        /// Vanilla `PandaRenderState.rollTime` (`rollCounter + partialTick` while rolling).
        pub roll_time: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct GoatRenderState {
        /// Vanilla `GoatRenderState.rammingXHeadRot` (`Goat.getRammingXHeadRot()`).
        pub ramming_x_head_rot: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct IronGolemRenderState {
        /// Vanilla `IronGolemRenderState.attackTicksRemaining` (the partial-lerped
        /// `attackAnimationTick`).
        pub attack_ticks_remaining: f32 = 0.0,
        /// Vanilla `IronGolemRenderState.offerFlowerTick`.
        pub offer_flower_tick: i32 = 0,
    }
}

render_state_struct! {
    pub struct RavagerRenderState {
        /// Vanilla `RavagerRenderState.stunnedTicksRemaining` (partial-lerped `stunnedTick`).
        pub stunned_ticks_remaining: f32 = 0.0,
        /// Vanilla `RavagerRenderState.attackTicksRemaining` (partial-lerped `attackTick`).
        pub attack_ticks_remaining: f32 = 0.0,
        /// Vanilla `RavagerRenderState.roarAnimation` (the `0..1` roar ramp).
        pub roar_animation: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct HoglinRenderState {
        /// Vanilla `HoglinRenderState.attackAnimationRemainingTicks` (the RAW `0..10` headbutt
        /// timer).
        pub attack_animation_tick: i32 = 0,
    }
}

render_state_struct! {
    pub struct ArmorStandRenderState {
        /// Vanilla `ArmorStandRenderState.wiggle`.
        pub wiggle: f32 = 5.0,
    }
}

render_state_struct! {
    pub struct ParrotRenderState {
        /// Vanilla `ParrotRenderState.pose == SITTING` (`Parrot.isInSittingPose()`, the
        /// `TamableAnimal.DATA_FLAGS_ID` sitting bit).
        pub sitting: bool = false,
        /// Vanilla `ParrotRenderState.pose == PARTY` (`Parrot.isPartyParrot()`).
        pub party: bool = false,
        /// Vanilla `ParrotRenderState.pose == ON_SHOULDER`.
        pub on_shoulder: bool = false,
        /// Vanilla `ParrotRenderState.flapAngle` (`ParrotRenderer.extractRenderState`:
        /// `(Mth.sin(lerp(oFlap, flap)) + 1) * lerp(oFlapSpeed, flapSpeed)`).
        pub flap_angle: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct TurtleRenderState {
        /// Vanilla `TurtleRenderState.hasEgg` (`!isBaby() && Turtle.hasEgg()`, the synced `HAS_EGG`
        /// boolean).
        pub has_egg: bool = false,
        /// Vanilla `TurtleRenderState.isLayingEgg` (the synced `Turtle.LAYING_EGG` boolean).
        pub laying_egg: bool = false,
    }
}

render_state_struct! {
    pub struct EndCrystalRenderState {
        /// Vanilla `EndCrystalRenderState.showsBottom` (the synced `EndCrystal.DATA_SHOW_BOTTOM`
        /// boolean, default `true`).
        pub shows_bottom: bool = true,
        /// Vanilla `EndCrystalRenderState.beamOffset`.
        pub beam: Option<EndCrystalBeamRenderState> = None,
    }
}

render_state_struct! {
    pub struct EnderDragonRenderState {
        /// Vanilla `EnderDragonRenderState.deathTime` (`entity.dragonDeathTime > 0 ?
        /// entity.dragonDeathTime + partialTick : 0`).
        pub death_time: f32 = 0.0,
        /// Vanilla `EnderDragonRenderState.beamOffset`.
        pub beam: Option<EnderDragonBeamRenderState> = None,
        /// Vanilla `EnderDragonRenderState`'s procedural-animation inputs (`flapTime`, phase flags,
        /// egg distance, interpolated flight history), consumed by `EnderDragonModel.setupAnim` and
        /// the renderer's flight-history root yaw/pitch.
        pub animation: Option<EnderDragonAnimationRenderState> = None,
    }
}

render_state_struct! {
    pub struct SquidRenderState {
        /// Vanilla `SquidRenderState.tentacleAngle` (`Mth.lerp(partialTick, oldTentacleAngle,
        /// tentacleAngle)`).
        pub tentacle_angle: f32 = 0.0,
        /// Vanilla `SquidRenderState.xBodyRot` (`Mth.lerp(partialTick, xBodyRotO, xBodyRot)`,
        /// degrees).
        pub x_body_rot: f32 = 0.0,
        /// Vanilla `SquidRenderState.zBodyRot` (`Mth.lerp(partialTick, zBodyRotO, zBodyRot)`,
        /// degrees).
        pub z_body_rot: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct GuardianRenderState {
        /// Vanilla `GuardianRenderState.tailAnimation` (`Mth.lerp(partialTick,
        /// clientSideTailAnimationO, clientSideTailAnimation)`).
        pub tail_animation: f32 = 0.0,
        /// Vanilla `GuardianRenderState.spikesAnimation` (`Guardian.getSpikesAnimation` lerped).
        pub spikes_animation: f32 = 1.0,
        /// Vanilla `GuardianRenderer` attack beam (`GuardianRenderState.attackTargetPosition`
        /// present).
        pub beam: Option<GuardianBeamRenderState> = None,
        /// Vanilla `GuardianRenderState.{eyePosition,lookDirection,lookAtPosition}`.
        pub eye_tracking: Option<GuardianEyeTrackingRenderState> = None,
    }
}

render_state_struct! {
    pub struct BreezeRenderState {
        /// Vanilla `Breeze`'s pose-driven action one-shots (elapsed seconds since each started,
        /// sampled by `BreezeModel.setupAnim` over the projected pose), applied additively to the
        /// body/head/rods.
        pub shoot_seconds: f32 = -1.0,
        pub slide_seconds: f32 = -1.0,
        pub slide_back_seconds: f32 = -1.0,
        pub inhale_seconds: f32 = -1.0,
        pub long_jump_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct ChickenRenderState {
        /// Vanilla `ChickenRenderState.flap` (`Mth.lerp(partialTick, oFlap, flap)`).
        pub flap: f32 = 0.0,
        /// Vanilla `ChickenRenderState.flapSpeed` (`Mth.lerp(partialTick, oFlapSpeed, flapSpeed)`).
        pub flap_speed: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct SlimeRenderState {
        /// Vanilla `SlimeRenderState.squish` (`Mth.lerp(partialTick, oSquish, squish)`).
        pub squish: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct EvokerFangsRenderState {
        /// Vanilla `EvokerFangsRenderState.biteProgress` (`EvokerFangs.getAnimationProgress`).
        pub bite_progress: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct AllayRenderState {
        /// Vanilla `AllayModel.setupAnim`.
        pub dancing: bool = false,
        /// Vanilla `AllayModel`: `true` during the spin sub-window of the dance (`danceAnimation %
        /// 55 < 15`), selecting the `4π * progress` body spin (`root.yRot`).
        pub spinning: bool = false,
        /// Vanilla `AllayModel`: the `0..1` lerped spin blend (`spinningAnimation / 15`) that
        /// cross-fades the body sway (`root.zRot`/`head` tilt scaled by `1 - progress`) into the
        /// spin.
        pub spinning_progress: f32 = 0.0,
        /// Vanilla `AllayRenderState.holdingAnimationProgress`
        /// (`Allay.getHoldingItemAnimationProgress(partialTick)`).
        pub holding_item_progress: f32 = 0.0,
    }
}

render_state_struct! {
    pub struct AxolotlRenderState {
        /// Vanilla `AxolotlRenderState.playingDeadFactor`
        /// (`Axolotl.playingDeadAnimator.getFactor`).
        pub playing_dead_factor: f32 = 0.0,
        /// Vanilla `AxolotlRenderState.inWaterFactor` (`Axolotl.inWaterAnimator.getFactor`).
        pub in_water_factor: f32 = 0.0,
        /// Vanilla `AxolotlRenderState.onGroundFactor` (`Axolotl.onGroundAnimator.getFactor`).
        pub on_ground_factor: f32 = 0.0,
        /// Vanilla `AxolotlRenderState.movingFactor` (`Axolotl.movingAnimator.getFactor`).
        pub moving_factor: f32 = 0.0,
        /// Vanilla baby Axolotl AnimationState elapsed seconds; `-1.0` means stopped.
        pub baby_swim_seconds: f32 = -1.0,
        pub baby_walk_seconds: f32 = -1.0,
        pub baby_idle_underwater_seconds: f32 = -1.0,
        pub baby_idle_underwater_on_ground_seconds: f32 = -1.0,
        pub baby_idle_on_ground_seconds: f32 = -1.0,
        pub baby_play_dead_seconds: f32 = -1.0,
    }
}

render_state_struct! {
    pub struct DolphinRenderState {
        /// Vanilla `DolphinRenderState.isMoving` (`getDeltaMovement().horizontalDistanceSqr() >
        /// 1e-7`).
        pub is_moving: bool = false,
    }
}

entity_kind_render_states! {
    Arrow(ArrowRenderState) => arrow, arrow_mut;
    Trident(TridentRenderState) => trident, trident_mut;
    FishingHook(FishingHookRenderState) => fishing_hook, fishing_hook_mut;
    Sheep(SheepRenderState) => sheep, sheep_mut;
    PolarBear(PolarBearRenderState) => polar_bear, polar_bear_mut;
    Creeper(CreeperRenderState) => creeper, creeper_mut;
    Villager(VillagerRenderState) => villager, villager_mut;
    Shulker(ShulkerRenderState) => shulker, shulker_mut;
    Warden(WardenRenderState) => warden, warden_mut;
    Rabbit(RabbitRenderState) => rabbit, rabbit_mut;
    Creaking(CreakingRenderState) => creaking, creaking_mut;
    Boat(BoatRenderState) => boat, boat_mut;
    Minecart(MinecartRenderState) => minecart, minecart_mut;
    Player(PlayerRenderState) => player, player_mut;
    Illager(IllagerRenderState) => illager, illager_mut;
    Drowned(DrownedRenderState) => drowned, drowned_mut;
    Enderman(EndermanRenderState) => enderman, enderman_mut;
    Bat(BatRenderState) => bat, bat_mut;
    Bee(BeeRenderState) => bee, bee_mut;
    Camel(CamelRenderState) => camel, camel_mut;
    Frog(FrogRenderState) => frog, frog_mut;
    Sniffer(SnifferRenderState) => sniffer, sniffer_mut;
    Armadillo(ArmadilloRenderState) => armadillo, armadillo_mut;
    Fox(FoxRenderState) => fox, fox_mut;
    Feline(FelineRenderState) => feline, feline_mut;
    Witch(WitchRenderState) => witch, witch_mut;
    CopperGolem(CopperGolemRenderState) => copper_golem, copper_golem_mut;
    Vex(VexRenderState) => vex, vex_mut;
    Wither(WitherRenderState) => wither, wither_mut;
    Pig(PigRenderState) => pig, pig_mut;
    Equine(EquineRenderState) => equine, equine_mut;
    Wolf(WolfRenderState) => wolf, wolf_mut;
    Strider(StriderRenderState) => strider, strider_mut;
    Nautilus(NautilusRenderState) => nautilus, nautilus_mut;
    HappyGhast(HappyGhastRenderState) => happy_ghast, happy_ghast_mut;
    SnowGolem(SnowGolemRenderState) => snow_golem, snow_golem_mut;
    Llama(LlamaRenderState) => llama, llama_mut;
    Piglin(PiglinRenderState) => piglin, piglin_mut;
    Panda(PandaRenderState) => panda, panda_mut;
    Goat(GoatRenderState) => goat, goat_mut;
    IronGolem(IronGolemRenderState) => iron_golem, iron_golem_mut;
    Ravager(RavagerRenderState) => ravager, ravager_mut;
    Hoglin(HoglinRenderState) => hoglin, hoglin_mut;
    ArmorStand(ArmorStandRenderState) => armor_stand, armor_stand_mut;
    Parrot(ParrotRenderState) => parrot, parrot_mut;
    Turtle(TurtleRenderState) => turtle, turtle_mut;
    EndCrystal(EndCrystalRenderState) => end_crystal, end_crystal_mut;
    EnderDragon(EnderDragonRenderState) => ender_dragon, ender_dragon_mut;
    Squid(SquidRenderState) => squid, squid_mut;
    Guardian(GuardianRenderState) => guardian, guardian_mut;
    Breeze(BreezeRenderState) => breeze, breeze_mut;
    Chicken(ChickenRenderState) => chicken, chicken_mut;
    Slime(SlimeRenderState) => slime, slime_mut;
    EvokerFangs(EvokerFangsRenderState) => evoker_fangs, evoker_fangs_mut;
    Allay(AllayRenderState) => allay, allay_mut;
    Axolotl(AxolotlRenderState) => axolotl, axolotl_mut;
    Dolphin(DolphinRenderState) => dolphin, dolphin_mut;
}

impl EntityKindRenderState {
    /// The boat or minecart hurt wobble; the defaults for every other kind.
    pub fn vehicle_hurt(&self) -> VehicleHurtRenderState {
        match self {
            Self::Boat(boat) => boat.hurt,
            Self::Minecart(minecart) => minecart.hurt,
            _ => VehicleHurtRenderState::DEFAULT,
        }
    }
}
