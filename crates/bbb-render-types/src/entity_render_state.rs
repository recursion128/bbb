//! The per-frame entity render state: projected once from world state at the
//! render partial tick and consumed directly by the renderer entity pass.

use bbb_protocol::packets::{GameProfile as ProtocolGameProfile, GameProfilePropertySummary};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    EntityArmorTrim, EntityCustomHeadSkull, EntityEquipmentAssetId, VillagerModelProfession,
    VillagerModelType,
};

pub const ELYTRA_DEFAULT_X_ROT: f32 = std::f32::consts::PI / 12.0;
pub const ELYTRA_DEFAULT_Y_ROT: f32 = 0.0;
pub const ELYTRA_DEFAULT_Z_ROT: f32 = -std::f32::consts::PI / 12.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct DragonFlightHistorySample {
    pub y: f64,
    pub y_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EntityAttachmentFace {
    #[default]
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl EntityAttachmentFace {
    pub fn from_3d_data(data: i32) -> Self {
        match data.rem_euclid(6) {
            0 => Self::Down,
            1 => Self::Up,
            2 => Self::North,
            3 => Self::South,
            4 => Self::West,
            _ => Self::East,
        }
    }
}

/// Vanilla `Crackiness.Level` for wolf armor (`Crackiness.WOLF_ARMOR.byDamage`): low / medium /
/// high damage cracks overlaid on the adult wolf armor model, with `None` for intact armor or any
/// body item that is not damageable client-side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WolfArmorCrackiness {
    #[default]
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoProfileState {
    pub uuid: Uuid,
    pub name: String,
    pub properties: Vec<GameProfilePropertySummary>,
}

impl From<&ProtocolGameProfile> for PlayerInfoProfileState {
    fn from(profile: &ProtocolGameProfile) -> Self {
        Self {
            uuid: profile.uuid,
            name: profile.name.clone(),
            properties: profile
                .properties
                .iter()
                .map(|property| GameProfilePropertySummary {
                    name: property.name.as_str().into(),
                    value: property.value.as_str().into(),
                    signature: property.signature.as_deref().map(Into::into),
                })
                .collect(),
        }
    }
}

/// Declares a `Copy` render-state struct whose fields carry their default values, plus the matching
/// `DEFAULT` constant and `Default` impl.
macro_rules! render_state_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($(#[$field_meta:meta])* pub $field:ident: $ty:ty = $default:expr,)*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub struct $name {
            $($(#[$field_meta])* pub $field: $ty,)*
        }

        impl $name {
            pub const DEFAULT: Self = Self { $($field: $default,)* };
        }

        impl Default for $name {
            fn default() -> Self {
                Self::DEFAULT
            }
        }
    };
}

mod kind;

pub use kind::*;

/// Vanilla `LightCoordsUtil.FULL_BRIGHT` (`block 15 | sky 15`): the
/// `EntityRenderState`/`LivingEntityRenderState.lightCoords` default used until
/// the entity scene projects sampled block+sky light.
pub const ENTITY_FULL_BRIGHT_LIGHT_COORDS: u32 = 15_728_880;

/// Vanilla `Entity.getTeamColor()` fallback (`0xFFFFFF`) wrapped by
/// `ARGB.opaque` for a glowing entity with no colored scoreboard team.
pub const ENTITY_DEFAULT_OUTLINE_COLOR: u32 = 0xffff_ffff;

/// Vanilla `DataComponents.KINETIC_WEAPON` subset needed by `SpearAnimations.thirdPersonHandUse`
/// and `thirdPersonUseItem`, projected from the effective typed stack component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpearKineticWeapon {
    pub delay_ticks: f32,
    pub dismount_duration_ticks: f32,
    pub knockback_duration_ticks: f32,
    pub damage_duration_ticks: f32,
    pub forward_movement: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpearKineticUseParams {
    pub raise_progress: f32,
    pub raise_progress_start: f32,
    pub raise_progress_middle: f32,
    pub raise_progress_end: f32,
    pub sway_progress: f32,
    pub lower_progress: f32,
    pub raise_back_progress: f32,
    pub sway_intensity: f32,
    pub sway_scale_slow: f32,
    pub sway_scale_fast: f32,
}

impl SpearKineticWeapon {
    pub fn use_params(self, time: f32) -> SpearKineticUseParams {
        let finish_raising_tick = self.delay_ticks;
        let finish_swaying_tick = self.dismount_duration_ticks + finish_raising_tick;
        let start_swaying_tick = finish_swaying_tick - 20.0;
        let finish_lowering_tick = self.knockback_duration_ticks + finish_raising_tick;
        let start_lowering_tick = finish_lowering_tick - 40.0;
        let finish_raising_back_tick = self.damage_duration_ticks + finish_raising_tick;
        let raise_progress = progress(time, 0.0, finish_raising_tick);
        let raise_progress_start = progress(raise_progress, 0.0, 0.5);
        let raise_progress_middle = progress(raise_progress, 0.5, 0.8);
        let raise_progress_end = progress(raise_progress, 0.8, 1.0);
        let sway_progress = progress(time, start_swaying_tick, finish_swaying_tick);
        let lower_progress = ease_out_cubic(ease_in_out_elastic(progress(
            time - 20.0,
            start_lowering_tick,
            finish_lowering_tick,
        )));
        let raise_back_progress = progress(
            time,
            finish_raising_back_tick - 5.0,
            finish_raising_back_tick,
        );
        let sway_intensity =
            2.0 * ease_out_circ(sway_progress) - 2.0 * ease_in_circ(raise_back_progress);
        let sway_scale_slow = (time * 19.0_f32.to_radians()).sin() * sway_intensity;
        let sway_scale_fast = (time * 30.0_f32.to_radians()).sin() * sway_intensity;
        SpearKineticUseParams {
            raise_progress,
            raise_progress_start,
            raise_progress_middle,
            raise_progress_end,
            sway_progress,
            lower_progress,
            raise_back_progress,
            sway_intensity,
            sway_scale_slow,
            sway_scale_fast,
        }
    }
}

fn progress(value: f32, start: f32, end: f32) -> f32 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}

fn ease_out_circ(x: f32) -> f32 {
    (1.0 - (x - 1.0).powi(2)).sqrt()
}

fn ease_in_circ(x: f32) -> f32 {
    1.0 - (1.0 - x * x).sqrt()
}

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}

fn ease_in_out_elastic(x: f32) -> f32 {
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    let sin = ((20.0 * x - 11.125) * std::f32::consts::PI * 4.0 / 9.0).sin();
    if x < 0.5 {
        -(2.0_f32.powf(20.0 * x - 10.0) * sin) / 2.0
    } else {
        2.0_f32.powf(-20.0 * x + 10.0) * sin / 2.0 + 1.0
    }
}

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

/// Vanilla `SheepRenderState.headEatPositionScale` / `headEatAngleScale`, the
/// per-frame eat-grass head animation projected from `Sheep.eatAnimationTick`.
/// `SheepModel`/`SheepFurModel.setupAnim` consume these to lower and tilt the
/// head part of the base, wool, and undercoat passes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheepHeadEatPose {
    pub position_scale: f32,
    pub angle_scale: f32,
}

