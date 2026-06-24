use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

use super::dimensions::vanilla_living_entity_type;
use super::dragon::{
    EnderDragonAnimationState, ENDER_DRAGON_PHASE_DATA_ID, ENDER_DRAGON_PHASE_HOVERING_ID,
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
};
use super::{EntityTransform, EntityVec3};

const VANILLA_ENTITY_TYPE_CHICKEN_ID: i32 = 26;
const VANILLA_ENTITY_TYPE_CREEPER_ID: i32 = 32;
const VANILLA_ENTITY_TYPE_GLOW_SQUID_ID: i32 = 61;
const VANILLA_ENTITY_TYPE_POLAR_BEAR_ID: i32 = 104;
const VANILLA_ENTITY_TYPE_SHEEP_ID: i32 = 111;
const VANILLA_ENTITY_TYPE_SHULKER_ID: i32 = 112;
const VANILLA_ENTITY_TYPE_SQUID_ID: i32 = 127;
const VANILLA_ENTITY_TYPE_WARDEN_ID: i32 = 142;
/// Vanilla `FlyingAnimal` implementors (`Bee`, `Parrot`): their
/// `LivingEntity.calculateEntityAnimation` measures the full 3-D travel distance
/// (`calculateEntityAnimation(this instanceof FlyingAnimal)`), so the limb-swing
/// distance includes the vertical component.
const VANILLA_ENTITY_TYPE_BEE_ID: i32 = 11;
const VANILLA_ENTITY_TYPE_PARROT_ID: i32 = 98;
/// Entities whose `updateWalkAnimation` override (`Camel`, `Creaking`, `Frog`)
/// replaces the base distance→speed mapping. `Camel`/`Frog` additionally gate on
/// pose/jump/dash animation states the client does not yet track, so their limb
/// swing is deferred rather than approximated with the base mapping.
const VANILLA_ENTITY_TYPE_CAMEL_ID: i32 = 19;
const VANILLA_ENTITY_TYPE_CREAKING_ID: i32 = 31;
const VANILLA_ENTITY_TYPE_FROG_ID: i32 = 55;
const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;
const POLAR_BEAR_STAND_ANIMATION_TICKS: f32 = 6.0;
/// Vanilla `Creeper.DATA_SWELL_DIR` (synced fuse direction, default `-1`) and
/// `DATA_IS_IGNITED`; the client advances `swell` toward `maxSwell` while the
/// effective direction is positive.
const CREEPER_SWELL_DIR_DATA_ID: u8 = 16;
const CREEPER_IGNITED_DATA_ID: u8 = 18;
/// Vanilla `Creeper.DEFAULT_MAX_SWELL`. `getSwelling` divides the lerped swell
/// by `maxSwell - 2`.
const CREEPER_MAX_SWELL: i32 = 30;
const SHULKER_PEEK_DATA_ID: u8 = 17;
const SHULKER_PEEK_PER_TICK: f32 = 0.05;
const SHULKER_MAX_PEEK_AMOUNT: f32 = 1.0;
/// Vanilla `Bee.DATA_FLAGS_ID` synced metadata id: `Entity` (`0..=7`),
/// `LivingEntity` (`8..=14`), `Mob` (`15`) and `AgeableMob` (`16..=17`) precede
/// the bee's own flags byte at `18` (the same slot `PolarBear` uses for its
/// standing flag). `FLAG_ROLL` is mask `2` within that byte.
const BEE_FLAGS_DATA_ID: u8 = 18;
const BEE_FLAG_ROLL: i8 = 2;
/// Vanilla `Fox` entity type id (`EntityType.FOX`).
const VANILLA_ENTITY_TYPE_FOX_ID: i32 = 54;
/// Vanilla `Fox.DATA_FLAGS_ID` synced metadata id: `Entity` (`0..=7`),
/// `LivingEntity` (`8..=14`), `Mob` (`15`) and `AgeableMob` (`16..=17`) precede
/// the fox's own `DATA_TYPE_ID` (`18`, the variant int) and then the flags byte
/// (`19`). `getFlag(mask) = (byte19 & mask) != 0`.
const FOX_FLAGS_DATA_ID: u8 = 19;
/// Vanilla `Fox.FLAG_CROUCHING`: the synced crouch flag that drives both the
/// client `crouchAmount` accumulator and the `FoxModel.setCrouchingPose`.
const FOX_FLAG_CROUCHING: i8 = 4;
/// Vanilla `Fox.FLAG_INTERESTED`: the synced interest flag that drives the
/// client `interestedAngle` accumulator (the head tilt). Not a render-state
/// field itself — only its eased angle is projected.
const FOX_FLAG_INTERESTED: i8 = 8;
/// Vanilla `Fox.interestedAngle` per-tick ease toward the `FLAG_INTERESTED`
/// target: `interestedAngle += (target - interestedAngle) * 0.4`.
const FOX_INTERESTED_EASE: f32 = 0.4;
/// Vanilla `Fox.getHeadRollAngle` scale: `lerp(pt, interestedAngleO,
/// interestedAngle) * 0.11 * π`.
const FOX_HEAD_ROLL_SCALE: f32 = 0.11;
/// Vanilla `Fox.crouchAmount` per-tick climb while `FLAG_CROUCHING` is set
/// (`crouchAmount += 0.2`).
const FOX_CROUCH_PER_TICK: f32 = 0.2;
/// Vanilla `Fox.MAX_CROUCH_AMOUNT`: the crouch accumulator saturates here.
const FOX_MAX_CROUCH_AMOUNT: f32 = 5.0;
/// Vanilla `LivingEntity.hurtDuration`: the hurt animation (and red damage
/// overlay) runs for 10 client ticks after a hurt animation or damage event.
const HURT_ANIMATION_DURATION: i32 = 10;
/// Vanilla `LivingEntity.DATA_HEALTH_ID` synced metadata id: `Entity` defines
/// ids `0..=7`, then `LivingEntity` adds the flags byte (8) and the health float
/// (9). `LivingEntity.isDeadOrDying` is `getHealth() <= 0`.
const VANILLA_ENTITY_HEALTH_DATA_ID: u8 = 9;
/// Vanilla `LivingEntity.tickDeath` removes the entity at `deathTime >= 20`, and
/// the death tip-over flip is fully clamped by then, so the client counter is
/// capped here to stay bounded when no server removal arrives.
const DEATH_ANIMATION_MAX_TICKS: i32 = 20;
/// Vanilla `Sheep.EAT_ANIMATION_TICKS`: the eat-grass animation runs for 40
/// client ticks after entity event `10`.
const SHEEP_EAT_ANIMATION_TICKS: i32 = 40;
/// Vanilla `Sheep.handleEntityEvent` triggers the eat-grass animation on event
/// id `10` (`EntityEvent.EAT_GRASS`).
const SHEEP_EAT_GRASS_EVENT_ID: i8 = 10;
/// Vanilla `Warden`: the client decrements `tendrilAnimation` from `10` toward
/// `0` each tick; `getTendrilAnimation` divides the lerped value by `10`.
const WARDEN_TENDRIL_ANIMATION_TICKS: i32 = 10;
/// Vanilla `Warden.handleEntityEvent` resets `tendrilAnimation` to `10` on event
/// id `61` (a received vibration signal).
const WARDEN_TENDRIL_EVENT_ID: i8 = 61;
/// Vanilla `Squid.handleEntityEvent` resets `tentacleMovement` to `0` on event id
/// `19` (`EntityEvent.SQUID_RESET_MOVEMENT`, broadcast each time the server-side
/// `tentacleMovement` wraps past `2π`). Without it the client tentacles freeze at
/// `2π` after the first cycle.
const SQUID_RESET_MOVEMENT_EVENT_ID: i8 = 19;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityClientAnimationState {
    #[serde(default)]
    pub age_ticks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polar_bear_standing: Option<PolarBearStandingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shulker_peek: Option<ShulkerPeekAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bee_roll: Option<BeeRollAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fox: Option<FoxAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ender_dragon: Option<EnderDragonAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheep_eat: Option<SheepEatAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hurt: Option<HurtAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death: Option<DeathAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creeper_swell: Option<CreeperSwellAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warden_tendril: Option<WardenTendrilAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub squid: Option<SquidAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chicken_flap: Option<ChickenFlapAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_animation: Option<WalkAnimationState>,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolarBearStandingAnimationState {
    pub target_standing: bool,
    pub previous_ticks: f32,
    pub current_ticks: f32,
    pub dimensions_ticks: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShulkerPeekAnimationState {
    pub target_peek_amount: f32,
    pub previous_peek_amount: f32,
    pub current_peek_amount: f32,
}

/// Canonical client-side bee barrel-roll animation, mirroring vanilla
/// `Bee.updateRollAmount`. While the synced `FLAG_ROLL` (mask `2` of the bee
/// flags byte, data id [`BEE_FLAGS_DATA_ID`]) is set, `rollAmount` climbs toward
/// `1` by `0.2`/tick; otherwise it falls toward `0` by `0.24`/tick.
/// `getRollAmount(partialTick)` lerps the pair, driving `BeeModel.setupAnim`'s
/// near-π `bone.xRot` flip that tips a rolling bee onto its back.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BeeRollAnimationState {
    pub rolling: bool,
    pub previous_roll_amount: f32,
    pub current_roll_amount: f32,
}

/// Canonical client-side fox accumulators, mirroring vanilla `Fox.tick`'s two
/// eased fields. While the synced `FLAG_INTERESTED` (mask `8`) is set,
/// `interestedAngle` eases toward `1` by `* 0.4`/tick (and toward `0` otherwise);
/// `getHeadRollAngle(pt) = lerp(pt, interestedAngleO, interestedAngle) * 0.11 * π`
/// drives the head tilt. While `FLAG_CROUCHING` (mask `4`) is set, `crouchAmount`
/// climbs by `0.2`/tick (clamped to `5.0`); otherwise it is reset INSTANTLY to
/// `0` (vanilla's non-crouching branch is an assignment, not a decay).
/// `getCrouchAmount(pt) = lerp(pt, crouchAmountO, crouchAmount)` drives
/// `FoxModel.setCrouchingPose`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FoxAnimationState {
    pub interested: bool,
    pub previous_interested_angle: f32,
    pub current_interested_angle: f32,
    pub crouching: bool,
    pub previous_crouch_amount: f32,
    pub current_crouch_amount: f32,
}

