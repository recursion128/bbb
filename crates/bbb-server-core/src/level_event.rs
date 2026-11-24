//! Level broadcasts, game events and requests that need the owning world.
//!
//! Mechanisms hold a level slice; `crate::world` consumes requests and fans
//! broadcasts out to sessions.
//!
//! # `levelEvent`
//!
//! `LevelAccessor.levelEvent(int, BlockPos, int)` (`LevelAccessor.java:84-86`)
//! is the three-argument overload every body in this crate reaches; it is the
//! four-argument `levelEvent(source, type, pos, data)`
//! (`LevelAccessor.java:82`) with a null source. `ServerLevel` answers it with
//! one `PlayerList.broadcast` (`ServerLevel.java:1095-1107`):
//! `broadcast(source instanceof Player ? player : null, x, y, z, 64.0,
//! dimension, new ClientboundLevelEventPacket(type, pos, data, false))`. The
//! per-recipient test is `PlayerList.broadcast`'s own
//! (`PlayerList.java:597-617`): not the excluded player, same dimension, and
//! the *squared* distance from the raw block coordinates to the recipient's
//! feet strictly below the squared range.
//!
//! The `globalEvent` flag on the packet is always false on this path.
//! `ServerLevel.globalLevelEvent` (`ServerLevel.java:1071-1092`) is the only
//! producer of a true one; it queues as
//! [`PendingLevelBroadcast::GlobalLevelEvent`] because its fan-out reads the
//! game rule and every dimension's players, and it falls back to this
//! broadcast when `minecraft:global_sound_events` is off (`:1089`).
//!
//! # `gameEvent`
//!
//! `LevelAccessor.gameEvent(Holder<GameEvent>, BlockPos, Context)`
//! (`LevelAccessor.java:98-100`) recentres the position with
//! `Vec3.atCenterOf(pos)` and hands it to `ServerLevel.gameEvent`
//! (`ServerLevel.java:1114-1116`), which is one line: `gameEventDispatcher.post`.
//!
//! Block entities and entities register listeners in section data. ServerWorld
//! resolves their live positions and delivers the events queued in
//! [`LevelEventSink::pending_game_events`].
//!
//! The same seam keeps a *record* that the event happened, so that a body which
//! stops raising one is visible: a bounded log of the last
//! [`MAX_RECORDED_GAME_EVENTS`] and an unbounded count, which
//! `crate::world::ServerWorld::game_events` reads. The bound is on
//! the record only -- nothing is dropped from the queue because of it.
//!
//! # The ids are named, not numbered at the call site
//!
//! `LevelEvent` (`net/minecraft/world/level/block/LevelEvent.java`) is a wall
//! of `public static final int`s; the ones this crate raises are named here so
//! that no body carries bare integer literals.

use std::collections::VecDeque;
use std::sync::Arc;

use bbb_protocol::codec::Encoder;
use bbb_protocol::debug_subscription_types::VANILLA_DEBUG_GAME_EVENT_TYPES;
use bbb_protocol::packets::{BlockPos, Vec3d};
use uuid::Uuid;

/// `LevelEvent.SOUND_STOP_JUKEBOX_SONG` (`LevelEvent.java:10`), raised with a
/// literal zero (`JukeboxSongPlayer.java:58`, `JukeboxBlockEntity.java:128`).
pub const LEVEL_EVENT_SOUND_STOP_JUKEBOX_SONG: i32 = 1011;

/// `LevelEvent.LAVA_FIZZ` (`LevelEvent.java:47`) -- the id both `LavaFluid.fizz`
/// (`LavaFluid.java:194-196`) and `LiquidBlock.fizz`
/// (`LiquidBlock.java:240-242`) pass to `levelEvent`.
pub const LEVEL_EVENT_LAVA_FIZZ: i32 = 1501;

/// `LevelEvent.REDSTONE_TORCH_BURNOUT` (`LevelEvent.java:48`) -- the smoke a
/// torch raises on the eighth forced toggle inside its window
/// (`RedstoneTorchBlock.java:85`). Its `data` is a literal zero there.
pub const LEVEL_EVENT_REDSTONE_TORCH_BURNOUT: i32 = 1502;

/// `LevelEvent.PARTICLES_DESTROY_BLOCK` (`LevelEvent.java:53`) -- the break
/// particles and sound `Level.destroyBlock` (`Level.java:283`) and
/// `Block.spawnDestroyParticles` (`Block.java:514-516`) both raise, whose
/// `data` is `Block.getId(state)` (`Block.java:131-138`), the global
/// block-state id.
pub const LEVEL_EVENT_PARTICLES_DESTROY_BLOCK: i32 = 2001;

/// The radius `ServerLevel.levelEvent` hands `PlayerList.broadcast`
/// (`ServerLevel.java:1103`).
///
/// It is a plain `64.0` at that one call site rather than a named constant, and
/// it is compared squared against the squared distance with a strict `<`
/// (`PlayerList.java:612-614`), so a listener exactly 64 blocks away is *not*
/// sent the packet.
pub const LEVEL_EVENT_BROADCAST_RANGE: f64 = 64.0;

