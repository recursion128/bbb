//! `EvokerFangs`: a warmup-then-strike `Entity` that deals one burst of magic
//! damage to the living entities standing over it, then discards itself.
//!

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID;
use bbb_protocol::packets::{EntityEvent, Vec3d};

use crate::entity::damage::{StoredDamageCausingEntity, StoredEntityDamageRules};
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{base_tick, Entity, EntityData};
use crate::level_event::{PendingHurtEntity, PendingPlayerHurt};
use bbb_server_datapack::damage::DamageTypeFacts;

/// `EvokerFangs.tick`'s damage sweep offset (`EvokerFangs.java:88`): the fang
/// hits every living entity in its bounding box grown by this on the two
/// horizontal axes.
const DAMAGE_INFLATE_XZ: f64 = 0.2;

/// The initial `lifeTicks` (`EvokerFangs.java:26`), which is
/// `ATTACK_DURATION + LIFE_OFFSET` (`:20-21`).
const INITIAL_LIFE_TICKS: i32 = 22;

/// `EvokerFangs.tick`'s strike tick (`EvokerFangs.java:87`): the fang deals its
/// damage on the tick `warmupDelayTicks` reaches this value.
const STRIKE_WARMUP_TICKS: i32 = -8;

/// The entity event the server broadcasts once the fang starts its strike
/// (`EvokerFangs.java:94`, `(byte)4`), which the client turns into the attack
/// animation and sound (`:123-140`).
const EVENT_ATTACK_START: i8 = 4;

/// `EvokerFangs.dealDamageTo`'s damage amount (`EvokerFangs.java:108`, `:115`).
const DAMAGE: f32 = 6.0;

/// `Entity.getDefaultGravity()` (`Entity.java:1432-1434`): the fang inherits the
/// zero default and never moves, so gravity is never applied either way.
pub const DEFAULT_GRAVITY: f64 = 0.0;

///
/// `clientSideAttackStarted` (`:27`) is a client-only animation flag and is not
/// kept. `lifeTicks` and `sentSpikeEvent` are runtime-only: neither is written
/// by `addAdditionalSaveData` (`:62-66`), so a reloaded fang restarts them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvokerFangsData {
    warmup_delay_ticks: i32,
    sent_spike_event: bool,
    life_ticks: i32,
    /// `EntityReference<LivingEntity> owner` (`:28`) reduced to its UUID, which
    /// is what `EntityReference.store`/`read` (`:65`, `:59`) round-trips.
    owner: Option<uuid::Uuid>,
}

impl EvokerFangsData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.sent_spike_event = previous.sent_spike_event;
        self.life_ticks = previous.life_ticks;
    }

    /// The fresh state a constructed fang holds (`EvokerFangs.java:24-26`).
    pub const fn new(warmup_delay_ticks: i32, owner: Option<uuid::Uuid>) -> Self {
        Self {
            warmup_delay_ticks,
            sent_spike_event: false,
            life_ticks: INITIAL_LIFE_TICKS,
            owner,
        }
    }

    /// `EvokerFangs.readAdditionalSaveData` (`EvokerFangs.java:57-60`).
    pub const fn from_saved(warmup_delay_ticks: i32, owner: Option<uuid::Uuid>) -> Self {
        Self::new(warmup_delay_ticks, owner)
    }

    pub const fn warmup_delay_ticks(&self) -> i32 {
        self.warmup_delay_ticks
    }

    pub const fn owner(&self) -> Option<uuid::Uuid> {
        self.owner
    }

    pub const fn sent_spike_event(&self) -> bool {
        self.sent_spike_event
    }

    pub const fn life_ticks(&self) -> i32 {
        self.life_ticks
    }
}

pub fn spawn_evoker_fangs(
    store: &mut EntityStore,
    position: Vec3d,
    rotation_radians: f32,
    warmup_delay_ticks: i32,
    owner: Option<uuid::Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let data = EvokerFangsData::new(warmup_delay_ticks, owner);
    // `:40`: `setYRot(rotationRadians * (180 / PI))`.
    let y_rot = rotation_radians * (180.0 / std::f32::consts::PI);
    let id = store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
        position,
        EntityData::EvokerFangs(data),
    )?;
    let entity = store.get_mut(id).expect("restore leaves the entity live");
    entity.set_rot(y_rot, 0.0);
    entity.set_needs_sync(true);
    Some(id)
}