/// Canonical client-side sheep eat-grass animation countdown, mirroring vanilla
/// `Sheep.eatAnimationTick`. Entity event `10` resets it to
/// [`SHEEP_EAT_ANIMATION_TICKS`]; each client tick decrements it toward `0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheepEatAnimationState {
    pub eat_animation_tick: i32,
}

/// Canonical client-side warden tendril animation, mirroring vanilla
/// `Warden.tendrilAnimation`/`tendrilAnimationO`. Entity event `61` resets
/// `current` to [`WARDEN_TENDRIL_ANIMATION_TICKS`]; each client tick saves the
/// previous value and decrements `current` toward `0`. `getTendrilAnimation`
/// lerps the pair and divides by `10`, driving the renderer
/// `WardenModel.animateTendrils` antenna sway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WardenTendrilAnimationState {
    /// Vanilla `Warden.tendrilAnimationO`.
    pub previous: i32,
    /// Vanilla `Warden.tendrilAnimation`.
    pub current: i32,
}

/// Canonical client-side squid tentacle/body animation, mirroring vanilla
/// `Squid` (`xBodyRot`/`zBodyRot`/`tentacleMovement`/`tentacleAngle` and their
/// `O`/`old` lerp endpoints). `Squid.aiStep` advances `tentacleMovement` by the
/// id-seeded `tentacleSpeed` each client tick and derives the tentacle flex angle
/// and body pitch/roll; `SquidRenderer.extractRenderState` lerps the pairs.
///
/// Only the in-water branch is modelled (a squid is a water creature; the rare
/// out-of-water/suffocating branch is deferred — see [`Self::advance_client_tick`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SquidAnimationState {
    /// Vanilla `Squid.tentacleSpeed`, seeded once from the entity id in the
    /// constructor: `random.setSeed(getId()); 1 / (random.nextFloat() + 1) * 0.2`.
    #[serde(default)]
    pub tentacle_speed: f32,
    /// Vanilla `Squid.tentacleMovement`.
    #[serde(default)]
    pub tentacle_movement: f32,
    /// Vanilla `Squid.oldTentacleMovement`.
    #[serde(default)]
    pub old_tentacle_movement: f32,
    /// Vanilla `Squid.tentacleAngle`.
    #[serde(default)]
    pub tentacle_angle: f32,
    /// Vanilla `Squid.oldTentacleAngle`.
    #[serde(default)]
    pub old_tentacle_angle: f32,
    /// Vanilla `Squid.xBodyRot` (swim pitch, degrees).
    #[serde(default)]
    pub x_body_rot: f32,
    /// Vanilla `Squid.xBodyRotO`.
    #[serde(default)]
    pub old_x_body_rot: f32,
    /// Vanilla `Squid.zBodyRot` (swim roll, degrees).
    #[serde(default)]
    pub z_body_rot: f32,
    /// Vanilla `Squid.zBodyRotO`.
    #[serde(default)]
    pub old_z_body_rot: f32,
    /// Vanilla `Squid.rotateSpeed`.
    #[serde(default)]
    pub rotate_speed: f32,
}