impl SheepHeadEatPose {
    /// Resting head pose used when the sheep is not eating grass.
    pub const NONE: Self = Self {
        position_scale: 0.0,
        angle_scale: 0.0,
    };

    /// Vanilla `Sheep.getHeadEatPositionScale`/`getHeadEatAngleScale` projected
    /// from the canonical `eatAnimationTick` and the renderer partial tick.
    pub fn from_eat_tick(eat_animation_tick: i32, partial_tick: f32) -> Self {
        Self {
            position_scale: sheep_head_eat_position_scale(eat_animation_tick, partial_tick),
            angle_scale: sheep_head_eat_angle_scale(eat_animation_tick, partial_tick),
        }
    }

    pub fn is_resting(self) -> bool {
        self == Self::NONE
    }
}

/// Vanilla `Sheep.getHeadEatAngleScale` plateau angle: `(float)(Math.PI / 5)`.
const SHEEP_HEAD_EAT_PLATEAU_ANGLE: f32 = std::f32::consts::PI / 5.0;

/// Vanilla `Sheep.getHeadEatPositionScale(partialTick)`.
fn sheep_head_eat_position_scale(eat_animation_tick: i32, partial_tick: f32) -> f32 {
    if eat_animation_tick <= 0 {
        0.0
    } else if (4..=36).contains(&eat_animation_tick) {
        1.0
    } else if eat_animation_tick < 4 {
        (eat_animation_tick as f32 - partial_tick) / 4.0
    } else {
        -(eat_animation_tick as f32 - 40.0 - partial_tick) / 4.0
    }
}