/// `ServerLevel.sendParticles` (`ServerLevel.java:1362-1363`) when
/// `overrideLimiter` is false.
pub const LEVEL_PARTICLE_BROADCAST_RANGE: f64 = 32.0;

/// One `LevelAccessor.levelEvent(type, pos, data)` a mechanism raised, waiting
/// for the fan-out.
///
/// There is no `source` field because the three-argument overload is the only
/// one any mechanism in this crate reaches and it passes null
/// (`LevelAccessor.java:85`). The one caller that does exclude a player --
/// `Block.spawnDestroyParticles` (`Block.java:514-516`) -- runs on the session
/// path, where the fan-out is reachable directly and nothing has to be queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingLevelEvent {
    /// The `LevelEvent` id, e.g. [`LEVEL_EVENT_PARTICLES_DESTROY_BLOCK`].
    pub event_type: i32,
    pub pos: BlockPos,
    /// The event's own payload; its meaning belongs to the event.
    pub data: i32,
}

/// Level broadcasts and world requests sharing the level drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingLevelBroadcast {
    BeehiveEmergencyRelease {
        pos: BlockPos,
        state: i32,
    },
    EntityAnimation {
        entity_id: i32,
        hand: bbb_protocol::packets::InteractionHand,
    },
    LevelEvent(PendingLevelEvent),
    /// `ServerLevel.globalLevelEvent` (`ServerLevel.java:1071-1092`).
    GlobalLevelEvent(PendingLevelEvent),
    BlockDestruction {
        entity_id: i32,
        pos: BlockPos,
        stage: i32,
    },
}

/// One `Level.playSound(null, pos, sound, SoundSource.BLOCKS)` a mechanism
/// queued for the same drain as [`PendingLevelEvent`].
#[derive(Debug, Clone, PartialEq)]
pub struct PendingBlockSound {
    pub pos: BlockPos,
    pub resource_id: String,
    pub volume: f32,
    pub pitch: f32,
    /// The `except` entity of `Level.playSound(except, ...)`; only a player
    /// id excludes a recipient (`ServerLevel.java:1034-1045`).
    pub except_entity_id: Option<i32>,
}

/// One `ServerLevel.sendParticles` trail particle queued for the level drain.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingTrailParticle {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub target_z: f64,
    pub color: i32,
    pub duration: i32,
}

/// `ParticleTypes.TRAIL` options (`world_effects.rs`, type id `49`).
#[must_use]
pub fn encode_trail_particle_options(
    target_x: f64,
    target_y: f64,
    target_z: f64,
    color: i32,
    duration: i32,
) -> Vec<u8> {
    let mut encoder = Encoder::default();
    encoder.write_f64(target_x);
    encoder.write_f64(target_y);
    encoder.write_f64(target_z);
    encoder.write_i32(color);
    encoder.write_var_i32(duration);
    encoder.into_inner()
}

const fn const_str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

const fn game_event_index_by_constant(constant_name: &str) -> u8 {
    let mut index = 0;
    while index < VANILLA_DEBUG_GAME_EVENT_TYPES.len() {
        if const_str_eq(
            VANILLA_DEBUG_GAME_EVENT_TYPES[index].constant_name,
            constant_name,
        ) {
            return index as u8;
        }
        index += 1;
    }
    panic!("unknown minecraft:game_event constant")
}

const fn game_event_index_by_resource(resource_id: &str) -> u8 {
    let mut index = 0;
    while index < VANILLA_DEBUG_GAME_EVENT_TYPES.len() {
        if const_str_eq(
            VANILLA_DEBUG_GAME_EVENT_TYPES[index].resource_id,
            resource_id,
        ) {
            return index as u8;
        }
        index += 1;
    }
    panic!("unknown minecraft:game_event resource id")
}

/// One row of the `minecraft:game_event` registry, resolved at compile time to
/// its registration index in `VANILLA_DEBUG_GAME_EVENT_TYPES`
/// (`net/minecraft/world/level/gameevent/GameEvent.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VanillaGameEvent {
    index: u8,
}

const RESONATE_GAME_EVENTS: [VanillaGameEvent; 15] = [
    VanillaGameEvent::by_resource("minecraft:resonate_1"),
    VanillaGameEvent::by_resource("minecraft:resonate_2"),
    VanillaGameEvent::by_resource("minecraft:resonate_3"),
    VanillaGameEvent::by_resource("minecraft:resonate_4"),
    VanillaGameEvent::by_resource("minecraft:resonate_5"),
    VanillaGameEvent::by_resource("minecraft:resonate_6"),
    VanillaGameEvent::by_resource("minecraft:resonate_7"),
    VanillaGameEvent::by_resource("minecraft:resonate_8"),
    VanillaGameEvent::by_resource("minecraft:resonate_9"),
    VanillaGameEvent::by_resource("minecraft:resonate_10"),
    VanillaGameEvent::by_resource("minecraft:resonate_11"),
    VanillaGameEvent::by_resource("minecraft:resonate_12"),
    VanillaGameEvent::by_resource("minecraft:resonate_13"),
    VanillaGameEvent::by_resource("minecraft:resonate_14"),
    VanillaGameEvent::by_resource("minecraft:resonate_15"),
];

