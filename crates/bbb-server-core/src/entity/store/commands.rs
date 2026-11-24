//! Work entity callbacks hand to the world, which owns the players, maps,
//! commands and blocks a borrowed store cannot reach.

use super::{EntityStore, FramedMapRemoval, PendingPlayerKill, PendingPlayerStat};
use super::{PendingBredAnimals, PendingTamedAnimal, PendingThrownItemPickup};
use crate::entity::projectile_action::mob_effect::PendingMobEffectAction;
use crate::entity::projectile_action::PendingProjectileAction;

#[derive(Debug, Clone, PartialEq)]
pub enum EntityCommand {
    FoxMouthFood {
        entity_id: i32,
        uuid: uuid::Uuid,
    },
    LandOnPlayerShoulder {
        entity_id: i32,
        owner: uuid::Uuid,
    },
    EnchantmentFunction {
        function: String,
        executing: crate::commands::CommandSource,
        origin: crate::commands::CommandOrigin,
    },
    /// A worn enchantment's entity effect that reaches the level, players or
    /// other entities.
    EnchantmentEntityEffect {
        entity_id: i32,
        slot: bbb_protocol::equipment_slots::EquipmentSlot,
        level: i32,
        effect: crate::enchantment_entity_effects::EnchantmentEntityEffect,
        position: bbb_protocol::packets::Vec3d,
    },
    /// `LivingEntity.postPiercingAttack` (`LivingEntity.java:1674-1677`).
    PostPiercingAttack {
        entity_id: i32,
    },
    GameEvent {
        kind: crate::level_event::VanillaGameEvent,
        position: bbb_protocol::packets::Vec3d,
        source_entity_id: Option<i32>,
    },
    LoadedTeam {
        entity_id: i32,
        uuid: uuid::Uuid,
    },
    ConversionTeam {
        source: uuid::Uuid,
        target: uuid::Uuid,
    },
    MovePlayerByShulker {
        player: uuid::Uuid,
        delta: bbb_protocol::packets::Vec3d,
    },
    FinalizeShulkerSplit {
        entity_id: i32,
    },
    StartExternalPlayerRide {
        player_entity_id: i32,
        vehicle_id: i32,
    },
    GuardianThorns {
        guardian_id: i32,
        target_id: i32,
    },
    WardenDarkness {
        source: uuid::Uuid,
        position: bbb_protocol::packets::Vec3d,
    },
    AxolotlSupport {
        axolotl: uuid::Uuid,
        player: uuid::Uuid,
    },
    GoatRamPlayer {
        goat: uuid::Uuid,
        player: uuid::Uuid,
        origin: bbb_protocol::packets::Vec3d,
        damage: f32,
        knockback: f64,
        direction: bbb_protocol::packets::Vec3d,
    },
    AllayDropItemOnBlock {
        player: uuid::Uuid,
        pos: bbb_protocol::packets::BlockPos,
        item: bbb_server_types::item_stack::RuntimeItemStack,
    },
    WardenSonicBoom {
        source_entity_id: i32,
        target_entity_id: i32,
    },
    TeleportRandomly {
        entity_id: i32,
        diameter: f32,
    },
    MinecartHopperSuck {
        entity_id: i32,
    },
    HopperItemInside {
        pos: bbb_protocol::packets::BlockPos,
        entity_id: i32,
    },
    Explosion(crate::explosion::PendingExplosion),
    Projectile(PendingProjectileAction),
    ProjectileSpawned {
        entity_id: i32,
        item: bbb_server_types::item_stack::RuntimeItemStack,
    },
    EnderPearlHit {
        entity_id: i32,
        target: Option<crate::level_event::PendingHurtEntity>,
    },
    MobEffect(PendingMobEffectAction),
    PlayerKill(PendingPlayerKill),
    PlayerHurtEntity(super::PendingPlayerHurtEntity),
    PlayerStat(PendingPlayerStat),
    /// `PiglinAi.angerNearbyPiglins(level, player, true)`.
    AngerNearbyPiglins {
        player: uuid::Uuid,
    },
    TamedAnimal(PendingTamedAnimal),
    PetDamage {
        entity_id: i32,
        tick_count: i32,
        enters_combat: bool,
        entry: bbb_server_datapack::damage::CombatEntry,
        damage_type: bbb_server_datapack::damage::DamageTypeFacts,
        causing_entity: Option<crate::entity::StoredDamageCausingEntity>,
        direct_entity_id: Option<i32>,
        causing_entity_snapshot: Option<crate::explosion::ExplosionSourceEntity>,
        direct_entity_snapshot: Option<crate::explosion::ExplosionSourceEntity>,
    },
    PetDeath {
        entity_id: i32,
        damage_type: bbb_server_datapack::damage::DamageTypeFacts,
    },
    BredAnimals(PendingBredAnimals),
    ThrownItemPickup(PendingThrownItemPickup),
    FramedMapRemoval(FramedMapRemoval),
    CommandMinecart(crate::entity::PendingCommandMinecartExecution),
    SpawnerMinecart(crate::entity::PendingSpawnerMinecartAttempt),
    MobSpell(crate::entity::ai::mob::ranged::PendingMobSpell),
    PlayerPotion(crate::entity::thrown_potion::PendingPlayerPotion),
    Zombie(crate::entity::zombie::PendingZombieAction),
    LlamaCaravan(crate::entity::llama::PendingLlamaCaravan),
    RaiderLeader(crate::entity::raider::PendingRaiderLeader),
    RaiderRecruit(crate::entity::raider::PendingRaiderRecruit),
    RaiderHoldGround(crate::entity::raider::hold_ground::PendingHoldGround),
    EndermanBlockMove(crate::entity::enderman::blocks::PendingBlockMove),
    RabbitRaidCarrot(crate::entity::rabbit::PendingCarrotRaid),
    /// `Raid.updateBossbar` from a hurt raider (`Raider.java:252-257`).
    RaiderHurt {
        raid_id: i32,
    },
    /// `ItemFrame.setItem(EMPTY, true)` (`ItemFrame.java:269-287`) comparator update.
    UpdateOutputSignal(bbb_protocol::packets::BlockPos),
    /// `Creaking.hurtServer` (`Creaking.java:168-174`) notification to the bound heart.
    CreakingHeartHurt {
        home_pos: bbb_protocol::packets::BlockPos,
        protector_uuid: uuid::Uuid,
        entity_id: i32,
        center: bbb_protocol::packets::Vec3d,
        responsible_player: bool,
    },
    SkeletonHorseTrap {
        entity_id: i32,
        uuid: uuid::Uuid,
    },
    SilverfishWakeFriends {
        entity_id: i32,
        uuid: uuid::Uuid,
    },
    SilverfishMerge {
        entity_id: i32,
        uuid: uuid::Uuid,
        pos: bbb_protocol::packets::BlockPos,
    },
    Villager(crate::entity::villager::PendingVillagerAction),
    RingBell {
        pos: bbb_protocol::packets::BlockPos,
        direction: bbb_protocol::packets::Direction,
        source_entity_id: i32,
    },
    Lightning(crate::entity::lightning_bolt::LightningAction),
    /// `setStingerCount(getStingerCount() + 1)` on a stung player (`Bee.java:232`).
    PlayerStinger(uuid::Uuid),
    /// `EnderDragonFight.setDragonKilled` once the death animation ends
    /// (`EnderDragon.java:546-548`).
    DragonKilled(uuid::Uuid),
    SummonedEntity {
        entity_id: i32,
        range: f64,
    },
    PlayerItemDurabilityChanged {
        player: uuid::Uuid,
        stack: bbb_server_types::item_stack::RuntimeItemStack,
        raw_new_damage: i32,
    },
    PlayerEnteredBlock {
        player: uuid::Uuid,
        state: i32,
    },
    /// `Tadpole.ageUp`'s frog conversion (`Tadpole.java:237-246`).
    TadpoleGrowUp {
        entity_id: i32,
    },
    /// `Block.pushEntitiesUp` for a block an entity callback replaced; the
    /// lift reaches sessions too.
    PushEntitiesUp {
        old_state: i32,
        new_state: i32,
        pos: bbb_protocol::packets::BlockPos,
    },
}

