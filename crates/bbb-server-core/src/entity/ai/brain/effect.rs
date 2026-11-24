//! The body and world actions one brain tick asks for.
//!
//! A brain is stored across ticks, so a behaviour cannot act on the level the
//! way `ShootTongue.eatEntity` acts on its `ServerLevel`: the body it acts on
//! is detached from the store while the tick runs
//! (`EntityStore::detach`). A behaviour records the call it would have made,
//! and the production owner replays it where the store and the level are both
//! writable, in the order the behaviour recorded it.

use bbb_protocol::packets::{BlockPos, Vec3d};

use super::memory::{MemoryModuleType, MemoryValue};

/// One `ServerLevel`/`Mob` call a behaviour made, held until the tick's store
/// borrow ends.
#[derive(Debug, Clone, PartialEq)]
pub enum BrainWorldEffect {
    /// `body.setPose(pose)`.
    SetPose { pose_id: i32 },
    /// `level.playSound(null, body, sound, source, volume, pitch)`
    PlayBodySound {
        resource_id: &'static str,
        source: bbb_protocol::packets::SoundSource,
        volume: f32,
        pitch: f32,
    },
    /// `target.setDeltaMovement(movement)`.
    SetDeltaMovement { entity_id: i32, movement: Vec3d },
    /// `body.setDeltaMovement(movement)`, which the long jump writes on the
    /// launching tick and damps on landing
    SetBodyDeltaMovement { movement: Vec3d },
    /// `body.setYRot(body.yBodyRot)`.
    SetBodyYRot { y_rot: f32 },
    /// `body.setDiscardFriction(discard)` (`LongJumpToRandomPos`,
    /// `LongJumpMidJump`), which `LivingEntity.travelInAir`
    /// branches on.
    SetBodyDiscardFriction { discard: bool },
    /// `level.setBlock(pos, state, 3)` with the `GameEvent.BLOCK_PLACE` it
    /// emits.
    PlaceBlock { pos: BlockPos, block_state_id: i32 },
    /// `body.doHurtTarget(level, target)` (`Mob.doHurtTarget`,
    /// `Mob`) by the body that owns the brain, together with the
    /// `target.remove(KILLED)` behind it. The two are one effect because nobody
    /// can read the target's health between them: the store applies this after
    /// the brain's tick, so `remove_if_killed` stands for the
    /// `if (!target.isAlive())` of `ShootTongue.eatEntity`
    HurtEntity {
        target_id: i32,
        damage_type: &'static str,
        amount: f32,
        remove_if_killed: bool,
    },
    /// `body.swing(InteractionHand.MAIN_HAND)` over
    /// `LivingEntity.swing`, whose
    /// `ClientboundAnimatePacket` the replayer sends.
    SwingMainHand,
    /// `body.doHurtTarget(level, target)` with the
    /// body's own override, from `MeleeAttack`.
    MeleeAttack { target_id: i32 },
    /// Ordered memory writes on the tick body or its breeding partner.
    /// `None` is `eraseMemory`.
    SetMobBrainMemory {
        entity_id: i32,
        memory_type: MemoryModuleType,
        value: Option<MemoryValue>,
    },
    /// `body.spawnChildFromBreeding(level, partner)`
    /// under `Frog`'s override
    SpawnChildFromBreeding { partner_id: i32 },
    /// `level.broadcastEntityEvent(body, id)`.
    BodyEntityEvent { event_id: i8 },
    /// `Villager.setVillagerData(data.withProfession(..))`.
    SetVillagerProfession { profession: &'static str },
    /// `assignProfessionWhenSpawned = false` after the brain tick.
    ClearAssignProfessionWhenSpawned,
    /// `WorkAtPoi.start`'s `useWorkstation` at the job site
    VillagerWorkedAtPoi { job_site: BlockPos },
    /// `level.broadcastEntityEvent(entity, id)` for the body or another mob.
    EntityEvent { entity_id: i32, event_id: i8 },
    /// `VillagerMakeLove.tick`'s birth.
    VillagerGiveBirth { partner_id: i32 },
    /// `HarvestFarmland.tick`'s harvest and sowing of one cell.
    VillagerFarmAt { pos: BlockPos },
    /// `TradeWithVillager.tick` with the target close
    TradeWithVillager { target_id: i32 },
    /// `LivingEntity.startSleeping(pos)`.
    StartSleeping { pos: BlockPos },
    /// `LivingEntity.stopSleeping()` of a sleeping body.
    StopSleeping,
    /// `Villager.spawnGolemIfNeeded(level, timestamp, needed)`.
    VillagerSpawnGolem { needed: usize },
    /// `UseBonemeal.tick`'s `BoneMealItem.growCrop` with one inventory bone
    /// meal.
    VillagerBonemealAt { pos: BlockPos },
    /// `body.setItemSlot(MAINHAND, ..)` from `UseBonemeal` or
    /// `ShowTradesToPlayer`.
    VillagerHeldItem(VillagerHeldItem),
}

/// What a villager's work and trade behaviours put in its main hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillagerHeldItem {
    Clear,
    BoneMeal,
    /// The `index`th in-stock offer result whose cost is `item_id`, cycling;
    /// cleared when none matches.
    Trade {
        item_id: i32,
        index: usize,
    },
}