impl VanillaGameEvent {
    const fn by_constant(constant_name: &str) -> Self {
        Self {
            index: game_event_index_by_constant(constant_name),
        }
    }

    pub(crate) const fn by_resource(resource_id: &str) -> Self {
        Self {
            index: game_event_index_by_resource(resource_id),
        }
    }

    pub(crate) const fn resonate(frequency: i32) -> Option<Self> {
        if frequency >= 1 && frequency <= 15 {
            Some(RESONATE_GAME_EVENTS[(frequency - 1) as usize])
        } else {
            None
        }
    }

    pub(crate) fn from_resource_id(id: &str) -> Option<Self> {
        VANILLA_DEBUG_GAME_EVENT_TYPES
            .iter()
            .position(|entry| entry.resource_id == id)
            .map(|index| Self { index: index as u8 })
    }

    /// GameEvent.java:56.
    pub const SPLASH: Self = Self::by_constant("SPLASH");

    /// GameEvent.java:42.
    pub const HIT_GROUND: Self = Self::by_constant("HIT_GROUND");

    /// `GameEvent.BLOCK_ACTIVATE` (`GameEvent.java:16`).
    pub const BLOCK_ACTIVATE: Self = Self::by_constant("BLOCK_ACTIVATE");

    /// `GameEvent.BLOCK_ATTACH` (`GameEvent.java:17`).
    pub const BLOCK_ATTACH: Self = Self::by_constant("BLOCK_ATTACH");

    /// `GameEvent.BLOCK_CHANGE` (`GameEvent.java:18`), which
    /// `FarmlandBlock.turnToDirt` raises (`FarmlandBlock.java:124`).
    pub const BLOCK_CHANGE: Self = Self::by_constant("BLOCK_CHANGE");

    /// `GameEvent.BLOCK_CLOSE` (`GameEvent.java:19`).
    pub const BLOCK_CLOSE: Self = Self::by_constant("BLOCK_CLOSE");

    /// `GameEvent.BLOCK_DESTROY` (`GameEvent.java:21`), which
    /// `Level.destroyBlock` (`Level.java:293`) and `Block.playerWillDestroy`
    /// (`Block.java:524`) both raise.
    pub const BLOCK_DESTROY: Self = Self::by_constant("BLOCK_DESTROY");

    /// `GameEvent.BLOCK_DEACTIVATE` (`GameEvent.java:20`).
    pub const BLOCK_DEACTIVATE: Self = Self::by_constant("BLOCK_DEACTIVATE");

    /// `GameEvent.BLOCK_DETACH` (`GameEvent.java:22`).
    pub const BLOCK_DETACH: Self = Self::by_constant("BLOCK_DETACH");

    /// `GameEvent.BLOCK_OPEN` (`GameEvent.java:23`).
    pub const BLOCK_OPEN: Self = Self::by_constant("BLOCK_OPEN");

    /// `GameEvent.BLOCK_PLACE` (`GameEvent.java:24`).
    pub const BLOCK_PLACE: Self = Self::by_constant("BLOCK_PLACE");

    /// `GameEvent.CONTAINER_CLOSE` (`GameEvent.java:25`).
    pub const CONTAINER_CLOSE: Self = Self::by_constant("CONTAINER_CLOSE");

    /// `GameEvent.CONTAINER_OPEN` (`GameEvent.java:26`).
    pub const CONTAINER_OPEN: Self = Self::by_constant("CONTAINER_OPEN");

    /// `GameEvent.DRINK` (`GameEvent.java:27`).
    pub const DRINK: Self = Self::by_constant("DRINK");

    /// `GameEvent.EAT` (`GameEvent.java:28`), which `CakeBlock.eat` raises
    /// before it writes `BITES` or removes the cake (`CakeBlock.java:102`).
    pub const EAT: Self = Self::by_constant("EAT");

    /// `GameEvent.ELYTRA_GLIDE` (`GameEvent.java:29`).
    pub const ELYTRA_GLIDE: Self = Self::by_constant("ELYTRA_GLIDE");

    /// `GameEvent.ENTITY_DAMAGE` (`GameEvent.java:30`), which
    /// `ItemEntity.hurtServer` raises after changing health and before running
    /// the destroyed-item hook (`ItemEntity.java:297-302`).
    pub const ENTITY_DAMAGE: Self = Self::by_constant("ENTITY_DAMAGE");

    /// `GameEvent.ENTITY_DIE` (`GameEvent.java:31`), which `LivingEntity.die`
    /// raises before it drops the death loot (`LivingEntity.java:1447`).
    pub const ENTITY_DIE: Self = Self::by_constant("ENTITY_DIE");