impl EntityStore {
    pub fn queue_command(&mut self, command: EntityCommand) {
        self.commands.push(command);
    }

    #[must_use]
    pub fn has_pending_commands(&self) -> bool {
        !self.commands.is_empty()
    }

    pub(crate) fn has_pending_player_hurt_entity_from(&self, entity_id: i32) -> bool {
        self.commands.iter().any(|command| {
            matches!(command,
            EntityCommand::PlayerHurtEntity(hit) if hit.direct_entity_id == Some(entity_id))
        })
    }

    pub fn drain_commands(&mut self) -> Vec<EntityCommand> {
        std::mem::take(&mut self.commands)
    }

    /// Takes the queued commands `take` selects, leaving the rest in order.
    pub fn drain_commands_where(
        &mut self,
        mut take: impl FnMut(&EntityCommand) -> bool,
    ) -> Vec<EntityCommand> {
        let (taken, kept) = std::mem::take(&mut self.commands)
            .into_iter()
            .partition(|command| take(command));
        self.commands = kept;
        taken
    }
}

#[cfg(test)]
macro_rules! drain_command_kind {
    ($name:ident, $variant:ident, $payload:ty) => {
        pub(crate) fn $name(&mut self) -> Vec<$payload> {
            self.drain_commands_where(|command| matches!(command, EntityCommand::$variant(_)))
                .into_iter()
                .filter_map(|command| match command {
                    EntityCommand::$variant(payload) => Some(payload),
                    _ => None,
                })
                .collect()
        }
    };
}

#[cfg(test)]
impl EntityStore {
    drain_command_kind!(
        drain_explosions,
        Explosion,
        crate::explosion::PendingExplosion
    );
    drain_command_kind!(drain_mob_effect_actions, MobEffect, PendingMobEffectAction);
    drain_command_kind!(drain_player_kills, PlayerKill, PendingPlayerKill);
    drain_command_kind!(drain_tamed_animals, TamedAnimal, PendingTamedAnimal);
    drain_command_kind!(
        drain_framed_map_removals,
        FramedMapRemoval,
        FramedMapRemoval
    );

    pub(crate) fn has_pending_explosions(&self) -> bool {
        self.commands
            .iter()
            .any(|command| matches!(command, EntityCommand::Explosion(_)))
    }
}