/// Canonical client-side chicken wing-flap animation, mirroring vanilla
/// `Chicken` (`flap`/`oFlap`/`flapSpeed`/`oFlapSpeed`/`flapping`). `Chicken.aiStep`
/// drives `flapSpeed` toward `1` while airborne (toward `0` on the ground), keeps
/// `flapping` decaying at `0.9` per tick (re-seeded to `1` whenever the chicken
/// leaves the ground), and integrates `flap += flapping * 2`.
/// `ChickenRenderer.extractRenderState` lerps `flap`/`flapSpeed` by the partial
/// tick, and `ChickenModel.setupAnim` turns them into the wing `zRot`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChickenFlapAnimationState {
    /// Vanilla `Chicken.flap`.
    #[serde(default)]
    pub flap: f32,
    /// Vanilla `Chicken.oFlap`.
    #[serde(default)]
    pub o_flap: f32,
    /// Vanilla `Chicken.flapSpeed`.
    #[serde(default)]
    pub flap_speed: f32,
    /// Vanilla `Chicken.oFlapSpeed`.
    #[serde(default)]
    pub o_flap_speed: f32,
    /// Vanilla `Chicken.flapping` (field initializer `1.0`).
    #[serde(default)]
    pub flapping: f32,
}

/// Canonical client-side limb-swing accumulator, mirroring vanilla
/// `WalkAnimationState` (`net.minecraft.world.entity.WalkAnimationState`). Each
/// client tick `LivingEntity.calculateEntityAnimation` measures the entity's
/// per-tick travel and feeds it to `update`, which low-passes the speed and
/// integrates the swing position; the living-entity models read the lerped
/// `position`/`speed` to sway legs and arms. Tracked for every living entity
/// whose `updateWalkAnimation` is the base mapping.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WalkAnimationState {
    /// Vanilla `WalkAnimationState.speedOld`.
    pub speed_old: f32,
    /// Vanilla `WalkAnimationState.speed`.
    pub speed: f32,
    /// Vanilla `WalkAnimationState.position`.
    pub position: f32,
    /// Vanilla `WalkAnimationState.positionScale` (`1.0`, or `3.0` for a baby).
    pub position_scale: f32,
    /// The entity feet position recorded at the previous client tick, used to
    /// measure the per-tick travel distance (vanilla `Entity.xo/yo/zo`). `None`
    /// until the first tick, when vanilla's `xo == getX()` makes the travel zero.
    #[serde(default)]
    pub previous_feet_position: Option<EntityVec3>,
}

impl Default for WalkAnimationState {
    fn default() -> Self {
        // Vanilla `WalkAnimationState` initialises `positionScale = 1.0F`; every
        // other field defaults to `0`.
        Self {
            speed_old: 0.0,
            speed: 0.0,
            position: 0.0,
            position_scale: 1.0,
            previous_feet_position: None,
        }
    }
}

impl WalkAnimationState {
    /// Vanilla `WalkAnimationState.update`: `speedOld = speed; speed += (target -
    /// speed) * factor; position += speed; positionScale = positionScale`.
    fn update(&mut self, target_speed: f32, factor: f32, position_scale: f32) {
        self.speed_old = self.speed;
        self.speed += (target_speed - self.speed) * factor;
        self.position += self.speed;
        self.position_scale = position_scale;
    }

    /// Vanilla `WalkAnimationState.stop`: zeroes the speed and position (it leaves
    /// `positionScale` untouched, matching vanilla).
    fn stop(&mut self) {
        self.speed_old = 0.0;
        self.speed = 0.0;
        self.position = 0.0;
    }

    /// Advances one client tick from the entity's current feet position, mirroring
    /// vanilla `LivingEntity.calculateEntityAnimation` → `updateWalkAnimation` (the
    /// base mapping `targetSpeed = min(distance * 4, 1)`, `factor = 0.4`). `use_y`
    /// is `entity instanceof FlyingAnimal` (the `Mth.length` distance includes the
    /// vertical travel only for flying animals).
    fn advance_client_tick(
        &mut self,
        feet_position: EntityVec3,
        use_y: bool,
        is_passenger: bool,
        is_alive: bool,
        is_baby: bool,
    ) {
        let distance = match self.previous_feet_position {
            Some(previous) => {
                let dx = feet_position.x - previous.x;
                let dy = if use_y {
                    feet_position.y - previous.y
                } else {
                    0.0
                };
                let dz = feet_position.z - previous.z;
                (dx * dx + dy * dy + dz * dz).sqrt() as f32
            }
            None => 0.0,
        };
        self.previous_feet_position = Some(feet_position);
        // Vanilla `LivingEntity.calculateEntityAnimation`: only an alive, non-riding
        // entity animates; otherwise the swing is stopped.
        if is_passenger || !is_alive {
            self.stop();
            return;
        }
        let target_speed = (distance * 4.0).min(1.0);
        self.update(target_speed, 0.4, if is_baby { 3.0 } else { 1.0 });
    }

    /// Vanilla `WalkAnimationState.position(partialTicks)`: `(position - speed * (1
    /// - partialTicks)) * positionScale`.
    fn position(&self, partial_ticks: f32) -> f32 {
        (self.position - self.speed * (1.0 - partial_ticks)) * self.position_scale
    }

    /// Vanilla `WalkAnimationState.speed(partialTicks)`: `min(lerp(partialTicks,
    /// speedOld, speed), 1)`.
    fn speed(&self, partial_ticks: f32) -> f32 {
        (self.speed_old + (self.speed - self.speed_old) * partial_ticks).min(1.0)
    }
}

/// Vanilla `FlyingAnimal` test used by `calculateEntityAnimation` to decide
/// whether the limb-swing travel distance includes the vertical component.
fn is_flying_animal(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BEE_ID | VANILLA_ENTITY_TYPE_PARROT_ID
    )
}