    /// `GameEvent.ENTITY_DISMOUNT` (`GameEvent.java:32`), which
    /// `Entity.removeVehicle` raises at the vehicle it just left
    /// (`Entity.java:2410`).
    pub const ENTITY_DISMOUNT: Self = Self::by_constant("ENTITY_DISMOUNT");

    /// `GameEvent.ENTITY_INTERACT` (`GameEvent.java:33`).
    pub const ENTITY_INTERACT: Self = Self::by_constant("ENTITY_INTERACT");

    /// `GameEvent.ENTITY_MOUNT` (`GameEvent.java:34`), which
    /// `Entity.startRiding` raises at the vehicle it just boarded
    /// (`Entity.java:2381`).
    pub const ENTITY_MOUNT: Self = Self::by_constant("ENTITY_MOUNT");

    /// `GameEvent.ENTITY_ACTION` (`GameEvent.java:36`).
    pub const ENTITY_ACTION: Self = Self::by_constant("ENTITY_ACTION");

    /// `GameEvent.ENTITY_PLACE` (`GameEvent.java:35`).
    pub const ENTITY_PLACE: Self = Self::by_constant("ENTITY_PLACE");

    /// `GameEvent.EXPLODE` (`GameEvent.java:38`), which `ServerExplosion.explode`
    /// raises before it samples any block (`ServerExplosion.java:236`).
    pub const EXPLODE: Self = Self::by_constant("EXPLODE");

    /// `GameEvent.JUKEBOX_PLAY` (`GameEvent.java:46`), the beat a playing
    /// `JukeboxSongPlayer.tick` raises (`JukeboxSongPlayer.java:68-70`).
    pub const JUKEBOX_PLAY: Self = Self::by_constant("JUKEBOX_PLAY");

    /// `GameEvent.JUKEBOX_STOP_PLAY` (`GameEvent.java:47`), raised by
    /// `JukeboxSongPlayer.stop` (`JukeboxSongPlayer.java:57`).
    pub const JUKEBOX_STOP_PLAY: Self = Self::by_constant("JUKEBOX_STOP_PLAY");

    /// `GameEvent.NOTE_BLOCK_PLAY` (`GameEvent.java:49`).
    pub const NOTE_BLOCK_PLAY: Self = Self::by_constant("NOTE_BLOCK_PLAY");

    /// `GameEvent.PRIME_FUSE` (`GameEvent.java:50`), which `TntBlock.prime`
    /// raises for the source that lit it (`TntBlock.java:94`).
    pub const PRIME_FUSE: Self = Self::by_constant("PRIME_FUSE");

    /// `GameEvent.LIGHTNING_STRIKE` (`GameEvent.java:48`), which
    /// `LightningBolt.tick` raises (`LightningBolt.java:110`).
    pub const LIGHTNING_STRIKE: Self = Self::by_constant("LIGHTNING_STRIKE");

    /// `GameEvent.ITEM_INTERACT_FINISH` (`GameEvent.java:44`).
    pub const ITEM_INTERACT_FINISH: Self = Self::by_constant("ITEM_INTERACT_FINISH");

    /// `GameEvent.ITEM_INTERACT_START` (`GameEvent.java:45`).
    pub const ITEM_INTERACT_START: Self = Self::by_constant("ITEM_INTERACT_START");

    /// `GameEvent.PROJECTILE_LAND` (`GameEvent.java:51`).
    pub const PROJECTILE_LAND: Self = Self::by_constant("PROJECTILE_LAND");

    /// `GameEvent.PROJECTILE_SHOOT` (`GameEvent.java:52`).
    pub const PROJECTILE_SHOOT: Self = Self::by_constant("PROJECTILE_SHOOT");

    /// `GameEvent.FLAP` (`GameEvent.java:39`).
    pub const FLAP: Self = Self::by_constant("FLAP");

    /// `GameEvent.FLUID_PICKUP` (`GameEvent.java:40`).
    pub const FLUID_PICKUP: Self = Self::by_constant("FLUID_PICKUP");

    /// `GameEvent.FLUID_PLACE` (`GameEvent.java:41`).
    pub const FLUID_PLACE: Self = Self::by_constant("FLUID_PLACE");

    /// `GameEvent.SHEAR` (`GameEvent.java:54`).
    pub const SHEAR: Self = Self::by_constant("SHEAR");

    /// `GameEvent.SCULK_SENSOR_TENDRILS_CLICKING` (`GameEvent.java:53`), which
    /// `SculkSensorBlock.activate` raises (`SculkSensorBlock.java:217`).
    pub const SCULK_SENSOR_TENDRILS_CLICKING: Self =
        Self::by_constant("SCULK_SENSOR_TENDRILS_CLICKING");

    /// `GameEvent.SHRIEK` (`GameEvent.java:55`), which
    /// `SculkShriekerBlockEntity.shriek` raises
    /// (`SculkShriekerBlockEntity.java:124`).
    pub const SHRIEK: Self = Self::by_constant("SHRIEK");