/// Vanilla `Sheep.getHeadEatAngleScale(partialTick)`, restricted to the eating
/// branches (`eatAnimationTick > 0`). The non-eating branch of the vanilla
/// method folds in the entity look pitch (`getXRot(a) * PI/180`); that pitch is
/// projected separately as [`EntityRenderState::head_pitch`] and applied by
/// [`sheep_head_pose`], so a resting (non-eating) sheep returns `0.0` here and
/// the head pitch comes from the look projection instead.
fn sheep_head_eat_angle_scale(eat_animation_tick: i32, partial_tick: f32) -> f32 {
    if eat_animation_tick > 4 && eat_animation_tick <= 36 {
        let scale = (eat_animation_tick as f32 - 4.0 - partial_tick) / 32.0;
        SHEEP_HEAD_EAT_PLATEAU_ANGLE + 0.21991149 * (scale * 28.7).sin()
    } else if eat_animation_tick > 0 {
        SHEEP_HEAD_EAT_PLATEAU_ANGLE
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillagerModelData {
    pub villager_type: VillagerModelType,
    pub profession: VillagerModelProfession,
    pub level: i32,
}

impl VillagerModelData {
    pub const DEFAULT: Self = Self {
        villager_type: VillagerModelType::Plains,
        profession: VillagerModelProfession::None,
        level: 1,
    };

    pub const fn new(
        villager_type: VillagerModelType,
        profession: VillagerModelProfession,
        level: i32,
    ) -> Self {
        Self {
            villager_type,
            profession,
            level,
        }
    }
}

/// Vanilla `Parrot.Variant` (the synced `DATA_VARIANT_ID` int): the five parrot colours, sharing
/// one `ParrotModel` and differing only by texture (`ParrotRenderer.getVariantTexture`). `RED_BLUE`
/// is the vanilla `DEFAULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParrotModelVariant {
    RedBlue,
    Blue,
    Green,
    YellowBlue,
    Gray,
}

impl ParrotModelVariant {
    /// Vanilla `Parrot.Variant.byId`, backed by `ByIdMap.continuous(..., CLAMP)`.
    pub fn from_id(id: i32) -> Self {
        match id {
            i32::MIN..=0 => Self::RedBlue,
            1 => Self::Blue,
            2 => Self::Green,
            3 => Self::YellowBlue,
            4..=i32::MAX => Self::Gray,
        }
    }

    /// Vanilla `Parrot.Variant.id`, used by the save-data debug projection.
    pub const fn vanilla_id(self) -> i32 {
        match self {
            Self::RedBlue => 0,
            Self::Blue => 1,
            Self::Green => 2,
            Self::YellowBlue => 3,
            Self::Gray => 4,
        }
    }
}

/// Vanilla `GuardianRenderer` attack-beam render state (`GuardianRenderState.attackTargetPosition`
/// present): a guardian firing its beam at a target. `eye_to_target` is the world-space vector from
/// the guardian's eye to the target center (`attackTargetPosition − eyePosition`, vanilla
/// `beamVector`); `eye_height` raises the beam origin from the entity feet to the eye;
/// `attack_time` is the lerped `clientSideAttackTime + partialTicks` (drives the texture V-scroll
/// and the prism twist); and `attack_scale` is `getAttackAnimationScale` (`0..1`, drives the beam
/// color ramp). Projected as `Option`, `None` for a guardian with no active target and every other
/// entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardianBeamRenderState {
    pub eye_to_target: [f32; 3],
    pub eye_height: f32,
    pub attack_time: f32,
    pub attack_scale: f32,
}

/// Vanilla `GuardianRenderState` eye-tracking inputs. `GuardianModel.setupAnim` keeps these in
/// world-space doubles while deriving the eye part's horizontal and vertical model-space offsets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuardianEyeTrackingRenderState {
    pub eye_position: [f64; 3],
    pub look_direction: [f64; 3],
    pub look_at_position: [f64; 3],
}

/// Vanilla `EndCrystalRenderState.beamOffset`: target block center relative to the crystal
/// position. The renderer combines this with the crystal's `ageInTicks` bob
/// (`EndCrystalRenderer.getY`) when submitting the `endCrystalBeam` custom geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndCrystalBeamRenderState {
    pub beam_offset: [f32; 3],
}

/// Vanilla `EnderDragonRenderState.beamOffset`: nearest end-crystal position (including the crystal
/// bob from `EndCrystalRenderer.getY`) relative to the dragon position. The dragon renderer submits
/// the same `endCrystalBeam` custom geometry after the body and eyes passes when this is present.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnderDragonBeamRenderState {
    pub beam_offset: [f32; 3],
}

/// Vanilla `EnderDragonRenderState`'s procedural-animation inputs for `EnderDragonModel.setupAnim`
/// and the renderer's flight-history root yaw/pitch. `historical` carries `getHistoricalPos(delay)`
/// pre-interpolated for every delay the model samples (`0..=23`), with the dead-dragon
/// `partialTicks = 0` freeze already applied by the world projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnderDragonAnimationRenderState {
    pub flap_time: f32,
    pub is_landing_or_taking_off: bool,
    pub is_sitting: bool,
    pub distance_to_egg: f64,
    pub historical: [DragonFlightHistorySample; 24],
}

impl EnderDragonAnimationRenderState {
    /// Vanilla `EnderDragonRenderState.getHistoricalPos(delay)` over the projected samples.
    pub fn historical_pos(&self, delay: usize) -> DragonFlightHistorySample {
        self.historical[delay]
    }