/// Whether an entity's `updateWalkAnimation` override is not yet modelled, so its
/// limb swing is deferred rather than driven with the base distance→speed mapping.
fn walk_animation_override_is_deferred(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CREAKING_ID
            | VANILLA_ENTITY_TYPE_FROG_ID
    )
}

impl Default for PolarBearStandingAnimationState {
    fn default() -> Self {
        Self {
            target_standing: false,
            previous_ticks: 0.0,
            current_ticks: 0.0,
            dimensions_ticks: 0.0,
        }
    }
}

impl Default for ShulkerPeekAnimationState {
    fn default() -> Self {
        Self {
            target_peek_amount: 0.0,
            previous_peek_amount: 0.0,
            current_peek_amount: 0.0,
        }
    }
}

impl Default for BeeRollAnimationState {
    fn default() -> Self {
        Self {
            rolling: false,
            previous_roll_amount: 0.0,
            current_roll_amount: 0.0,
        }
    }
}

impl Default for FoxAnimationState {
    fn default() -> Self {
        Self {
            interested: false,
            previous_interested_angle: 0.0,
            current_interested_angle: 0.0,
            crouching: false,
            previous_crouch_amount: 0.0,
            current_crouch_amount: 0.0,
        }
    }
}

impl PolarBearStandingAnimationState {
    pub(crate) fn dimensions_height_scale(self) -> f32 {
        1.0 + self.dimensions_ticks / POLAR_BEAR_STAND_ANIMATION_TICKS
    }

    /// Vanilla `PolarBear.getStandingAnimationScale`: `lerp(partialTick,
    /// clientSideStandAnimationO, clientSideStandAnimation) / 6.0`.
    fn standing_animation_scale(self, partial_tick: f32) -> f32 {
        let ticks = self.previous_ticks + partial_tick * (self.current_ticks - self.previous_ticks);
        ticks / POLAR_BEAR_STAND_ANIMATION_TICKS
    }

    fn set_target(&mut self, target_standing: bool) {
        self.target_standing = target_standing;
    }

    fn advance_client_tick(&mut self) {
        if self.current_ticks != self.previous_ticks {
            self.dimensions_ticks = self.current_ticks;
        }

        self.previous_ticks = self.current_ticks;
        self.current_ticks = if self.target_standing {
            (self.current_ticks + 1.0).min(POLAR_BEAR_STAND_ANIMATION_TICKS)
        } else {
            (self.current_ticks - 1.0).max(0.0)
        };
    }
}

impl SheepEatAnimationState {
    /// Vanilla `Sheep.aiStep` on the client: `eatAnimationTick = max(0,
    /// eatAnimationTick - 1)`.
    fn advance_client_tick(&mut self) {
        self.eat_animation_tick = (self.eat_animation_tick - 1).max(0);
    }
}

impl WardenTendrilAnimationState {
    /// Vanilla `Warden.tick` client branch: `tendrilAnimationO = tendrilAnimation;
    /// if (tendrilAnimation > 0) tendrilAnimation--`.
    fn advance_client_tick(&mut self) {
        self.previous = self.current;
        if self.current > 0 {
            self.current -= 1;
        }
    }

    /// Whether the tendril pulse has fully settled at rest so the state can be
    /// dropped (both the lerp endpoints are back to zero).
    fn is_settled(&self) -> bool {
        self.previous == 0 && self.current == 0
    }

    /// Vanilla `Warden.getTendrilAnimation(partialTick)`:
    /// `lerp(partialTick, tendrilAnimationO, tendrilAnimation) / 10`.
    fn tendril_animation(self, partial_tick: f32) -> f32 {
        let lerped = self.previous as f32 + partial_tick * (self.current - self.previous) as f32;
        lerped / WARDEN_TENDRIL_ANIMATION_TICKS as f32
    }
}

/// Java `java.util.Random` LCG `setSeed(seed)` then `nextFloat()`, used to seed
/// `Squid.tentacleSpeed` from the entity id deterministically on the client.
/// Replicates the multiplier/addend/mask of `LevelEventSoundRandomState` (the
/// audio module's Java-compatible LCG) so the value matches vanilla exactly.
fn java_random_first_next_float(seed: i64) -> f32 {
    const MULTIPLIER: u64 = 0x5DEEC_E66D;
    const ADDEND: u64 = 0xB;
    const MASK: u64 = (1_u64 << 48) - 1;
    // `setSeed(seed)`.
    let mut state = ((seed as u64) ^ MULTIPLIER) & MASK;
    // `next(24)`.
    state = state.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
    let bits = (state >> (48 - 24)) as u32;
    // `nextFloat() = next(24) / (1 << 24)`.
    bits as f32 / (1_u32 << 24) as f32
}

impl SquidAnimationState {
    /// Vanilla `Squid` constructor: `random.setSeed(getId()); tentacleSpeed = 1 /
    /// (random.nextFloat() + 1) * 0.2`. Seeding by the entity id makes the speed
    /// deterministic on the client (the `aiStep` re-randomization is server-only).
    pub(crate) fn new(entity_id: i32) -> Self {
        let tentacle_speed = 1.0 / (java_random_first_next_float(i64::from(entity_id)) + 1.0) * 0.2;
        Self {
            tentacle_speed,
            tentacle_movement: 0.0,
            old_tentacle_movement: 0.0,
            tentacle_angle: 0.0,
            old_tentacle_angle: 0.0,
            x_body_rot: 0.0,
            old_x_body_rot: 0.0,
            z_body_rot: 0.0,
            old_z_body_rot: 0.0,
            rotate_speed: 0.0,
        }
    }

    /// Vanilla `Squid.handleEntityEvent(19)`: resets `tentacleMovement = 0`, the
    /// per-cycle reset broadcast by the server when its counter wraps past `2π`.
    fn reset_movement(&mut self) {
        self.tentacle_movement = 0.0;
    }