    /// `GameEvent.STEP` (`GameEvent.java:57`), which
    /// `Entity.vibrationAndSoundEffectsFromBlock` raises beside the walking
    /// step sound (`Entity.java:936-938`).
    pub const STEP: Self = Self::by_constant("STEP");

    /// `GameEvent.SWIM` (`GameEvent.java:58`), which
    /// `Entity.applyMovementEmissionAndPlaySound` raises on the water arm
    /// (`Entity.java:832-833`).
    pub const SWIM: Self = Self::by_constant("SWIM");

    /// `GameEvent.INSTRUMENT_PLAY` (`GameEvent.java:43`).
    pub const INSTRUMENT_PLAY: Self = Self::by_constant("INSTRUMENT_PLAY");

    /// `GameEvent.EQUIP` (`GameEvent.java:37`).
    pub const EQUIP: Self = Self::by_constant("EQUIP");

    /// `GameEvent.UNEQUIP` (`GameEvent.java:60`).
    pub const UNEQUIP: Self = Self::by_constant("UNEQUIP");

    /// `GameEvent.TELEPORT` (`GameEvent.java:59`).
    pub const TELEPORT: Self = Self::by_constant("TELEPORT");

    /// The `GameEvent` constant that declares this row.
    #[must_use]
    pub const fn constant_name(self) -> &'static str {
        self.catalog_row().constant_name
    }

    /// The registry id `Registry.registerForHolder` handed this row
    /// (`GameEvent.java:87-89`), i.e. its position in registration order.
    #[must_use]
    pub const fn holder_id(self) -> i32 {
        self.catalog_row().holder_id
    }

    /// `minecraft:<name>`, the `Identifier` the row was registered under
    /// (`GameEvent.java:88`).
    #[must_use]
    pub const fn resource_id(self) -> &'static str {
        self.catalog_row().resource_id
    }

    const fn catalog_row(
        self,
    ) -> &'static bbb_protocol::debug_subscription_types::DebugHolderTypeCatalogEntry {
        &VANILLA_DEBUG_GAME_EVENT_TYPES[self.index as usize]
    }
}

/// One `Level.gameEvent(event, pos, Context.of(..))` a mechanism raised.
///
/// `GameEvent.Context` (`GameEvent.java:91-103`) is a `sourceEntity` and an
/// `affectedState`. Block producers carry only the second; entity damage
/// carries the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedGameEvent {
    pub game_event: VanillaGameEvent,
    /// The block projection retained by existing diagnostic consumers.
    pub pos: BlockPos,
    /// `GameEvent.Context.sourceEntity().getId()`, when the producer supplied
    /// one. Block-owned producers continue to record `None`.
    pub source_entity_id: Option<i32>,
    /// `GameEvent.Context.affectedState` (`GameEvent.java:91`).
    pub affected_state: Option<i32>,
}

/// One event's diagnostic row and the exact position handed to listeners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionedGameEvent {
    pub recorded: RecordedGameEvent,
    pub position: Vec3d,
}

/// How many game events [`LevelEventSink`] keeps as its *record*.
///
/// Not a vanilla number: vanilla keeps none, because it delivers each one to
/// its listeners and forgets it. This is a bound on the log
/// [`LevelEventSink::recent_game_events`] answers, so that a body which stops
/// raising an event is visible without the log growing without limit. It bounds
/// nothing else: the events waiting for the dispatcher
/// ([`LevelEventSink::pending_game_events`]) are not dropped by it.
pub const MAX_RECORDED_GAME_EVENTS: usize = 256;

/// The source metadata a queued entity hurt needs after the entity tick.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingHurtEntity {
    Player {
        uuid: Uuid,
    },
    Stored {
        entity_id: i32,
    },
    FallingBlock {
        entity_id: i32,
        uuid: Uuid,
        position: Vec3d,
        display_name: Arc<bbb_protocol::NbtSummaryValue>,
    },
}

/// One `player.hurtServer(level, source, amount)` an entity tick asks for
/// (`EvokerFangs.dealDamageTo`, `EvokerFangs.java:108`, `:115`;
/// `Entity.thunderHit`, `Entity.java:2768`), queued because the sessions are
/// not reachable from a ticking level; the driver applies it after the entity
/// loop.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPlayerHurt {
    pub player: Uuid,
    /// `DamageSource.type()` as its registry id.
    pub damage_type: &'static str,
    pub amount: f32,
    /// `DamageSource.getEntity()`.
    pub causing: Option<PendingHurtEntity>,
    /// `DamageSource.getDirectEntity()`.
    pub direct: Option<PendingHurtEntity>,
    /// Whether an accepted hurt runs `EnchantmentHelper.doPostAttackEffects`
    /// (`Mob.java:1369`, `EvokerFangs.java:115-116`).
    pub post_attack: bool,
    /// What an accepted melee hit does to the player's motion past the
    /// hurt's own knockback.
    pub impulse: Option<PendingHitImpulse>,
}