    /// Vanilla `EnderDragonRenderState.getHeadPartYOffset(part, bodyPos, partPos)`: the landing /
    /// takeoff droop divides the part index by the egg distance, sitting droops linearly, the head
    /// itself (`part == 6`) stays level, and free flight reads the sample height difference.
    pub fn head_part_y_offset(
        &self,
        part: i32,
        body_pos: DragonFlightHistorySample,
        part_pos: DragonFlightHistorySample,
    ) -> f32 {
        let result = if self.is_landing_or_taking_off {
            f64::from(part) / (self.distance_to_egg / 4.0).max(1.0)
        } else if self.is_sitting {
            f64::from(part)
        } else if part == 6 {
            0.0
        } else {
            part_pos.y - body_pos.y
        };
        result as f32
    }
}

/// One rope of a leash connection (vanilla `EntityRenderState.LeashState`): `offset` is the rotated
/// attach offset (the rope start relative to the entity render position), `delta` is the
/// world-space `end − start` span (`LeashFeatureRenderer.renderLeash`'s `dx/dy/dz` after its
/// `float` casts), the endpoint block/sky lights feed the per-step light lerp, and `slack` selects
/// the sag curve over the taut line a quad connection uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeashRopeRenderState {
    pub offset: [f32; 3],
    pub delta: [f32; 3],
    pub start_block_light: u8,
    pub start_sky_light: u8,
    pub end_block_light: u8,
    pub end_sky_light: u8,
    pub slack: bool,
}

/// Vanilla `EntityRenderState.leashStates`: the ropes attached to a leashed entity — one for a
/// normal leash, four for a ghast-to-boat quad connection. Fixed-size because `EntityRenderState`
/// is `Copy`; `count` bounds the live prefix of `ropes`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityLeashRopesRenderState {
    pub ropes: [LeashRopeRenderState; 4],
    pub count: u8,
}

impl EntityLeashRopesRenderState {
    /// Packs up to four ropes; `None` for an empty slice (an unleashed entity).
    pub fn from_ropes(source: &[LeashRopeRenderState]) -> Option<Self> {
        let (&first, _) = source.split_first()?;
        let count = source.len().min(4);
        let mut ropes = [first; 4];
        ropes[..count].copy_from_slice(&source[..count]);
        Some(Self {
            ropes,
            count: count as u8,
        })
    }

    /// The live ropes.
    pub fn ropes(&self) -> &[LeashRopeRenderState] {
        &self.ropes[..usize::from(self.count).min(self.ropes.len())]
    }
}