///
/// `super.tick()` is `Entity.tick` -> `baseTick` (`:70`); the fang neither
/// moves nor applies gravity, so nothing else of the base tick is relevant.
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let EntityData::EvokerFangs(mut data) = entity.data.clone() else {
        return true;
    };
    // `:70`.
    if !base_tick(entity, store, level) {
        entity.data = EntityData::EvokerFangs(data);
        return false;
    }
    // `:86`: `--this.warmupDelayTicks < 0`. The decrement is unconditional on
    // the server (the client branch is never taken here).
    data.warmup_delay_ticks -= 1;
    if data.warmup_delay_ticks < 0 {
        // `:87-91`.
        if data.warmup_delay_ticks == STRIKE_WARMUP_TICKS
            && !deal_damage_to_all(entity, store, level, &data)
        {
            return false;
        }
        // `:93-96`.
        if !data.sent_spike_event {
            store.broadcast_to_tracking(
                entity.id(),
                crate::connection::ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id: entity.id(),
                    event_id: EVENT_ATTACK_START,
                }),
            );
            data.sent_spike_event = true;
        }
        // `:98-100`.
        data.life_ticks -= 1;
        if data.life_ticks < 0 {
            entity.discard();
        }
    }
    entity.data = EntityData::EvokerFangs(data);
    true
}

fn deal_damage_to_all(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    data: &EvokerFangsData,
) -> bool {
    let sweep = entity
        .bounding_box()
        .inflate(DAMAGE_INFLATE_XZ, 0.0, DAMAGE_INFLATE_XZ);
    let owner = resolve_owner(data.owner, store, level);

    let stored_targets: Vec<i32> = store
        .ids_in_box_sections(sweep)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|target| {
                target.living().is_some() && target.bounding_box().intersects(sweep)
            })
        })
        .collect();
    for target_id in stored_targets {
        if !deal_damage_to_stored(entity, store, level, target_id, &owner) {
            return false;
        }
    }

    for player in level.players_in_box(sweep) {
        if owner.is_some_and(|owner| owner.is_uuid(player.uuid)) {
            continue;
        }
        if player.dead_or_dying || player.invulnerable {
            continue;
        }
        if owner.is_some_and(|owner| owner_allied_to(entity, store, level, owner, player.entity_id))
        {
            continue;
        }
        let (damage_type, causing) = match owner {
            None => ("minecraft:magic", None),
            Some(owner) => ("minecraft:indirect_magic", Some(owner.as_pending_entity())),
        };
        level.hurt_player(PendingPlayerHurt {
            player: player.uuid,
            damage_type,
            amount: DAMAGE,
            post_attack: causing.is_some(),
            impulse: None,
            direct: causing.as_ref().map(|_| PendingHurtEntity::Stored {
                entity_id: entity.id(),
            }),
            causing,
        });
    }
    true
}

fn deal_damage_to_stored(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    owner: &Option<OwnerResolution>,
) -> bool {
    let fangs_position = entity.position();
    // `:106`: skip the owner and any dead/invulnerable target.
    let Some(target) = store.get(target_id) else {
        return true;
    };
    if owner.is_some_and(|owner| owner.is_uuid(target.uuid())) {
        return true;
    }
    if !target.is_alive() || target.invulnerable() {
        return true;
    }
    // `EvokerFangs.java:110-112`.
    if owner.is_some_and(|owner| owner_allied_to(entity, store, level, owner, target_id)) {
        return true;
    }
    let Some(facts) = damage_type_facts(match owner {
        None => "minecraft:magic",
        Some(_) => "minecraft:indirect_magic",
    }) else {
        return true;
    };
    let source = crate::entity::damage::StoredEntityDamageSource {
        damage_type: facts,
        // `indirectMagic(this, owner)` direct entity is the fang; `magic()` has none.
        direct_entity_id: owner.map(|_| entity.id()),
        causing_entity: owner.map(OwnerResolution::as_stored_causing),
        attacker_views: Default::default(),
    };
    let rules = StoredEntityDamageRules {
        mob_griefing: level.mob_griefing(),
        entity_drops: level.entity_drops(),
        ..StoredEntityDamageRules::default()
    };
    enum HurtKind {
        General,
        ArmorStand(crate::entity::armor_stand::HurtDecision),
    }
    let kind = if matches!(target.data(), EntityData::GeneralLiving(_)) {
        Some(HurtKind::General)
    } else if matches!(target.data(), EntityData::ArmorStand(_)) {
        let Some(decision) = crate::entity::armor_stand::hurt_decision_in_level(
            target,
            source,
            rules,
            crate::entity::damage::StoredEntityDamageContext {
                block_drops: level.block_drops(),
                ..crate::entity::damage::StoredEntityDamageContext::default()
            },
            level,
        ) else {
            return false;
        };
        Some(HurtKind::ArmorStand(decision))
    } else {
        None
    };
    let Some(mut target) = store.detach(target_id) else {
        return true;
    };
    if let Some(kind) = kind {
        let position = target.position();
        let mut general_effects = None;
        let mut armor_stand_entity_die = false;
        let hurt;
        let (emits_entity_damage, source_id) = match kind {
            HurtKind::General => {
                let effects = crate::entity::general_living::hurt_live_entity(
                    &mut target,
                    store,
                    level,
                    None,
                    source,
                    None,
                    None,
                    owner.map(|_| "minecraft:evoker_fangs"),
                    DAMAGE,
                    rules,
                    owner.map(|_| fangs_position),
                    None,
                    None,
                );
                general_effects = Some(effects);
                hurt = effects.outcome.succeeded;
                (effects.emits_entity_damage, effects.game_event_source_id)
            }
            HurtKind::ArmorStand(decision) => {
                let Some(effects) = crate::entity::armor_stand::hurt_server_with_decision(
                    &mut target,
                    store,
                    decision,
                    level.block_drops(),
                ) else {
                    store.attach(target);
                    return false;
                };
                armor_stand_entity_die = effects.emits_entity_die;
                hurt = effects.outcome.succeeded;
                (
                    effects.emits_entity_damage,
                    owner.map(OwnerResolution::entity_id),
                )
            }
        };
        if emits_entity_damage {
            level.game_event_at(
                crate::level_event::VanillaGameEvent::ENTITY_DAMAGE,
                position,
                source_id,
            );
        }
        if armor_stand_entity_die {
            crate::entity::kill::raise_entity_die(&target, level);
        }
        if let Some(effects) = general_effects {
            if crate::entity::general_living::emit_entity_die(
                store,
                level,
                &mut target,
                source,
                effects,
            )
            .is_none()
            {
                store.attach(target);
                return false;
            }
        }
        if hurt {
            if let Some(owner) = owner {
                level.post_attack(crate::level_event::PendingPostAttack {
                    victim_entity_id: target_id,
                    damage_type: "minecraft:indirect_magic",
                    causing: Some(owner.as_pending_entity()),
                    direct: Some(PendingHurtEntity::Stored {
                        entity_id: entity.id(),
                    }),
                });
            }
        }
    }
    store.attach(target);
    true
}