/// `LivingEntity.stabAttack` (`LivingEntity.java:2825-2867`) by a mob on a
/// player: the hurt when the strike deals damage, then the knockback and the
/// dismount, which land whether or not the hurt was accepted.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPlayerStab {
    pub player: Uuid,
    pub hurt: Option<PendingPlayerHurt>,
    pub knockback: Option<PendingHitImpulse>,
    pub dismount: bool,
}

/// One queued hit on a player, in the order the entity loop asked for them.
#[derive(Debug, Clone, PartialEq)]
pub enum PendingPlayerHit {
    Hurt(PendingPlayerHurt),
    Stab(PendingPlayerStab),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PendingHitImpulse {
    /// `LivingEntity.causeExtraKnockback`'s `knockback(power, xd, zd)`
    /// (`LivingEntity.java:2679-2684`).
    Knockback {
        power: f64,
        xd: f64,
        zd: f64,
    },
    /// `IronGolem.doHurtTarget`'s upward toss (`IronGolem.java:195-197`).
    Lift {
        y: f64,
    },
    HoglinThrow {
        from: Vec3d,
        power: f64,
    },
}

/// `EnchantmentHelper.doPostAttackEffects` (`EnchantmentHelper.java:210-215`)
/// owed to a stored victim whose hurt an entity tick already applied; the
/// driver runs it after the entity loop with the player hurts.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPostAttack {
    pub victim_entity_id: i32,
    pub damage_type: &'static str,
    pub causing: Option<PendingHurtEntity>,
    pub direct: Option<PendingHurtEntity>,
}

/// One `Entity.propagateFallToPassengers` arm that lands on an online player
/// (`Entity.java:1497-1503`); the session's own `Player.causeFallDamage`
/// computes the damage from its attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPlayerFall {
    pub player: Uuid,
    pub distance: f64,
    pub modifier: f32,
    pub damage_type: &'static str,
}

/// One `ExperienceOrb.award` (`ExperienceOrb.java:187-199`) a death owes.
///
/// `LivingEntity.die` raises `GameEvent.ENTITY_DIE` before `dropAllDeathLoot`
/// (`LivingEntity.java:1447-1448`), and the sculk catalyst's listener consumes
/// the victim's experience from inside that raise
/// (`SculkCatalystBlockEntity.java:95`). The award is therefore queued at the
/// death and paid after the frame's game events, so the listener's
/// `skipDropExperience` is visible to [`LevelEventSink`]'s owner when it pays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingDeathExperience {
    pub entity_id: i32,
    pub position: Vec3d,
    /// `DamageSource.getEntity()` of the killing blow (`LivingEntity.java:1488`).
    pub killer_entity_id: Option<i32>,
}

/// The level-owned end of both channels.
///
/// A mechanism reaches it through the level seam it already holds; the driver
/// drains the level events into the session fan-out and reads the game-event
/// log.
#[derive(Debug, Clone, Default)]
pub struct LevelEventSink {
    pending: Vec<PendingLevelBroadcast>,
    pending_sounds: Vec<PendingBlockSound>,
    pending_trail_particles: Vec<PendingTrailParticle>,
    recent_game_events: VecDeque<PositionedGameEvent>,
    /// The events waiting for the dispatcher, oldest first.
    ///
    /// A raiser holds a level slice, and the registry the dispatcher walks is
    /// in the chunk store the slice's owner lends to the mechanism, so the
    /// world posts these when the body that raised them yields
    /// (`crate::world::ServerWorld::post_pending_game_events`).
    ///
    /// Unbounded on purpose: `MAX_RECORDED_GAME_EVENTS` bounds the record, and
    /// a delivery must not be lost to a record's ring.
    pending_game_events: Vec<PositionedGameEvent>,
    game_event_count: u64,
    pending_player_hurts: Vec<PendingPlayerHit>,
    pending_post_attacks: Vec<PendingPostAttack>,
    pending_player_falls: Vec<PendingPlayerFall>,
    /// Death experience waiting for the frame's game events, oldest first.
    pending_death_experience: Vec<PendingDeathExperience>,
}

impl LevelEventSink {
    pub(crate) fn beehive_emergency_release(&mut self, pos: BlockPos, state: i32) {
        self.pending
            .push(PendingLevelBroadcast::BeehiveEmergencyRelease { pos, state });
    }

    /// `LevelAccessor.levelEvent(type, pos, data)` (`LevelAccessor.java:84-86`).
    pub fn level_event(&mut self, event_type: i32, pos: BlockPos, data: i32) {
        self.pending
            .push(PendingLevelBroadcast::LevelEvent(PendingLevelEvent {
                event_type,
                pos,
                data,
            }));
    }

    /// `Level.globalLevelEvent` as `ServerLevel` overrides it (`ServerLevel.java:1071-1092`).
    pub fn global_level_event(&mut self, event_type: i32, pos: BlockPos, data: i32) {
        self.pending
            .push(PendingLevelBroadcast::GlobalLevelEvent(PendingLevelEvent {
                event_type,
                pos,
                data,
            }));
    }