render_state_struct! {
    /// Vanilla `EntityRenderState` and its living/humanoid subclasses: the fields shared across
    /// renderers, plus the kind-specific part in [`EntityRenderState::kind`].
    pub struct EntityRenderState {
        /// Vanilla `LivingEntityRenderState.bodyRot`: the interpolated body yaw, in
        /// degrees, that orients the model root transform. The entity scene folds the
        /// `LivingEntityRenderer.setupRotations` freezing body shake into this value.
        pub body_rot: f32 = 0.0,
        /// Vanilla `LivingEntityRenderState.yRot`: the net head yaw in degrees
        /// (`Mth.wrapDegrees(headRot - bodyRot)`), i.e. the head turn relative to the
        /// body that models apply as `head.yRot = yRot * π/180`. `0.0` when the head
        /// is aligned with the body. The entity scene projects it from the canonical
        /// head/body yaw.
        pub head_yaw: f32 = 0.0,
        /// Vanilla `LivingEntityRenderState.xRot`: the head pitch in degrees
        /// (`entity.getXRot`), applied as `head.xRot = xRot * π/180`. `0.0` when the
        /// head is level.
        pub head_pitch: f32 = 0.0,
        /// Vanilla `LivingEntityRenderState.deathTime` (`entity.deathTime > 0 ?
        /// entity.deathTime + partialTick : 0`): the lerped death-animation counter
        /// that tips a dying living entity over in `LivingEntityRenderer.setupRotations`
        /// (`Axis.ZP.rotationDegrees(sqrt(min((deathTime-1)/20*1.6, 1)) *
        /// getFlipDegrees())`). `0.0` for every entity that is alive.
        pub death_time: f32 = 0.0,
        /// Vanilla `EntityRenderState.lightCoords` (`LightCoordsUtil.pack(block,
        /// sky)`): the packed block+sky light sampled at the entity's light-probe
        /// block position. Defaults to [`ENTITY_FULL_BRIGHT_LIGHT_COORDS`]; the
        /// entity scene projects the sampled value with the on-fire override.
        pub light_coords: u32 = ENTITY_FULL_BRIGHT_LIGHT_COORDS,
        /// Vanilla dispatcher-owned flame dimensions (`boundingBoxWidth`,
        /// `boundingBoxHeight`) when `EntityRenderState.displayFireAnimation` is
        /// true. `None` suppresses the solid flame feature.
        pub flame_dimensions: Option<[f32; 2]> = None,
        /// Vanilla `LivingEntityRenderState.hasRedOverlay` (`hurtTime > 0`): selects
        /// the red row of `OverlayTexture` so the entity flashes red when hurt.
        pub has_red_overlay: bool = false,
        /// Vanilla `CreeperRenderer.getWhiteOverlayProgress` (`0.0..=1.0`): selects
        /// the white-flash column of `OverlayTexture` so a priming creeper flashes
        /// white. `0.0` for every entity that is not flashing white.
        pub white_overlay_progress: f32 = 0.0,
        /// Vanilla `LivingEntityRenderState.isAutoSpinAttack` riptide spin: when the
        /// entity is mid-trident-spin, `Some(ageInTicks)` (the lerped
        /// `ageInTicks + partialTick`) drives the `LivingEntityRenderer.setupRotations`
        /// branch `Axis.XP.rotationDegrees(-90 - xRot)` then
        /// `Axis.YP.rotationDegrees(ageInTicks * -75)`. `None` for every entity that is
        /// not spinning (the death tip-over takes precedence over this branch).
        pub auto_spin_age_ticks: Option<f32> = None,
        /// Vanilla `LivingEntityRenderState.isUpsideDown` Dinnerbone/Grumm flip: when
        /// the entity is upside down, `Some(boundingBoxHeight)` drives the
        /// `LivingEntityRenderer.setupRotations` branch `translate(0, (bbHeight + 0.1) /
        /// entityScale, 0)` then `Axis.ZP.rotationDegrees(180)`. Carried as the world
        /// `boundingBoxHeight` because the post-yaw frame is already in world units (the
        /// model scale is applied innermost), so the `/ entityScale` is unnecessary.
        /// `None` for every entity that is not upside down (death and the riptide spin
        /// both take precedence over this branch).
        pub upside_down_height: Option<f32> = None,
        /// Vanilla `EntityRenderState.boundingBoxHeight` (`Entity.getBbHeight`), in world units.
        /// Drowned swim rotations use `bbHeight / 2 / entityScale` as their pivot after the
        /// standard living setup. `0.0` for callers that do not provide an AABB-derived height.
        pub bounding_box_height: f32 = 0.0,
        /// Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: when sleeping in a
        /// bed, the renderer skips the `180 - bodyRot` yaw and lays the model down via
        /// [`SleepingPose`]. `None` for every entity that is not sleeping. Death and
        /// the riptide spin take precedence over this branch; this branch takes
        /// precedence over the upside-down flip.
        pub sleeping: Option<SleepingPose> = None,
        /// Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the `SCALE`
        /// attribute): the uniform model scale `LivingEntityRenderer.submit` applies as
        /// `poseStack.scale(scale, scale, scale)` before `setupRotations`. `1.0` for an
        /// entity at its default size.
        pub scale: f32 = 1.0,
        /// Vanilla `LivingEntityRenderState.ageScale` (`LivingEntity.getAgeScale`): the independent
        /// adult/baby proportion multiplier consumed inside model `setupAnim`. It is not the SCALE
        /// attribute-backed [`scale`](Self::scale); adults and non-living entities use `1.0`.
        pub age_scale: f32 = 1.0,
        /// Vanilla `HumanoidRenderState.swimAmount` / `LivingEntity.getSwimAmount(partialTick)`:
        /// the eased 0..1 blend toward `isVisuallySwimming()`. The drowned consumes this for
        /// `DrownedRenderer.setupRotations` and the `DrownedModel.setupAnim` swim limb override.
        /// `0.0` for a living entity that is not visually swimming and every non-living entity.
        pub swim_amount: f32 = 0.0,
        /// Vanilla `HumanoidRenderState.speedValue`: fall-flying movement speed
        /// normalization used to divide the inherited humanoid limb swing.
        pub humanoid_speed_value: f32 = 1.0,
        /// Vanilla `LivingEntityRenderState.walkAnimationPos`
        /// (`WalkAnimationState.position(partialTick)`): the lerped limb-swing position
        /// that models feed into the `cos(animationPos * 0.6662 ...)` leg/arm sway in
        /// `setupAnim`. `0.0` for a standing entity.
        pub walk_animation_pos: f32 = 0.0,
        /// Vanilla `LivingEntityRenderState.walkAnimationSpeed`
        /// (`WalkAnimationState.speed(partialTick)`): the lerped limb-swing amplitude
        /// (`0.0..=1.0`) that scales the sway in `setupAnim`. `0.0` for a standing
        /// entity, leaving the model in its rest pose.
        pub walk_animation_speed: f32 = 0.0,
        /// Vanilla `LivingEntityRenderState.wornHeadAnimationPos`: the animation position passed to
        /// `SkullBlockRenderer.submitSkull` for skull equipment. It equals the entity walk
        /// animation position for non-passenger entities and the vehicle walk animation position
        /// when riding a living entity.
        pub worn_head_animation_pos: f32 = 0.0,
        /// Vanilla `IllagerRenderState.isRiding` (`entity.isPassenger()`): an illager riding any
        /// vehicle uses the fixed seated arm/leg pose before its arm-pose branch runs. `false` for
        /// every non-passenger entity; currently consumed by `IllagerModel`.
        pub is_riding: bool = false,
        /// Vanilla `HumanoidRenderState.attackTime` (`LivingEntity.getAttackAnim(partialTick)`):
        /// the lerped `0..1` melee swing progress that `HumanoidModel.setupAttackAnimation` turns
        /// into the body twist + arm whack (the off arm tracked via
        /// [`attack_arm_off_hand`](Self::attack_arm_off_hand)). `0.0` for an entity that is not
        /// mid-swing, leaving the arms on their walk/idle pose.
        pub attack_anim: f32 = 0.0,
        /// Vanilla `ArmedEntityRenderState.mainArm`: whether the entity's main arm is the left arm.
        /// The default `false` path is vanilla's right-handed default.
        pub main_arm_left: bool = false,
        /// Vanilla `HumanoidRenderState.attackArm` (`LivingEntity.swingingArm`): whether the active
        /// swing is the off (left) hand. `false` for a main-hand swing (the common case) and every
        /// entity that is not mid-swing.
        pub attack_arm_off_hand: bool = false,
        /// Vanilla `EntityRenderState.ageInTicks` (`entity.tickCount + partialTick`): the
        /// lerped per-frame age that drives continuous idle animations (e.g. the
        /// `AbstractPiglinModel` ear flap). `0.0` until the entity scene projects it.
        pub age_in_ticks: f32 = 0.0,
        /// Vanilla `Mob.isAggressive()` (`DATA_MOB_FLAGS_ID & 4`): deepens the held-out
        /// `animateZombieArms` arm drop for the zombie-model family (`-π / 1.5` aggressive vs
        /// `-π / 2.25` calm). `false` for every calm or non-zombie-family entity.
        pub is_aggressive: bool = false,
        /// Vanilla `SkeletonRenderState.isHoldingBow` (`getMainHandItem().is(Items.BOW)`): with
        /// [`is_aggressive`](Self::is_aggressive), `AbstractSkeletonRenderer.getArmPose` returns
        /// `BOW_AND_ARROW`, so `SkeletonModel` aims both arms forward along the head look. `false`
        /// for every non-skeleton entity and for a skeleton not holding a bow.
        pub main_hand_holds_bow: bool = false,
        /// Vanilla `HumanoidRenderState.swingAnimationType == STAB` for the current attack arm
        /// (`getItemHeldByArm(attackArm).getSwingAnimation().type()`): a spear makes
        /// `HumanoidModel.setupAttackAnimation` run `SpearAnimations.thirdPersonAttackHand` (the
        /// lunge/retract stab) instead of the default `WHACK` arm chop. `false` for every entity
        /// whose attack-arm item is not a spear. PlayerModel, zombie-family models, and the
        /// standard humanoid held-item layer consume it. The builder name is historical.
        pub main_hand_swing_is_stab: bool = false,
        /// Vanilla `HumanoidRenderState.swingAnimationType == NONE` for the current attack arm: the
        /// shared `HumanoidModel.setupAttackAnimation` body twist / arm-anchor prologue still runs
        /// for positive `attackTime`, but the WHACK arm chop and STAB lunge are skipped.
        pub main_hand_swing_is_none: bool = false,
        /// Vanilla `LivingEntityRenderState.ticksSinceKineticHitFeedback`: sampled from
        /// `LivingEntity.getTicksSinceLastKineticHitFeedback(partialTicks)` and consumed by
        /// `SpearAnimations.thirdPersonUseItem` to kick the held spear item backward after a
        /// kinetic hit. Defaults to `0.0`, the no-feedback case.
        pub ticks_since_kinetic_hit_feedback: f32 = 0.0,
        /// Vanilla `HumanoidMobRenderer.getArmPose` `SPEAR` for a non-player humanoid mob's main
        /// hand. The base mob renderer returns `SPEAR` for a held item tagged `minecraft:spears`
        /// (and for STAB while swinging); the zombie renderer additionally checks the opposite
        /// hand's STAB component. `false` for players and non-humanoid-mob renderers.
        pub humanoid_mob_main_hand_spear_pose: bool = false,
        /// Vanilla `HumanoidMobRenderer.getArmPose` `SPEAR` for a non-player humanoid mob's off
        /// hand. In the default right-handed, non-using `HumanoidModel.setupAnim` dispatch this
        /// pose runs first and, because `SPEAR.affectsOffhandPose()`, suppresses the main hand's
        /// pose. The zombie renderer can set this from a STAB item in the opposite main hand.
        pub humanoid_mob_off_hand_spear_pose: bool = false,
        /// Vanilla armor trim over the chest item's WINGS equipment model.
        pub chest_wings_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `ItemStack.hasFoil()` for the WINGS equipment item.
        pub chest_wings_foil: bool = false,
        /// Vanilla `LivingEntity.isUsingItem()`: true while the entity is actively
        /// using a hand item. Item-model `minecraft:using_item` conditions need
        /// this raw bit plus [`use_item_off_hand`](Self::use_item_off_hand) to
        /// identify the exact stack returned by `getUseItem()`.
        pub is_using_item: bool = false,
        /// Vanilla `LivingEntity.getUsedItemHand()` off-hand bit: which arm the use-item pose
        /// ([`player_using_spyglass`](Self::player_using_spyglass)) applies to. `false` (main /
        /// right arm) when not using an off-hand item.
        pub use_item_off_hand: bool = false,
        /// Vanilla render-state `ticksUsingItem`: the reconstructed `getTicksUsingItem()` counter
        /// used by crossbow `CROSSBOW_CHARGE` poses and the player's spear kinetic use sway.
        /// `AnimationUtils` consumers clamp it to their own durations. `0.0` for anything not using
        /// / charging.
        pub crossbow_charge_ticks: f32 = 0.0,
        /// Vanilla `CrossbowItem.getChargeDuration(stack, livingOwner)`: synchronized enchantment
        /// effects may replace the base 25-tick duration, so crossbow arm poses consume this
        /// projected per-owner value instead of a renderer constant.
        pub crossbow_charge_duration_ticks: f32 = 25.0,
        /// Vanilla `LivingEntityRenderState.wornHeadType`: a supported skull block item in the HEAD
        /// slot, rendered by `CustomHeadLayer` through `SkullBlockRenderer` instead of the generic
        /// item-model path. Static mob skulls, profileless default player heads, profiled
        /// player-head default skin fallbacks, and the specialized dragon/piglin skull models are
        /// projected; live player-skin downloads remain deferred.
        pub custom_head_skull: Option<EntityCustomHeadSkull> = None,
        /// Vanilla `HumanoidArmorLayer` worn armor, one generation-local equipment asset per slot.
        pub head_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub chest_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub legs_equipment_asset: Option<EntityEquipmentAssetId> = None,
        pub feet_equipment_asset: Option<EntityEquipmentAssetId> = None,
        /// Vanilla armor-trim atlas sprite per humanoid armor slot. Native publishes a value only
        /// after the startup pack's generated sprite has entered the shared runtime entity texture
        /// atlas.
        pub head_armor_trim: Option<EntityArmorTrim> = None,
        pub chest_armor_trim: Option<EntityArmorTrim> = None,
        pub legs_armor_trim: Option<EntityArmorTrim> = None,
        pub feet_armor_trim: Option<EntityArmorTrim> = None,
        /// Vanilla `DyedItemColor.getOrDefault` per worn armor slot: the worn item's `dyed_color`
        /// component (a packed RGB). The catalog layer decides whether to consume it.
        pub head_armor_dye: Option<u32> = None,
        pub chest_armor_dye: Option<u32> = None,
        pub legs_armor_dye: Option<u32> = None,
        pub feet_armor_dye: Option<u32> = None,
        /// Vanilla `ItemStack.hasFoil()` per worn armor slot. The equipment renderer emits one
        /// `armorEntityGlint` pass after the first rendered armor layer for that slot.
        pub head_armor_foil: bool = false,
        pub chest_armor_foil: bool = false,
        pub legs_armor_foil: bool = false,
        pub feet_armor_foil: bool = false,
        /// Effective chest-stack equipment asset consumed by `WINGS`.
        pub chest_wings_equipment_asset: Option<EntityEquipmentAssetId> = None,
        /// Vanilla `LivingEntityRenderState.isCrouching` (`Pose.CROUCHING`): a sneaking player,
        /// whose `HumanoidModel.setupAnim` leans the body forward, drops the head, tucks the legs
        /// back and tilts the arms. `false` for every other entity and for a standing player.
        pub is_crouching: bool = false,
        /// Vanilla `HumanoidRenderState.isFallFlying`, sourced from the base Entity
        /// fall-flying flag rather than inferred from equipped wings.
        pub is_fall_flying: bool = false,
        /// Vanilla `HumanoidRenderState.isVisuallySwimming`, including the
        /// FALL_FLYING-pose crawling fallback while not actually fall-flying.
        pub is_visually_swimming: bool = false,
        /// Vanilla `HumanoidRenderState.elytraRotX` from `LivingEntity.elytraAnimationState`.
        /// Defaults to `ElytraAnimationState`'s steady non-flying target (`PI/12`) until the native
        /// world projects the animated per-entity state.
        pub elytra_rot_x: f32 = ELYTRA_DEFAULT_X_ROT,
        /// Vanilla `HumanoidRenderState.elytraRotY`; steady non-flying target is `0`.
        pub elytra_rot_y: f32 = ELYTRA_DEFAULT_Y_ROT,
        /// Vanilla `HumanoidRenderState.elytraRotZ`; steady non-flying target is `-PI/12` and the
        /// right wing mirrors this value.
        pub elytra_rot_z: f32 = ELYTRA_DEFAULT_Z_ROT,
        /// Vanilla `LivingEntityRenderer.isBodyVisible`: a normally-invisible entity
        /// (Invisibility effect / `setInvisible`) draws no normal body. The textured
        /// path hides the model when [`Self::invisible_to_player`] is true, and uses
        /// `entityTranslucentCullItemTarget` for the base body when the entity is
        /// invisible but still visible to this client. Invisible glowing submissions
        /// record outline metadata and feed the static-atlas outline bucket, which is
        /// drawn through the renderer-owned `entity_outline` target before compositing
        /// back to the main surface.
        pub invisible: bool = false,
        /// Vanilla `LivingEntityRenderState.isInvisibleToPlayer`: `state.isInvisible
        /// && entity.isInvisibleTo(minecraft.player)`, so visible entities default to
        /// false. Use [`EntityModelInstance::with_invisible`] for the ordinary
        /// non-spectator hidden branch, then override this to false for the vanilla
        /// spectator/self-visible translucent branch.
        pub invisible_to_player: bool = false,
        /// Vanilla `EntityRenderState.appearsGlowing()` / client-side
        /// `Entity.isCurrentlyGlowing()`: when an invisible living entity is still
        /// invisible to this client, `LivingEntityRenderer.getRenderType` submits the
        /// base model with `RenderTypes.outline(texture)` instead of dropping it.
        pub appears_glowing: bool = false,
        /// Vanilla `EntityRenderState.outlineColor`: `0` when the entity does not
        /// appear glowing, otherwise `ARGB.opaque(entity.getTeamColor())`. The
        /// renderer records this on every texture-backed submission so later GPU
        /// outline work can preserve the official submit metadata.
        pub outline_color: u32 = 0,
        /// Vanilla `EntityRenderState.leashStates`: the leash ropes attached to this entity (one
        /// for a normal leash, four for a ghast-to-boat quad connection), each carrying the rotated
        /// attach offset, rope span, endpoint lights, and sag flag. `None` for unleashed entities.
        pub leash_ropes: Option<EntityLeashRopesRenderState> = None,
        /// Vanilla `LivingEntityRenderState.isInWater` / `FrogRenderState.isSwimming`
        /// (`entity.isInWater()`): a fish out of water thrashes harder and flops onto its side,
        /// while `FrogModel.setupAnim` switches from `FROG_WALK` to `FROG_SWIM` while in water.
        /// `false` (the Java default) for every entity until the entity scene projects it.
        pub in_water: bool = false,
        /// Vanilla `Entity.onGround()`: combined with [`in_water`](Self::in_water) to drive the
        /// vanilla `TurtleRenderer` `isOnLand = !isInWater && onGround` walk/swim leg branch.
        /// `false` (the Java default) for every entity until the entity scene projects it.
        pub on_ground: bool = false,
        /// The kind-specific render state; [`EntityKindRenderState::Other`] for kinds whose
        /// renderer reads only the shared fields.
        pub kind: EntityKindRenderState = EntityKindRenderState::Other,
    }
}

impl EntityRenderState {
    /// Projects the packed light coords into the renderer per-vertex lightmap
    /// input `[block, sky]`, each normalized to `0.0..=1.0`, mirroring the
    /// terrain mesh's `[block/15, sky/15]` shader light.
    pub fn shader_light(&self) -> [f32; 2] {
        let block = (self.light_coords >> 4) & 0xF;
        let sky = (self.light_coords >> 20) & 0xF;
        [block as f32 / 15.0, sky as f32 / 15.0]
    }

    /// Projects the entity overlay into the renderer per-vertex overlay coords
    /// `[u, v]` (vanilla `OverlayTexture.pack` channels). `u` is the white-flash
    /// column `OverlayTexture.u(whiteOverlayProgress)` = `(int)(progress * 15)`,
    /// `v` is `RED_OVERLAY_V` (`3`) when hurt and `WHITE_OVERLAY_V` (`10`, no red
    /// overlay) otherwise.
    pub fn overlay_coords(&self) -> [f32; 2] {
        let u = (self.white_overlay_progress.clamp(0.0, 1.0) * 15.0).floor();
        [u, if self.has_red_overlay { 3.0 } else { 10.0 }]
    }
}