    /// Advances one client tick of `Squid.aiStep`, in-water branch.
    ///
    /// Vanilla saves the lerp endpoints (`xBodyRotO`/`zBodyRotO`/
    /// `oldTentacleMovement`/`oldTentacleAngle`), advances `tentacleMovement` by
    /// `tentacleSpeed`, and on the client clamps it at `2π` (the server instead
    /// resets to `0` and broadcasts event `19`). The in-water branch then derives
    /// `tentacleAngle`/`rotateSpeed` from the half-cycle position and turns the
    /// body roll/pitch from the synced velocity.
    ///
    /// DEFERRED: the out-of-water/suffocating branch (`!isInWater()`:
    /// `tentacleAngle = abs(sin(tentacleMovement)) * π * 0.25` and `xBodyRot`
    /// easing toward `-90°`) is not modelled — a squid is a water creature, so the
    /// in-water branch is the always-relevant case. The vanilla movement-derived
    /// `yBodyRot` refinement is also deferred (that is the entity body yaw, which
    /// bbb projects separately and must not be perturbed here).
    fn advance_client_tick(&mut self, delta_movement: EntityVec3) {
        use std::f32::consts::{PI, TAU};

        self.old_x_body_rot = self.x_body_rot;
        self.old_z_body_rot = self.z_body_rot;
        self.old_tentacle_movement = self.tentacle_movement;
        self.old_tentacle_angle = self.tentacle_angle;
        self.tentacle_movement += self.tentacle_speed;
        if self.tentacle_movement > TAU {
            // Client clamp (server resets to 0 and broadcasts event 19).
            self.tentacle_movement = TAU;
        }

        if self.tentacle_movement < PI {
            let scale = self.tentacle_movement / PI;
            self.tentacle_angle = (scale * scale * PI).sin() * PI * 0.25;
            if scale > 0.75 {
                self.rotate_speed = 1.0;
            } else {
                self.rotate_speed *= 0.8;
            }
        } else {
            self.tentacle_angle = 0.0;
            self.rotate_speed *= 0.99;
        }

        let horizontal = (delta_movement.x * delta_movement.x + delta_movement.z * delta_movement.z)
            .sqrt() as f32;
        self.z_body_rot += PI * self.rotate_speed * 1.5;
        self.x_body_rot +=
            (-(horizontal.atan2(delta_movement.y as f32)) * (180.0 / PI) - self.x_body_rot) * 0.1;
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks,
    /// oldTentacleAngle, tentacleAngle)`.
    fn tentacle_angle(&self, partial_tick: f32) -> f32 {
        self.old_tentacle_angle + (self.tentacle_angle - self.old_tentacle_angle) * partial_tick
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks, xBodyRotO,
    /// xBodyRot)`.
    fn x_body_rot(&self, partial_tick: f32) -> f32 {
        self.old_x_body_rot + (self.x_body_rot - self.old_x_body_rot) * partial_tick
    }

    /// Vanilla `SquidRenderer.extractRenderState`: `lerp(partialTicks, zBodyRotO,
    /// zBodyRot)`.
    fn z_body_rot(&self, partial_tick: f32) -> f32 {
        self.old_z_body_rot + (self.z_body_rot - self.old_z_body_rot) * partial_tick
    }
}

impl Default for ChickenFlapAnimationState {
    fn default() -> Self {
        // Vanilla `Chicken` field initializers: `flapping = 1.0F`; every other flap
        // field defaults to `0`.
        Self {
            flap: 0.0,
            o_flap: 0.0,
            flap_speed: 0.0,
            o_flap_speed: 0.0,
            flapping: 1.0,
        }
    }
}

impl ChickenFlapAnimationState {
    /// Advances one client tick of `Chicken.aiStep`'s wing-flap accumulator.
    ///
    /// Vanilla saves the lerp endpoints (`oFlap`/`oFlapSpeed`), drives `flapSpeed`
    /// toward `1` while airborne (toward `0` on the ground) clamped to `0..=1`,
    /// re-seeds `flapping` to `1` whenever the chicken is airborne and decays it by
    /// `0.9`, then integrates `flap += flapping * 2`. The `deltaMovement` tweak in
    /// the same method is server physics and is intentionally not modelled here.
    fn advance_client_tick(&mut self, on_ground: bool) {
        self.o_flap = self.flap;
        self.o_flap_speed = self.flap_speed;
        self.flap_speed += if on_ground { -1.0 } else { 4.0 } * 0.3;
        self.flap_speed = self.flap_speed.clamp(0.0, 1.0);
        if !on_ground && self.flapping < 1.0 {
            self.flapping = 1.0;
        }
        self.flapping *= 0.9;
        self.flap += self.flapping * 2.0;
    }

    /// Vanilla `ChickenRenderer.extractRenderState`: `lerp(partialTicks, oFlap,
    /// flap)`.
    fn flap(&self, partial_tick: f32) -> f32 {
        self.o_flap + (self.flap - self.o_flap) * partial_tick
    }

    /// Vanilla `ChickenRenderer.extractRenderState`: `lerp(partialTicks, oFlapSpeed,
    /// flapSpeed)`.
    fn flap_speed(&self, partial_tick: f32) -> f32 {
        self.o_flap_speed + (self.flap_speed - self.o_flap_speed) * partial_tick
    }
}

impl CreeperSwellAnimationState {
    fn set_swell_dir(&mut self, swell_dir: i32) {
        self.swell_dir = swell_dir;
    }

    /// Vanilla `Creeper.tick`: `oldSwell = swell; swell += swellDir`, clamped to
    /// `0..=maxSwell`.
    fn advance_client_tick(&mut self) {
        self.previous_swell = self.current_swell;
        self.current_swell = (self.current_swell + self.swell_dir).clamp(0, CREEPER_MAX_SWELL);
    }

    /// Whether the fuse has fully settled at rest so the state can be dropped.
    fn is_settled(&self) -> bool {
        self.swell_dir <= 0 && self.current_swell == 0 && self.previous_swell == 0
    }

    /// Vanilla `Creeper.getSwelling`: `lerp(partialTick, oldSwell, swell) /
    /// (maxSwell - 2)`.
    fn swelling(self, partial_tick: f32) -> f32 {
        let lerped = self.previous_swell as f32
            + partial_tick * (self.current_swell - self.previous_swell) as f32;
        lerped / (CREEPER_MAX_SWELL as f32 - 2.0)
    }
}

impl ShulkerPeekAnimationState {
    fn set_target(&mut self, target_peek_amount: f32) {
        self.target_peek_amount = target_peek_amount;
    }

    fn advance_client_tick(&mut self) {
        self.previous_peek_amount = self.current_peek_amount;
        if self.current_peek_amount == self.target_peek_amount {
            return;
        }

        self.current_peek_amount = if self.current_peek_amount > self.target_peek_amount {
            (self.current_peek_amount - SHULKER_PEEK_PER_TICK)
                .clamp(self.target_peek_amount, SHULKER_MAX_PEEK_AMOUNT)
        } else {
            (self.current_peek_amount + SHULKER_PEEK_PER_TICK).clamp(0.0, self.target_peek_amount)
        };
    }

    /// Vanilla `Shulker.getClientPeekAmount(partialTick)` =
    /// `Mth.lerp(partialTick, currentPeekAmountO, currentPeekAmount)`.
    fn peek_amount(self, partial_tick: f32) -> f32 {
        self.previous_peek_amount
            + partial_tick * (self.current_peek_amount - self.previous_peek_amount)
    }
}

impl BeeRollAnimationState {
    fn set_rolling(&mut self, rolling: bool) {
        self.rolling = rolling;
    }

    /// Vanilla `Bee.updateRollAmount`: `rollAmountO = rollAmount`, then a rolling
    /// bee climbs `min(1, rollAmount + 0.2)` while a non-rolling one decays
    /// `max(0, rollAmount - 0.24)`.
    fn advance_client_tick(&mut self) {
        self.previous_roll_amount = self.current_roll_amount;
        self.current_roll_amount = if self.rolling {
            (self.current_roll_amount + 0.2).min(1.0)
        } else {
            (self.current_roll_amount - 0.24).max(0.0)
        };
    }

    /// Vanilla `Bee.getRollAmount(partialTick)` =
    /// `Mth.lerp(partialTick, rollAmountO, rollAmount)`.
    fn roll_amount(self, partial_tick: f32) -> f32 {
        self.previous_roll_amount
            + partial_tick * (self.current_roll_amount - self.previous_roll_amount)
    }
}

impl FoxAnimationState {
    fn set_flags(&mut self, interested: bool, crouching: bool) {
        self.interested = interested;
        self.crouching = crouching;
    }

    /// Vanilla `Fox.tick`: `interestedAngleO = interestedAngle`, then it eases
    /// toward `isInterested ? 1 : 0` by `* 0.4`/tick. Independently,
    /// `crouchAmountO = crouchAmount`, then a crouching fox climbs by `0.2`/tick
    /// (clamped to `MAX_CROUCH_AMOUNT`) while a non-crouching one is reset INSTANTLY
    /// to `0` (an assignment, not a decay).
    fn advance_client_tick(&mut self) {
        self.previous_interested_angle = self.current_interested_angle;
        let target = if self.interested { 1.0 } else { 0.0 };
        self.current_interested_angle +=
            (target - self.current_interested_angle) * FOX_INTERESTED_EASE;

        self.previous_crouch_amount = self.current_crouch_amount;
        if self.crouching {
            self.current_crouch_amount =
                (self.current_crouch_amount + FOX_CROUCH_PER_TICK).min(FOX_MAX_CROUCH_AMOUNT);
        } else {
            self.current_crouch_amount = 0.0;
        }
    }

    /// Vanilla `Fox.getHeadRollAngle(partialTick)` =
    /// `Mth.lerp(partialTick, interestedAngleO, interestedAngle) * 0.11 * π`.
    fn head_roll_angle(self, partial_tick: f32) -> f32 {
        let lerped = self.previous_interested_angle
            + partial_tick * (self.current_interested_angle - self.previous_interested_angle);
        lerped * FOX_HEAD_ROLL_SCALE * std::f32::consts::PI
    }

    /// Vanilla `Fox.getCrouchAmount(partialTick)` =
    /// `Mth.lerp(partialTick, crouchAmountO, crouchAmount)`.
    fn crouch_amount(self, partial_tick: f32) -> f32 {
        self.previous_crouch_amount
            + partial_tick * (self.current_crouch_amount - self.previous_crouch_amount)
    }
}

impl EntityClientAnimationState {
    pub(crate) fn sync_targets_from_metadata(
        &mut self,
        entity_type_id: i32,
        data_values: &[EntityDataValue],
    ) {
        // Vanilla `LivingEntity.isDeadOrDying()` (`getHealth() <= 0`): a living
        // entity whose synced health has reached zero begins the death animation;
        // restoring health clears it. Only living entities carry the health float,
        // so the per-type model animations below are unaffected.
        if vanilla_living_entity_type(entity_type_id) {
            if let Some(health) = entity_data_float(data_values, VANILLA_ENTITY_HEALTH_DATA_ID) {
                if health <= 0.0 {
                    self.death
                        .get_or_insert(DeathAnimationState { death_time: 0 });
                } else {
                    self.death = None;
                }
            }
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                let target_standing =
                    entity_data_bool(data_values, POLAR_BEAR_STANDING_DATA_ID, false);
                if let Some(standing) = self.polar_bear_standing.as_mut() {
                    standing.set_target(target_standing);
                } else if target_standing {
                    self.polar_bear_standing = Some(PolarBearStandingAnimationState {
                        target_standing,
                        ..PolarBearStandingAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_CREEPER_ID => {
                let swell_dir = creeper_effective_swell_dir(data_values);
                if let Some(swell) = self.creeper_swell.as_mut() {
                    swell.set_swell_dir(swell_dir);
                } else if swell_dir > 0 {
                    self.creeper_swell = Some(CreeperSwellAnimationState {
                        swell_dir,
                        previous_swell: 0,
                        current_swell: 0,
                    });
                }
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                let target_peek_amount = shulker_target_peek_amount(data_values);
                if let Some(peek) = self.shulker_peek.as_mut() {
                    peek.set_target(target_peek_amount);
                } else if target_peek_amount > 0.0 {
                    self.shulker_peek = Some(ShulkerPeekAnimationState {
                        target_peek_amount,
                        ..ShulkerPeekAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_BEE_ID => {
                let rolling = bee_is_rolling(data_values);
                if let Some(roll) = self.bee_roll.as_mut() {
                    roll.set_rolling(rolling);
                } else if rolling {
                    self.bee_roll = Some(BeeRollAnimationState {
                        rolling,
                        ..BeeRollAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_FOX_ID => {
                let interested = fox_is_interested(data_values);
                let crouching = fox_is_crouching(data_values);
                if let Some(fox) = self.fox.as_mut() {
                    fox.set_flags(interested, crouching);
                } else if interested || crouching {
                    self.fox = Some(FoxAnimationState {
                        interested,
                        crouching,
                        ..FoxAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
                let phase_id = entity_data_int(
                    data_values,
                    ENDER_DRAGON_PHASE_DATA_ID,
                    ENDER_DRAGON_PHASE_HOVERING_ID,
                );
                if let Some(dragon) = self.ender_dragon.as_mut() {
                    dragon.set_phase(phase_id);
                } else {
                    self.ender_dragon = Some(EnderDragonAnimationState {
                        phase_id,
                        ..EnderDragonAnimationState::default()
                    });
                }
            }
            _ => {}
        }
    }

    /// Projects a client entity event into client animation state. Vanilla
    /// `Sheep.handleEntityEvent` resets the eat-grass animation on event `10`.
    pub(crate) fn handle_entity_event(&mut self, entity_type_id: i32, event_id: i8) {
        if entity_type_id == VANILLA_ENTITY_TYPE_SHEEP_ID && event_id == SHEEP_EAT_GRASS_EVENT_ID {
            self.sheep_eat = Some(SheepEatAnimationState {
                eat_animation_tick: SHEEP_EAT_ANIMATION_TICKS,
            });
        } else if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
            && event_id == WARDEN_TENDRIL_EVENT_ID
        {
            // Vanilla `Warden.handleEntityEvent(61)`: `tendrilAnimation = 10`. Only
            // `tendrilAnimation` (`current`) is set; `tendrilAnimationO` (`previous`)
            // is untouched, so the lerp fades in from the prior frame.
            self.warden_tendril
                .get_or_insert(WardenTendrilAnimationState {
                    previous: 0,
                    current: 0,
                })
                .current = WARDEN_TENDRIL_ANIMATION_TICKS;
        } else if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
        ) && event_id == SQUID_RESET_MOVEMENT_EVENT_ID
        {
            // Vanilla `Squid.handleEntityEvent(19)`: `tentacleMovement = 0`. Only an
            // already-ticked squid has a state; if the event arrives first the
            // animation will start fresh on the next tick anyway, so nothing to do.
            if let Some(squid) = self.squid.as_mut() {
                squid.reset_movement();
            }
        }
    }

    /// Vanilla `Sheep.eatAnimationTick`, exposed for renderer head-pose
    /// projection. Returns `0` when the sheep is not currently eating.
    pub fn sheep_eat_animation_tick(&self) -> i32 {
        self.sheep_eat.map_or(0, |state| state.eat_animation_tick)
    }

    /// Vanilla `Warden.getTendrilAnimation(partialTick)`, exposed for the renderer
    /// `WardenModel.animateTendrils` sway. Returns `0.0` when the entity is not a
    /// warden with an active tendril pulse.
    pub fn warden_tendril_animation(&self, partial_tick: f32) -> f32 {
        self.warden_tendril
            .map_or(0.0, |state| state.tendril_animation(partial_tick))
    }

    /// Vanilla `Creeper.getSwelling(partialTick)`, exposed for the renderer white
    /// swelling overlay. Returns `0.0` when the entity is not a priming creeper.
    pub fn creeper_swelling(&self, partial_tick: f32) -> f32 {
        self.creeper_swell
            .map_or(0.0, |state| state.swelling(partial_tick))
    }

    /// Vanilla `Shulker.getClientPeekAmount(partialTick)`, exposed for the renderer
    /// `ShulkerModel.setupAnim` lid open/close. Returns `0.0` (the closed/bind pose) when
    /// the entity is not a shulker or its lid is shut.
    pub fn shulker_peek_amount(&self, partial_tick: f32) -> f32 {
        self.shulker_peek
            .map_or(0.0, |state| state.peek_amount(partial_tick))
    }

    /// Vanilla `Bee.getRollAmount(partialTick)` projected for the renderer
    /// `BeeModel.setupAnim` barrel-roll flip. Returns `0.0` when the entity is
    /// not a rolling bee.
    pub fn bee_roll_amount(&self, partial_tick: f32) -> f32 {
        self.bee_roll
            .map_or(0.0, |state| state.roll_amount(partial_tick))
    }

    /// Vanilla `Fox.getHeadRollAngle(partialTick)` projected for the renderer
    /// `FoxModel.setWalkingPose` head tilt. Returns `0.0` when the entity is not an
    /// interested fox.
    pub fn fox_head_roll_angle(&self, partial_tick: f32) -> f32 {
        self.fox
            .map_or(0.0, |state| state.head_roll_angle(partial_tick))
    }

    /// Vanilla `Fox.getCrouchAmount(partialTick)` projected for the renderer
    /// `FoxModel.setCrouchingPose` body drop. Returns `0.0` when the entity is not a
    /// crouching fox.
    pub fn fox_crouch_amount(&self, partial_tick: f32) -> f32 {
        self.fox
            .map_or(0.0, |state| state.crouch_amount(partial_tick))
    }

    /// Resets the hurt animation countdown to [`HURT_ANIMATION_DURATION`],
    /// mirroring vanilla `LivingEntity.animateHurt` / `handleDamageEvent` setting
    /// `hurtTime = hurtDuration`.
    pub(crate) fn trigger_hurt(&mut self) {
        self.hurt = Some(HurtAnimationState {
            hurt_time: HURT_ANIMATION_DURATION,
        });
    }

    /// Vanilla `LivingEntityRenderer.extractRenderState`:
    /// `hasRedOverlay = hurtTime > 0 || deathTime > 0`.
    pub fn has_red_overlay(&self) -> bool {
        self.hurt.is_some_and(|state| state.hurt_time > 0)
            || self.death.is_some_and(|state| state.death_time > 0)
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

    /// Vanilla `PolarBear.getStandingAnimationScale(partialTick)` projected for
    /// the renderer `PolarBearModel.setupAnim` standing pose. Returns `0.0` when
    /// the entity is not a rearing polar bear.
    pub fn polar_bear_stand_scale(&self, partial_tick: f32) -> f32 {
        self.polar_bear_standing
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
        self.squid
            .map_or(0.0, |squid| squid.tentacle_angle(partial_tick))
    }

    /// Vanilla `SquidRenderState.xBodyRot` (`Squid.xBodyRot` lerped, degrees): the
    /// swim pitch `SquidRenderer.setupRotations` applies to the squid root.
    /// Returns `0.0` for every non-squid entity.
    pub fn squid_x_body_rot(&self, partial_tick: f32) -> f32 {
        self.squid
            .map_or(0.0, |squid| squid.x_body_rot(partial_tick))
    }

    /// Vanilla `SquidRenderState.zBodyRot` (`Squid.zBodyRot` lerped, degrees): the
    /// swim roll `SquidRenderer.setupRotations` applies to the squid root. Returns
    /// `0.0` for every non-squid entity.
    pub fn squid_z_body_rot(&self, partial_tick: f32) -> f32 {
        self.squid
            .map_or(0.0, |squid| squid.z_body_rot(partial_tick))
    }

    /// Vanilla `ChickenRenderState.flap` (`Chicken.flap` lerped): the wing-flap
    /// phase `ChickenModel.setupAnim` feeds to `(sin(flap) + 1) * flapSpeed`.
    /// Returns `0.0` for every non-chicken entity (and an unticked chicken).
    pub fn chicken_flap(&self, partial_tick: f32) -> f32 {
        self.chicken_flap
            .map_or(0.0, |flap| flap.flap(partial_tick))
    }

    /// Vanilla `ChickenRenderState.flapSpeed` (`Chicken.flapSpeed` lerped): the
    /// wing-flap amplitude `ChickenModel.setupAnim` multiplies the flap phase by.
    /// Returns `0.0` for every non-chicken entity, so the wings hold the bind pose.
    pub fn chicken_flap_speed(&self, partial_tick: f32) -> f32 {
        self.chicken_flap
            .map_or(0.0, |flap| flap.flap_speed(partial_tick))
    }

    pub(crate) fn advance_client_tick(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        transform: EntityTransform,
        is_passenger: bool,
        is_baby: bool,
    ) {
        self.age_ticks = self.age_ticks.saturating_add(1);
        // Vanilla `LivingEntity.baseTick`: `if (hurtTime > 0) hurtTime--`. Applies
        // to every living entity, so it runs outside the per-type match.
        if let Some(hurt) = self.hurt.as_mut() {
            hurt.hurt_time = (hurt.hurt_time - 1).max(0);
            if hurt.hurt_time == 0 {
                self.hurt = None;
            }
        }
        // Vanilla `LivingEntity.tickDeath`: `deathTime++` each tick while dying.
        // The death state is only present for a dying living entity (set from the
        // synced health), so this also runs outside the per-type match.
        if let Some(death) = self.death.as_mut() {
            death.death_time = (death.death_time + 1).min(DEATH_ANIMATION_MAX_TICKS);
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_CREEPER_ID => {
                if let Some(swell) = self.creeper_swell.as_mut() {
                    swell.advance_client_tick();
                    if swell.is_settled() {
                        self.creeper_swell = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                if let Some(standing) = self.polar_bear_standing.as_mut() {
                    standing.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_SHEEP_ID => {
                if let Some(eat) = self.sheep_eat.as_mut() {
                    eat.advance_client_tick();
                    if eat.eat_animation_tick == 0 {
                        self.sheep_eat = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                if let Some(peek) = self.shulker_peek.as_mut() {
                    peek.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_BEE_ID => {
                if let Some(roll) = self.bee_roll.as_mut() {
                    roll.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_FOX_ID => {
                if let Some(fox) = self.fox.as_mut() {
                    fox.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_WARDEN_ID => {
                if let Some(tendril) = self.warden_tendril.as_mut() {
                    tendril.advance_client_tick();
                    if tendril.is_settled() {
                        self.warden_tendril = None;
                    }
                }
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => self
                .ender_dragon
                .get_or_insert_with(EnderDragonAnimationState::default)
                .advance_client_tick(transform),
            VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => self
                .squid
                .get_or_insert_with(|| SquidAnimationState::new(entity_id))
                .advance_client_tick(transform.delta_movement),
            VANILLA_ENTITY_TYPE_CHICKEN_ID => self
                .chicken_flap
                .get_or_insert_with(ChickenFlapAnimationState::default)
                // Vanilla `Chicken.aiStep` reads `onGround()`. A chicken with no
                // synced ground flag defaults to on-ground (wings still), the safe
                // common case.
                .advance_client_tick(transform.on_ground.unwrap_or(true)),
            _ => {}
        }
        // Vanilla `LivingEntity.calculateEntityAnimation` runs every client tick
        // (`aiStep`, or `RemotePlayer.tick`) for every living entity, feeding the
        // per-tick travel to the limb-swing accumulator. Entities whose
        // `updateWalkAnimation` override is not yet modelled are left deferred.
        if vanilla_living_entity_type(entity_type_id)
            && !walk_animation_override_is_deferred(entity_type_id)
        {
            // Vanilla `isAlive() = getHealth() > 0`: the client death animation
            // state is present exactly while the entity is dead/dying.
            let is_alive = self.death.is_none();
            let use_y = is_flying_animal(entity_type_id);
            self.walk_animation
                .get_or_insert_with(WalkAnimationState::default)
                .advance_client_tick(transform.position, use_y, is_passenger, is_alive, is_baby);
        }
    }
}

impl WorldStore {
    pub fn advance_entity_client_animations(&mut self, ticks: u32) {
        self.entities.advance_client_animations(ticks);
    }
}

fn entity_data_bool(data_values: &[EntityDataValue], data_id: u8, default: bool) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn shulker_target_peek_amount(data_values: &[EntityDataValue]) -> f32 {
    f32::from(entity_data_byte(data_values, SHULKER_PEEK_DATA_ID, 0).clamp(0, 100)) * 0.01
}

/// Vanilla `Bee.isRolling()` = `getFlag(FLAG_ROLL)` = `(flags & 2) != 0`.
fn bee_is_rolling(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, BEE_FLAGS_DATA_ID, 0) & BEE_FLAG_ROLL != 0
}

/// Vanilla `Fox.isInterested()` = `getFlag(FLAG_INTERESTED)` = `(flags & 8) != 0`.
fn fox_is_interested(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, FOX_FLAGS_DATA_ID, 0) & FOX_FLAG_INTERESTED != 0
}

/// Vanilla `Fox.isCrouching()` = `getFlag(FLAG_CROUCHING)` = `(flags & 4) != 0`.
fn fox_is_crouching(data_values: &[EntityDataValue]) -> bool {
    entity_data_byte(data_values, FOX_FLAGS_DATA_ID, 0) & FOX_FLAG_CROUCHING != 0
}

/// Vanilla `Creeper.tick`: the fuse advances by `getSwellDir()`, but an ignited
/// creeper forces the direction to `1`. Mirrors that effective direction from
/// the synced `DATA_SWELL_DIR` (default `-1`) and `DATA_IS_IGNITED`.
fn creeper_effective_swell_dir(data_values: &[EntityDataValue]) -> i32 {
    if entity_data_bool(data_values, CREEPER_IGNITED_DATA_ID, false) {
        1
    } else {
        entity_data_int(data_values, CREEPER_SWELL_DIR_DATA_ID, -1)
    }
}

fn entity_data_byte(data_values: &[EntityDataValue], data_id: u8, default: i8) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_float(data_values: &[EntityDataValue], data_id: u8) -> Option<f32> {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Float(value) => Some(*value),
            _ => None,
        })
}

fn entity_data_int(data_values: &[EntityDataValue], data_id: u8, fallback: i32) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}