    pub fn entity_animation(
        &mut self,
        entity_id: i32,
        hand: bbb_protocol::packets::InteractionHand,
    ) {
        self.pending
            .push(PendingLevelBroadcast::EntityAnimation { entity_id, hand });
    }

    // BreakDoorGoal.java:62-81: crack progress and level events retain their emission order.
    pub fn destroy_block_progress(&mut self, entity_id: i32, pos: BlockPos, stage: i32) {
        self.pending.push(PendingLevelBroadcast::BlockDestruction {
            entity_id,
            pos,
            stage,
        });
    }

    /// `Level.playSound(null, pos, sound, SoundSource.BLOCKS)`
    /// (`Level.java:389-393`).
    pub fn play_block_sound(&mut self, pos: BlockPos, resource_id: &str) {
        self.play_block_sound_with_volume_pitch(pos, resource_id, 1.0, 1.0);
    }

    pub fn play_block_sound_with_volume_pitch(
        &mut self,
        pos: BlockPos,
        resource_id: &str,
        volume: f32,
        pitch: f32,
    ) {
        self.play_block_sound_except(pos, resource_id, volume, pitch, None);
    }

    /// `Level.playSound(except, pos, sound, SoundSource.BLOCKS, volume, pitch)`.
    pub fn play_block_sound_except(
        &mut self,
        pos: BlockPos,
        resource_id: &str,
        volume: f32,
        pitch: f32,
        except_entity_id: Option<i32>,
    ) {
        self.pending_sounds.push(PendingBlockSound {
            pos,
            resource_id: resource_id.to_string(),
            volume,
            pitch,
            except_entity_id,
        });
    }

    pub fn send_trail_particle(&mut self, from: [f64; 3], to: [f64; 3], color: i32, duration: i32) {
        self.pending_trail_particles.push(PendingTrailParticle {
            x: from[0],
            y: from[1],
            z: from[2],
            target_x: to[0],
            target_y: to[1],
            target_z: to[2],
            color,
            duration,
        });
    }

    /// `LevelAccessor.gameEvent(event, pos, context)`
    /// (`LevelAccessor.java:98-100`), queued for the dispatcher and recorded.
    pub fn game_event(
        &mut self,
        game_event: VanillaGameEvent,
        pos: BlockPos,
        affected_state: Option<i32>,
    ) {
        self.game_event_with_source(game_event, pos, None, affected_state);
    }

    /// The full `GameEvent.Context` shape for entity-owned producers.
    pub fn game_event_with_source(
        &mut self,
        game_event: VanillaGameEvent,
        pos: BlockPos,
        source_entity_id: Option<i32>,
        affected_state: Option<i32>,
    ) {
        // `LevelAccessor.java:99`: only a BlockPos producer is centred.
        self.record_game_event(
            RecordedGameEvent {
                game_event,
                pos,
                source_entity_id,
                affected_state,
            },
            Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
        );
    }

    /// `Entity.gameEvent` forwards its exact `Vec3` (`Entity.java:1331-1332`).
    pub fn game_event_with_source_at(
        &mut self,
        game_event: VanillaGameEvent,
        position: Vec3d,
        source_entity_id: Option<i32>,
        affected_state: Option<i32>,
    ) {
        self.record_game_event(
            RecordedGameEvent {
                game_event,
                pos: BlockPos {
                    x: position.x.floor() as i32,
                    y: position.y.floor() as i32,
                    z: position.z.floor() as i32,
                },
                source_entity_id,
                affected_state,
            },
            position,
        );
    }

    fn record_game_event(&mut self, recorded: RecordedGameEvent, position: Vec3d) {
        let event = PositionedGameEvent { recorded, position };
        self.game_event_count += 1;
        if self.recent_game_events.len() == MAX_RECORDED_GAME_EVENTS {
            self.recent_game_events.pop_front();
        }
        self.recent_game_events.push_back(event);
        self.pending_game_events.push(event);
    }

    /// The game events raised since the last drain, in the order they were
    /// raised.
    pub fn drain_game_events(&mut self) -> Vec<RecordedGameEvent> {
        self.drain_positioned_game_events()
            .into_iter()
            .map(|event| event.recorded)
            .collect()
    }

    /// The world dispatcher consumes the exact positions from this queue.
    pub fn drain_positioned_game_events(&mut self) -> Vec<PositionedGameEvent> {
        std::mem::take(&mut self.pending_game_events)
    }

    /// Whether any game event is still waiting for the dispatcher.
    #[must_use]
    pub fn has_pending_game_events(&self) -> bool {
        !self.pending_game_events.is_empty()
    }

