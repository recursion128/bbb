use super::*;
use crate::registries::BlockStateInfo;

pub use bbb_protocol::packets::Vec3d as EntityVec3;

#[derive(Debug, Clone, PartialEq)]
pub struct ItemEntityStackState<'a> {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub light: TerrainLight,
    /// Constructor-stable `ItemEntity.bobOffs`; zero for non-item users of the
    /// generic ITEM_STACK projection.
    pub bob_offset: f32,
    pub stack: &'a ProtocolItemStackSummary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FireworkRocketItemState<'a> {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub light: TerrainLight,
    pub stack: &'a ProtocolItemStackSummary,
    pub shot_at_angle: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FireworkRocketExplosionParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
    #[serde(default)]
    pub has_explosions: bool,
    #[serde(default)]
    pub explosions: Vec<ProtocolFireworkExplosionSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireworkRocketTrailParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EyeOfEnderTrailParticleState {
    pub entity_id: i32,
    pub origin: EntityVec3,
    pub delta_movement: EntityVec3,
    #[serde(default)]
    pub in_water: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShulkerBulletTrailParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementSimpleParticleKind {
    Bubble,
    Crit,
    Smoke,
    LargeSmoke,
    DragonBreath,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityClientMovementSimpleParticleState {
    pub entity_id: i32,
    pub kind: EntityClientMovementSimpleParticleKind,
    pub position: EntityVec3,
    pub velocity: EntityVec3,
}

/// One `PowderSnowBlock.entityInside` snowflake chance. The consumer owns the
/// level random, so Vanilla's `nextBoolean` gate and velocity rolls happen at
/// spawn time; only the anchored position is decided here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PowderSnowFlakeParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

/// One Blaze `aiStep` particle submission. Keeping both smoke positions in one
/// typed owner preserves the Vanilla per-entity ordering while avoiding two
/// maximum-sized FIFO entries and duplicate renderer template lookup.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlazeAmbientParticleState {
    pub entity_id: i32,
    pub positions: [EntityVec3; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnderAmbientParticleState {
    pub entity_id: i32,
    pub positions: [EntityVec3; 2],
    pub velocities: [EntityVec3; 2],
}

/// One moving, submerged Guardian `aiStep` bubble pair. Keeping the fixed pair
/// under one FIFO owner preserves per-entity ordering and performs one particle
/// template lookup in the native resolver.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GuardianAmbientParticleState {
    pub entity_id: i32,
    pub positions: [EntityVec3; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardianAttackParticleState {
    pub entity_id: i32,
    pub positions: Vec<EntityVec3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlimeLandingParticleKind {
    ItemSlime,
    Flame,
}

/// The complete ordered particle submission from one `Slime.tick` landing.
/// Vanilla rolls every radial position before evaluating the squish sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlimeLandingParticleState {
    pub entity_id: i32,
    pub kind: SlimeLandingParticleKind,
    pub positions: Vec<EntityVec3>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementParticleState {
    BlazeAmbient(BlazeAmbientParticleState),
    EnderAmbient(EnderAmbientParticleState),
    GuardianAmbient(GuardianAmbientParticleState),
    GuardianAttack(GuardianAttackParticleState),
    FireworkRocket(FireworkRocketTrailParticleState),
    EyeOfEnder(EyeOfEnderTrailParticleState),
    ShulkerBullet(ShulkerBulletTrailParticleState),
    SlimeLanding(SlimeLandingParticleState),
    Simple(EntityClientMovementSimpleParticleState),
    PowderSnowFlake(PowderSnowFlakeParticleState),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementSoundKind {
    BlazeBurn {
        volume: f32,
        pitch: f32,
    },
    WardenHeartbeat {
        pitch: f32,
    },
    EnderDragonFlap {
        pitch: f32,
    },
    EnderDragonGrowl {
        pitch: f32,
    },
    LightningBoltThunder {
        pitch: f32,
    },
    LightningBoltImpact {
        pitch: f32,
    },
    GuardianFlop {
        elder: bool,
    },
    PlayerBlockStep {
        block_state_id: i32,
        muffled: bool,
    },
    PlayerSwim {
        volume: f32,
        pitch: f32,
    },
    PlayerSplash {
        high_speed: bool,
        volume: f32,
        pitch: f32,
    },
    PlayerCrystalChime {
        volume: f32,
        pitch: f32,
    },
    PlayerHoneyBlockSlide,
    PlayerPowderSnowFall {
        big: bool,
    },
    PlayerFall {
        big: bool,
    },
    PlayerBlockFall {
        block_state_id: i32,
    },
    EndermanStare,
    BreezeWhirl {
        volume: f32,
        pitch: f32,
    },
    BreezeIdle {
        on_ground: bool,
    },
    PhantomFlap {
        volume: f32,
        pitch: f32,
    },
    SnifferSearching,
    SnifferDigHit {
        block_state_id: i32,
    },
    CopperGolemSpin {
        weather_state: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityClientMovementSoundState {
    pub entity_id: i32,
    pub kind: EntityClientMovementSoundKind,
    pub position: EntityVec3,
}

/// Client-owned particle and sound effects emitted by entity movement in exact
/// vanilla tick order. The queue is drained into the lossless simulation effect
/// batch before the corresponding immutable publication becomes visible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityClientMovementEffectState {
    Particle(EntityClientMovementParticleState),
    Sound(EntityClientMovementSoundState),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LlamaSpitRecreationParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OminousItemSpawnerItemState<'a> {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub age_ticks: f32,
    pub stack: &'a ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OminousItemSpawnerParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PrimedTntSmokeParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

/// The wall the front of an item frame faces (vanilla `ItemFrame.getDirection`). Drives the frame's
/// render orientation: horizontal facings rotate the frame about Y, vertical facings tilt it about X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemFrameFacing {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

/// Everything needed to render one item-frame entity (vanilla `ItemFrameRenderState`): the resolved
/// wall-mounted center, the facing wall, the sampled entity-renderer light, the `0..=7` item rotation,
/// whether it is a glow frame, whether the entity is invisible, the framed item (`None` for an empty
/// frame), and the framed map id (which vanilla renders as a full-frame map only when map data exists).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameRenderState<'a> {
    pub entity_id: i32,
    pub center: EntityVec3,
    pub facing: ItemFrameFacing,
    pub light: TerrainLight,
    pub rotation: u8,
    pub glow: bool,
    pub invisible: bool,
    pub item: Option<&'a ProtocolItemStackSummary>,
    pub map_id: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityInterpolationState {
    pub remaining_steps: u8,
    pub target_position: EntityVec3,
    pub target_y_rot: f32,
    pub target_x_rot: f32,
    pub previous_position: EntityVec3,
    pub previous_y_rot: f32,
    pub previous_x_rot: f32,
}

/// Stable identity projection for a client-owned entity reference. ECS
/// incarnations are deliberately store-local, so serialized references retain
/// both the protocol id and UUID and are rebound after restoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRuntimeReferenceState {
    pub entity_id: i32,
    pub uuid: Uuid,
}

/// Client-owned `AbstractArrow` runtime that affects later collision results
/// but is not included in Vanilla's entity save tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbstractArrowRuntimeState {
    pub random_seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_deflected_by: Option<EntityRuntimeReferenceState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub piercing_ignore_entity_ids: Vec<i32>,
}

/// Serializable client-owned entries from Vanilla `LivingEntity.brain`.
///
/// Client-side brains start empty. The two retained memories are the only
/// codec-backed values written by client-reachable 26.1 behavior: Allay
/// interaction owns `LIKED_PLAYER`, and Villager `stopSleeping` owns
/// `LAST_WOKEN`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LivingEntityBrainState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liked_player: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_woken: Option<i64>,
}

/// Client-owned numeric furnace-minecart state.
///
/// `MinecartFurnace.interact` mutates both values on the client before the
/// server confirms the interaction. Its client tick does not decrement fuel or
/// clear push; the synchronized fuel accessor is only the lit/smoke boolean.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MinecartFurnaceState {
    pub fuel: i32,
    pub push: EntityVec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub data: i32,
    pub position: EntityVec3,
    pub position_base: EntityVec3,
    pub delta_movement: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub on_ground: Option<bool>,
    /// Active vanilla `InterpolationHandler` target and previous-tick state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpolation: Option<EntityInterpolationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_supporting_block_pos: Option<BlockPos>,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub on_ground_no_blocks: bool,
    pub data_values: Vec<ProtocolEntityDataValue>,
    /// Client fields projected from the painting variant holder through the active registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub painting_variant_projection: Option<PaintingVariantClientProjection>,
    pub equipment: Vec<ProtocolEquipmentSlotUpdate>,
    pub attributes: Vec<ProtocolAttributeSnapshot>,
    pub vehicle_id: Option<i32>,
    pub passengers: Vec<i32>,
    pub leash_holder_id: Option<i32>,
    pub last_animation_action: Option<u8>,
    pub last_event_id: Option<i8>,
    pub last_hurt_yaw: Option<f32>,
    #[serde(default)]
    pub mob_effects: BTreeMap<i32, MobEffectState>,
    #[serde(default)]
    pub client_animations: EntityClientAnimationState,
    #[serde(default)]
    pub last_damage: Option<EntityDamageEventState>,
    /// Vanilla client-side `LivingEntity.invulnerableTime`, reset by
    /// DamageEvent and decremented by scheduled living client ticks.
    #[serde(default)]
    pub damage_invulnerable_time: i32,
    /// Client-side `LivingEntity.dead`. Unlike synchronized health, this becomes
    /// true only after the guarded base `die` transition has run.
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub living_dead: bool,
    /// `Some` for every living entity, including an empty Vanilla brain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub living_brain: Option<LivingEntityBrainState>,
    /// Client-owned `Player.sleepCounter`. `None` identifies every non-player
    /// entity; player incarnations always project `Some(0..=109)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_sleep_counter: Option<i32>,
    #[serde(default)]
    pub minecart_lerp_pending_steps: Vec<ProtocolMinecartStep>,
    #[serde(default)]
    pub minecart_lerp_steps: Vec<ProtocolMinecartStep>,
    #[serde(default)]
    pub minecart_lerp_old_step: Option<ProtocolMinecartStep>,
    #[serde(default)]
    pub minecart_lerp_delay: i32,
    /// Client-local `BaseCommandBlock.trackOutput` for command-block
    /// minecarts. `None` identifies every other entity family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecart_command_block_track_output: Option<bool>,
    /// Client-owned numeric fuel/push for a furnace minecart. The protocol's
    /// fuel metadata is a separate visual boolean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecart_furnace: Option<MinecartFurnaceState>,
    #[serde(default)]
    pub hurting_projectile: Option<HurtingProjectileState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_lifecycle: Option<ProjectileLifecycleState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abstract_arrow_runtime: Option<AbstractArrowRuntimeState>,
}

pub(super) fn bool_is_false(value: &bool) -> bool {
    !*value
}

pub(super) fn u32_is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HurtingProjectileState {
    pub acceleration_power: f64,
}

/// Client-owned state from vanilla `Projectile` and its saveable subclasses.
/// Besides the spawn-resolved parent fields, this retains the private trident
/// tick state needed to reproduce `ThrownTrident.addAdditionalSaveData`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectileLifecycleState {
    pub owner_uuid: Option<Uuid>,
    pub left_owner: bool,
    pub has_been_shot: bool,
    #[serde(default, skip_serializing_if = "u32_is_zero")]
    pub trident_in_ground_time: u32,
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub trident_dealt_damage: bool,
    #[serde(default, skip_serializing_if = "u32_is_zero")]
    pub trident_return_ticks: u32,
    /// Spawn-time `AbstractArrow.pickup == ALLOWED`. The client upgrades the
    /// constructor default only when the add-entity owner resolves to Player;
    /// later owner unloading does not change the private enum.
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub arrow_pickup_allowed: bool,
    /// Client-side `Entity.noPhysics` field mirror: only `ThrownTrident.tick`
    /// sets it via `setNoPhysics(true)`, and no client path ever clears it.
    /// `isAffectedByBlocks` reads this field, not the synced arrow flag.
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub arrow_no_physics: bool,
    /// Last block state captured by client-side `AbstractArrow.onHitBlock`.
    /// The global id resolves through the current session block-state registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrow_last_block_state_id: Option<i32>,
    /// Whether a trident's mutable ground-hit sound has changed from
    /// `TRIDENT_HIT_GROUND` to `ARROW_HIT`. Ordinary arrows already use
    /// `ARROW_HIT`, so this bit is meaningful only for ThrownTrident.
    #[serde(default, skip_serializing_if = "bool_is_false")]
    pub arrow_hit_ground_sound_is_arrow: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePowerUpdateState {
    pub entity_id: i32,
    pub acceleration_power: f64,
    pub applied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleMoveReport {
    pub vehicle_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub snapped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityTransformState {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub data: i32,
    pub position: EntityVec3,
    pub position_base: EntityVec3,
    pub delta_movement: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub on_ground: Option<bool>,
}

/// Client-owned LightningBolt render source. Vanilla constructs `seed` from
/// the same entity RNG that later supplies first-tick thunder/impact pitch.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LightningBoltRenderSourceState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub seed: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RemovedPlayerVehicleTeleportRecovery {
    pub removed_vehicle_id: i32,
    pub player_pose: crate::LocalPlayerPoseState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeItemEntityPickupParticleState {
    pub item_entity_id: i32,
    pub item_entity_type_id: i32,
    pub item_position: EntityVec3,
    pub item_delta_movement: EntityVec3,
    /// The picked entity's yaw at extraction (vanilla `ItemPickupParticle` runs
    /// `extractEntity(entity, 1.0F)`; `ArrowRenderer`/`ThrownTridentRenderer`
    /// orient the projectile with `state.yRot`/`state.xRot`).
    #[serde(default)]
    pub item_y_rot: f32,
    #[serde(default)]
    pub item_x_rot: f32,
    #[serde(default)]
    pub item_age_ticks: f32,
    #[serde(default)]
    pub item_bob_offset: f32,
    #[serde(default = "entity_model_source_full_bright_light")]
    pub item_light: TerrainLight,
    pub target_entity_id: i32,
    pub target_position: EntityVec3,
    pub target_eye_height: f32,
    #[serde(default)]
    pub item_stack: Option<ProtocolItemStackSummary>,
    #[serde(default)]
    pub experience_orb_icon: Option<i32>,
    #[serde(default)]
    pub projectile_model: Option<TakeItemEntityPickupProjectileModel>,
}

/// The carried 3D model when the picked-up entity is an `AbstractArrow` /
/// thrown trident. Vanilla `ClientboundTakeItemEntity` spawns an
/// `ItemPickupParticle` for any entity kind and renders the extracted
/// `EntityRenderState` through `EntityRenderDispatcher.submit`
/// (`ItemPickupParticleGroup.State.submit`); the three picked-up families are
/// item entities (the `item_stack` field), experience orbs
/// (`experience_orb_icon`), and these projectiles. The shape mirrors bbb's
/// entity-scene kind projection (`EntityModelKind::Arrow { texture }` /
/// `EntityModelKind::Trident`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TakeItemEntityPickupProjectileModel {
    Arrow {
        /// Vanilla `TippableArrowRenderer.isTipped = getColor() > 0`
        /// (`Arrow.ID_EFFECT_COLOR`): swaps `arrow.png` for `arrow_tipped.png`.
        tipped: bool,
    },
    SpectralArrow,
    Trident {
        /// Vanilla `ThrownTrident.isFoil()` (`ID_FOIL`): the enchant glint pass.
        foil: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RavagerRoarParticleState {
    pub entity_id: i32,
    pub center: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RavagerStunParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EvokerFangsCritParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub velocity: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WitchMagicParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub bounding_box_max_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityPoofParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

/// Pose and stack used to spawn living entity item particles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityItemParticleState {
    pub count: usize,
    pub entity_id: i32,
    pub position: EntityVec3,
    pub eye_height: f32,
    pub y_rot: f32,
    pub x_rot: f32,
    pub item_stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityDrownParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LivingEntityPortalParticleState {
    pub entity_id: i32,
    pub previous_position: EntityVec3,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ArrowEffectParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
    pub color_rgb: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimalLoveParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AllayDuplicationParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityTamingParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VillagerParticleKind {
    Heart,
    Angry,
    Happy,
    Splash,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VillagerParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
    pub kind: VillagerParticleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DolphinHappyParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoxEatParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub item_stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnowballHitParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    /// `None` matches vanilla's empty-stack `ParticleTypes.ITEM_SNOWBALL` branch;
    /// missing metadata uses `Items.SNOWBALL` as the default projectile item.
    pub item_stack: Option<ProtocolItemStackSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrownEggHitParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    /// Missing metadata uses `Items.EGG`; explicit empty stacks do not emit particles.
    pub item_stack: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HoneyBlockParticleState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub count: u32,
    pub block_state_id: i32,
}

/// One `Rabbit.spawnSprintParticle` block-particle submission for entity event 1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RabbitSprintParticleState {
    pub entity_id: i32,
    pub block_state_id: i32,
    pub position: EntityVec3,
    pub velocity: EntityVec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityStatusProbeState {
    pub id: i32,
    pub entity_type_id: i32,
    pub last_animation_action: Option<u8>,
    pub last_event_id: Option<i8>,
    pub last_hurt_yaw: Option<f32>,
    pub mob_effects: BTreeMap<i32, MobEffectState>,
    pub last_damage: Option<EntityDamageEventState>,
    #[serde(default)]
    pub damage_invulnerable_time: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityCameraPoseState {
    pub id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub eye_height: f32,
}

/// The living-camera inputs consumed by vanilla `GameRenderer.bobHurt`.
///
/// This is client-observable presentation state only: packet-driven hurt yaw,
/// client-ticked hurt/death counters, and the generated LivingEntity capability.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityCameraHurtState {
    pub hurt_time: f32,
    pub hurt_duration: f32,
    pub hurt_dir: f32,
    pub death_time: f32,
    pub is_dead_or_dying: bool,
}

/// The player-camera inputs consumed by vanilla `GameRenderer.bobView`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityCameraViewBobState {
    pub backwards_interpolated_walk_distance: f32,
    pub bob: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPickTargetState {
    pub entity_id: i32,
    pub position: EntityVec3,
    pub bounds: EntityPickBoundsState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NearbyEntityItemProbe {
    pub item_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_name: Option<String>,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NearbyEntityProbe {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    pub position: EntityVec3,
    pub delta_movement: EntityVec3,
    pub distance: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub eye_height: f32,
    pub on_ground: Option<bool>,
    pub pickable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<NearbyEntityItemProbe>,
}

/// Copy-only inputs for the three renderer-specific `shouldRender` fallbacks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum EntityRendererSpecialCullingState {
    #[default]
    None,
    Guardian {
        attack_target_id: i32,
        eye_height: f32,
    },
    EndCrystal {
        has_beam_target: bool,
    },
    Shulker {
        render_offset_at_zero: Option<EntityVec3>,
        half_width: f32,
        half_height: f32,
    },
}

/// Narrow immutable row used to decide renderer visibility before projecting
/// the heap-heavy [`EntityRenderSource`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityRenderCandidateState {
    pub entity_id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub position: EntityVec3,
    /// Current, non-interpolated render position used by
    /// `EntityRenderDispatcher.distanceToSqr` during shadow extraction. For a
    /// passenger this is projected from the current root pose without mutating
    /// its packet-owned transform.
    pub current_position: EntityVec3,
    pub bounds: EntityPickBoundsState,
    pub vehicle_id: Option<i32>,
    pub leash_holder_id: Option<i32>,
    pub fishing_hook_owner_id: Option<i32>,
    pub is_spectator: bool,
    pub is_invisible: bool,
    pub is_sleeping: bool,
    pub wears_dragon_head: bool,
    pub has_glowing_flag: bool,
    pub has_always_visible_name: bool,
    pub display_culling: Option<DisplayEntityCullingState>,
    pub special_culling: EntityRendererSpecialCullingState,
}

/// Per-frame world state consumed by vanilla `FishingHookRenderer.extractRenderState`.
///
/// The hook and owner transforms are sampled at one partial tick so native can derive
/// the renderer's hand-to-hook offset without mixing generations. Item handedness,
/// crouching, scale, eye height, and attack animation all remain owned by the canonical
/// entity store.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FishingHookRenderSourceState {
    pub hook_position: EntityVec3,
    pub owner_id: i32,
    pub owner_eye_position: EntityVec3,
    pub owner_body_y_rot: f32,
    pub owner_scale: f32,
    pub owner_is_crouching: bool,
    pub holding_arm_left: bool,
    pub owner_attack_anim: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MinecartRailShape {
    NorthSouth,
    EastWest,
    AscendingEast,
    AscendingWest,
    AscendingNorth,
    AscendingSouth,
    SouthEast,
    SouthWest,
    NorthWest,
    NorthEast,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct MinecartRailRenderState {
    pub(super) pos_on_rail: [f32; 3],
    pub(super) front_pos: [f32; 3],
    pub(super) back_pos: [f32; 3],
    pub(super) rail_offset: [f32; 3],
    pub(super) rail_direction: [f32; 3],
}

impl MinecartRailShape {
    pub(super) fn from_vanilla_name(name: &str) -> Option<Self> {
        match name {
            "north_south" => Some(Self::NorthSouth),
            "east_west" => Some(Self::EastWest),
            "ascending_east" => Some(Self::AscendingEast),
            "ascending_west" => Some(Self::AscendingWest),
            "ascending_north" => Some(Self::AscendingNorth),
            "ascending_south" => Some(Self::AscendingSouth),
            "south_east" => Some(Self::SouthEast),
            "south_west" => Some(Self::SouthWest),
            "north_west" => Some(Self::NorthWest),
            "north_east" => Some(Self::NorthEast),
            _ => None,
        }
    }

    pub(super) fn is_slope(self) -> bool {
        matches!(
            self,
            Self::AscendingEast | Self::AscendingWest | Self::AscendingNorth | Self::AscendingSouth
        )
    }

    pub(super) fn exits(self) -> ([i32; 3], [i32; 3]) {
        match self {
            Self::NorthSouth => ([0, 0, -1], [0, 0, 1]),
            Self::EastWest => ([-1, 0, 0], [1, 0, 0]),
            Self::AscendingEast => ([-1, -1, 0], [1, 0, 0]),
            Self::AscendingWest => ([-1, 0, 0], [1, -1, 0]),
            Self::AscendingNorth => ([0, 0, -1], [0, -1, 1]),
            Self::AscendingSouth => ([0, -1, -1], [0, 0, 1]),
            Self::SouthEast => ([0, 0, 1], [1, 0, 0]),
            Self::SouthWest => ([0, 0, 1], [-1, 0, 0]),
            Self::NorthWest => ([0, 0, -1], [-1, 0, 0]),
            Self::NorthEast => ([0, 0, -1], [1, 0, 0]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallingBlockModelState {
    pub block_state_id: i32,
    pub block: &'static BlockStateInfo,
    /// Vanilla `MovingBlockRenderState.randomSeedPos`: the synchronized
    /// `FallingBlockEntity.DATA_START_POS`, initialized from the add-entity
    /// position by `recreateFromPacket`.
    pub random_seed_pos: BlockPos,
    /// Vanilla `MovingBlockRenderState.blockPos`: `BlockPos.containing(x,
    /// boundingBox.maxY, z)` at extraction time. Biome tint and moving-block
    /// light lookup use this position, independently of `random_seed_pos`.
    pub block_pos: BlockPos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinecartDisplayBlockState {
    pub block: &'static BlockStateInfo,
    pub display_offset: i32,
}