/// `currentOwner.isAlliedTo(target)`.
fn owner_allied_to(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    owner: OwnerResolution,
    target_id: i32,
) -> bool {
    let sensing = Default::default();
    let view = crate::entity::ai::mob::target::TargetView::new(entity, store, level, &sensing);
    let (Some(owner), Some(target)) = (view.facts(owner.entity_id()), view.facts(target_id)) else {
        return false;
    };
    crate::entity::ai::mob::target::conditions::is_allied_to(&view, &owner, &target)
}

/// The resolved `getOwner()` of a fang: the owner is either a live player of
/// the level or a stored living entity, and each carries the id and creative
/// flag `dealDamageTo`'s `indirectMagic` source needs.
#[derive(Debug, Clone, Copy)]
enum OwnerResolution {
    Player {
        uuid: uuid::Uuid,
        entity_id: i32,
        instabuild: bool,
    },
    Stored {
        uuid: uuid::Uuid,
        entity_id: i32,
        is_mob: bool,
    },
}

impl OwnerResolution {
    fn is_uuid(self, uuid: uuid::Uuid) -> bool {
        match self {
            Self::Player { uuid: owner, .. } | Self::Stored { uuid: owner, .. } => owner == uuid,
        }
    }

    fn entity_id(self) -> i32 {
        match self {
            Self::Player { entity_id, .. } | Self::Stored { entity_id, .. } => entity_id,
        }
    }

    fn as_pending_entity(self) -> PendingHurtEntity {
        match self {
            Self::Player { uuid, .. } => PendingHurtEntity::Player { uuid },
            Self::Stored { entity_id, .. } => PendingHurtEntity::Stored { entity_id },
        }
    }

    fn as_stored_causing(self) -> StoredDamageCausingEntity {
        match self {
            Self::Player {
                uuid,
                entity_id,
                instabuild,
            } => StoredDamageCausingEntity::Player {
                uuid,
                entity_id,
                instabuild,
            },
            Self::Stored {
                entity_id,
                is_mob: true,
                ..
            } => StoredDamageCausingEntity::Mob { entity_id },
            Self::Stored { entity_id, .. } => StoredDamageCausingEntity::Other { entity_id },
        }
    }
}

fn resolve_owner(
    owner: Option<uuid::Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<OwnerResolution> {
    let owner = owner?;
    if let Some(player) = level.player(owner) {
        if !player.dead_or_dying {
            return Some(OwnerResolution::Player {
                uuid: owner,
                entity_id: player.entity_id,
                instabuild: player.instabuild,
            });
        }
    }
    // The stored living entities are the armor stands; find one by UUID.
    if let Some(entity) = store.entity_id_for_uuid(owner).and_then(|id| store.get(id)) {
        if entity.uuid() == owner && entity.living().is_some() && entity.is_alive() {
            return Some(OwnerResolution::Stored {
                uuid: owner,
                entity_id: entity.id(),
                is_mob: matches!(entity.data(), EntityData::GeneralLiving(_)),
            });
        }
    }
    None
}

/// One `minecraft:damage_type` from the built-in registry.
fn damage_type_facts(resource_id: &str) -> Option<&'static DamageTypeFacts> {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
}

pub fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    crate::entity::living::entity_synched_values(entity)
}