    /// The level events raised since the last drain, in the order they were
    /// raised.
    pub fn drain_level_broadcasts(&mut self) -> std::vec::Drain<'_, PendingLevelBroadcast> {
        self.pending.drain(..)
    }

    /// Whether anything is waiting for the fan-out.
    #[must_use]
    pub fn has_pending_level_events(&self) -> bool {
        !self.pending.is_empty()
            || !self.pending_sounds.is_empty()
            || !self.pending_trail_particles.is_empty()
    }

    /// The block sounds raised since the last drain, in the order they were
    /// raised.
    pub fn drain_block_sounds(&mut self) -> std::vec::Drain<'_, PendingBlockSound> {
        self.pending_sounds.drain(..)
    }

    pub fn drain_trail_particles(&mut self) -> std::vec::Drain<'_, PendingTrailParticle> {
        self.pending_trail_particles.drain(..)
    }

    /// Queues one [`PendingPlayerHurt`] for the driver.
    pub fn hurt_player(&mut self, hurt: PendingPlayerHurt) {
        self.pending_player_hurts.push(PendingPlayerHit::Hurt(hurt));
    }

    pub fn stab_player(&mut self, stab: PendingPlayerStab) {
        self.pending_player_hurts.push(PendingPlayerHit::Stab(stab));
    }

    /// The player hits queued since the last drain, in the order they were
    /// asked for.
    pub fn drain_player_hurts(&mut self) -> Vec<PendingPlayerHit> {
        std::mem::take(&mut self.pending_player_hurts)
    }

    pub fn post_attack(&mut self, post_attack: PendingPostAttack) {
        self.pending_post_attacks.push(post_attack);
    }

    pub fn drain_post_attacks(&mut self) -> Vec<PendingPostAttack> {
        std::mem::take(&mut self.pending_post_attacks)
    }

    pub fn fall_player(&mut self, fall: PendingPlayerFall) {
        self.pending_player_falls.push(fall);
    }

    pub fn drain_player_falls(&mut self) -> Vec<PendingPlayerFall> {
        std::mem::take(&mut self.pending_player_falls)
    }

    /// Queues one death's experience for the frame's post-game-event flush.
    ///
    /// The reward is read by the flush, not here, because
    /// `LivingEntity.getExperienceReward` draws off the victim's own generator
    /// (`Mob.java:308`) and the catalyst listener must be the one draw that
    /// happens before `skipDropExperience`.
    pub fn queue_death_experience(
        &mut self,
        entity_id: i32,
        position: Vec3d,
        killer_entity_id: Option<i32>,
    ) {
        self.pending_death_experience.push(PendingDeathExperience {
            entity_id,
            position,
            killer_entity_id,
        });
    }

    /// The death experience queued since the last drain, in the order the
    /// deaths raised it.
    pub fn drain_death_experience(&mut self) -> Vec<PendingDeathExperience> {
        std::mem::take(&mut self.pending_death_experience)
    }

    /// The last [`MAX_RECORDED_GAME_EVENTS`] game events, oldest first.
    ///
    /// A record, not the delivery queue.
    pub fn recent_game_events(&self) -> impl Iterator<Item = &RecordedGameEvent> {
        self.recent_game_events.iter().map(|event| &event.recorded)
    }

    /// The same bounded record with the source positions preserved.
    pub fn recent_positioned_game_events(&self) -> impl Iterator<Item = &PositionedGameEvent> {
        self.recent_game_events.iter()
    }

    /// Every game event this level ever raised, including the ones the record's
    /// bound rolled out of [`Self::recent_game_events`].
    #[must_use]
    pub const fn game_event_count(&self) -> u64 {
        self.game_event_count
    }
}

/// Whether a listener at `listener` is sent an event raised at `pos`.
///
/// `PlayerList.broadcast` (`PlayerList.java:606-616`) measures from the raw
/// block coordinates -- not the block centre -- to the recipient's feet, and
/// compares the squared distance with a strict `<` against the squared range.
#[must_use]
pub fn level_event_reaches(
    pos: BlockPos,
    range: f64,
    listener_x: f64,
    listener_y: f64,
    listener_z: f64,
) -> bool {
    let dx = f64::from(pos.x) - listener_x;
    let dy = f64::from(pos.y) - listener_y;
    let dz = f64::from(pos.z) - listener_z;
    dx * dx + dy * dy + dz * dz < range * range
}

/// Whether a listener at `listener` is sent a particle raised at `particle`.
///
/// `ServerLevel.sendParticles` (`ServerLevel.java:1362-1363`) measures from
/// `player.blockPosition().getCenter()` to the particle with a strict `<`.
#[must_use]
pub fn particle_reaches(
    particle_x: f64,
    particle_y: f64,
    particle_z: f64,
    listener_x: f64,
    listener_y: f64,
    listener_z: f64,
    override_limiter: bool,
) -> bool {
    let range = if override_limiter {
        512.0
    } else {
        LEVEL_PARTICLE_BROADCAST_RANGE
    };
    let center_x = listener_x.floor() + 0.5;
    let center_y = listener_y.floor() + 0.5;
    let center_z = listener_z.floor() + 0.5;
    let dx = center_x - particle_x;
    let dy = center_y - particle_y;
    let dz = center_z - particle_z;
    dx * dx + dy * dy + dz * dz < range * range
}

#[cfg(test)]
mod tests;
