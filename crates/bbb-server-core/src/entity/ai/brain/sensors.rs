//! The sensors of `net.minecraft.world.entity.ai.sensing` this runtime runs.

use super::memory::{MemoryModuleType, MemoryValue};
use super::sensor::{BrainPlayer, EntityTargetingTest, DEFAULT_SCAN_RATE};
use super::{BrainTick, SensorType};
use crate::entity::ai::mob::frog;

impl SensorType {
    pub(super) fn requires(self) -> &'static [MemoryModuleType] {
        match self {
            Self::HurtBy => &[MemoryModuleType::HurtBy, MemoryModuleType::HurtByEntity],
            Self::IsInWater => &[MemoryModuleType::IsInWater],
            Self::NearestLivingEntities => &[
                MemoryModuleType::NearestLivingEntities,
                MemoryModuleType::NearestVisibleLivingEntities,
            ],
            Self::NearestPlayers => &[
                MemoryModuleType::NearestPlayers,
                MemoryModuleType::NearestVisiblePlayer,
                MemoryModuleType::NearestVisibleAttackablePlayer,
                MemoryModuleType::NearestVisibleAttackablePlayers,
            ],
            Self::VillagerHostiles => &[
                MemoryModuleType::NearestHostile,
                MemoryModuleType::NearestVisibleLivingEntities,
            ],
            Self::GolemDetected => &[
                MemoryModuleType::NearestLivingEntities,
                MemoryModuleType::GolemDetectedRecently,
            ],
            Self::FrogTemptations => &[MemoryModuleType::TemptingPlayer],
            Self::FrogAttackables => &[
                MemoryModuleType::NearestAttackable,
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryModuleType::UnreachableTongueTargets,
            ],
            Self::SecondaryPois => &[MemoryModuleType::SecondaryJobSite],
        }
    }

    pub(super) fn scan_rate(self) -> i32 {
        match self {
            Self::GolemDetected => 200,
            Self::SecondaryPois => 40,
            Self::HurtBy
            | Self::IsInWater
            | Self::NearestLivingEntities
            | Self::NearestPlayers
            | Self::VillagerHostiles
            | Self::FrogTemptations
            | Self::FrogAttackables => DEFAULT_SCAN_RATE,
        }
    }

    pub(super) fn do_tick(self, tick: &mut BrainTick<'_>) {
        match self {
            Self::HurtBy => hurt_by(tick),
            Self::IsInWater => is_in_water(tick),
            Self::NearestLivingEntities => nearest_living_entities(tick),
            Self::NearestPlayers => nearest_players(tick),
            Self::VillagerHostiles => villager_hostiles(tick),
            Self::GolemDetected => golem_detected(tick),
            Self::FrogTemptations => frog_temptations(tick),
            Self::FrogAttackables => frog_attackables(tick),
            Self::SecondaryPois => secondary_pois(tick),
        }
    }
}

// `HurtBySensor.doTick`.
fn hurt_by(tick: &mut BrainTick<'_>) {
    match tick.world().body().last_damage.clone() {
        Some(source) => {
            let causing = source.source_entity;
            tick.set_memory_value(MemoryModuleType::HurtBy, MemoryValue::DamageSource(source));
            if let Some(entity_id) = causing {
                // `entitySource instanceof LivingEntity`
                let living = tick
                    .world()
                    .entity_facts(entity_id)
                    .is_some_and(|facts| facts.living);
                if living {
                    tick.set_memory_value(
                        MemoryModuleType::HurtByEntity,
                        MemoryValue::EntityRef(entity_id),
                    );
                }
            }
        }
        // A null source erases only `HURT_BY`; the entity memory below is
        // judged on its own.
        None => tick.erase_memory(MemoryModuleType::HurtBy),
    }

    let remembered = tick.memory_entity(MemoryModuleType::HurtByEntity);
    if let Some(entity_id) = remembered {
        // `!isAlive() || level() != level`: this
        // world answers for an entity it no longer holds with `None`.
        let alive = tick
            .world()
            .entity_facts(entity_id)
            .is_some_and(|facts| facts.alive);
        if !alive {
            tick.erase_memory(MemoryModuleType::HurtByEntity);
        }
    }
}

// `IsInWaterSensor.doTick`.
fn is_in_water(tick: &mut BrainTick<'_>) {
    if tick.world().body().in_water {
        tick.set_memory_value(MemoryModuleType::IsInWater, MemoryValue::Unit);
    } else {
        tick.erase_memory(MemoryModuleType::IsInWater);
    }
}

// `NearestLivingEntitySensor.doTick`.
fn nearest_living_entities(tick: &mut BrainTick<'_>) {
    let follow_range = tick.world().body().follow_range;
    let nearby = tick.world().nearest_living_entities(follow_range);
    // `Brain.setMemory` erases a slot an empty list would fill
    // which `MemoryValue`'s empty-list rule keeps.
    tick.set_memory_value(
        MemoryModuleType::NearestLivingEntities,
        MemoryValue::EntityList(nearby.clone()),
    );
    tick.set_memory_value(
        MemoryModuleType::NearestVisibleLivingEntities,
        MemoryValue::VisibleEntities(nearby),
    );
}

// `PlayerSensor.doTick`.
fn nearest_players(tick: &mut BrainTick<'_>) {
    // `getFollowDistance` (`:44-46`) is the follow-range attribute.
    let follow_range = tick.world().body().follow_range;
    let mut players = tick.world().nearest_players(follow_range);
    // `EntitySelector.NO_SPECTATORS`: a session
    // player this level holds is one the world answers facts for.
    players.retain(|entity_id| {
        tick.world()
            .entity_facts(*entity_id)
            .is_some_and(|facts| !facts.spectator)
    });
    // `isEntityTargetable` (`:37`) then `isEntityAttackable` (`:39`), both
    // over the sorted list, so a singular memory is the first of each and
    // the attackable list is a filter of the visible one.
    let mut nearest_visible = None;
    let mut attackable = Vec::new();
    for &entity_id in &players {
        if is_entity_targetable(tick, entity_id) {
            nearest_visible.get_or_insert(entity_id);
            if is_entity_attackable(tick, entity_id) {
                attackable.push(entity_id);
            }
        }
    }
    let nearest_attackable = attackable.first().copied();
    tick.set_memory_value(
        MemoryModuleType::NearestPlayers,
        MemoryValue::EntityList(players),
    );
    tick.set_memory(
        MemoryModuleType::NearestVisiblePlayer,
        nearest_visible.map(MemoryValue::EntityRef),
    );
    // `setMemory` erases a slot a list would empty,
    // which `MemoryValue`'s empty-list rule keeps.
    tick.set_memory_value(
        MemoryModuleType::NearestVisibleAttackablePlayers,
        MemoryValue::EntityList(attackable),
    );
    tick.set_memory(
        MemoryModuleType::NearestVisibleAttackablePlayer,
        nearest_attackable.map(MemoryValue::EntityRef),
    );
}

/// `VillagerHostilesSensor.ACCEPTABLE_DISTANCE_FROM_HOSTILES`
/// (`VillagerHostilesSensor.java:9-21`).
const VILLAGER_HOSTILES: [(&str, f64); 11] = [
    ("minecraft:drowned", 8.0),
    ("minecraft:evoker", 12.0),
    ("minecraft:husk", 8.0),
    ("minecraft:illusioner", 12.0),
    ("minecraft:pillager", 15.0),
    ("minecraft:ravager", 12.0),
    ("minecraft:vex", 8.0),
    ("minecraft:vindicator", 10.0),
    ("minecraft:zoglin", 10.0),
    ("minecraft:zombie", 8.0),
    ("minecraft:zombie_villager", 8.0),
];

fn villager_hostiles(tick: &mut BrainTick<'_>) {
    let nearby = match tick.memory(MemoryModuleType::NearestVisibleLivingEntities) {
        Some(MemoryValue::VisibleEntities(ids)) => ids.as_slice(),
        _ => &[],
    };
    let nearest = nearby.iter().copied().find(|entity_id| {
        let Some(facts) = tick.world().entity_facts(*entity_id) else {
            return false;
        };
        VILLAGER_HOSTILES
            .iter()
            .find(|(id, _)| *id == facts.entity_resource_id)
            .is_some_and(|(_, reach)| {
                tick.world()
                    .entity_distance_to_sqr(*entity_id)
                    .is_some_and(|distance| distance <= reach * reach)
            })
            && is_entity_targetable(tick, *entity_id)
    });
    tick.set_memory(
        MemoryModuleType::NearestHostile,
        nearest.map(MemoryValue::EntityRef),
    );
}

/// `GolemSensor.MEMORY_TIME_TO_LIVE` (`GolemSensor.java:13`).
const GOLEM_MEMORY_TIME_TO_LIVE: i64 = 599;

fn golem_detected(tick: &mut BrainTick<'_>) {
    let golem_present = match tick.memory(MemoryModuleType::NearestLivingEntities) {
        Some(MemoryValue::EntityList(ids)) => ids.iter().any(|entity_id| {
            tick.world()
                .entity_facts(*entity_id)
                .is_some_and(|facts| facts.entity_resource_id == "minecraft:iron_golem")
        }),
        _ => false,
    };
    if golem_present {
        tick.set_memory_with_expiry(
            MemoryModuleType::GolemDetectedRecently,
            MemoryValue::Bool(true),
            GOLEM_MEMORY_TIME_TO_LIVE,
        );
    }
}

/// `Sensor.isEntityTargetable`.
#[inline(always)]
pub(super) fn is_entity_targetable(tick: &BrainTick<'_>, entity_id: i32) -> bool {
    tick.world().targeting_test(EntityTargetingTest {
        entity_id,
        combat: false,
        // `updateTargetingConditionRanges` moves the
        // static condition to the body's follow range before every scan.
        range: tick.world().body().follow_range,
        ignore_line_of_sight: false,
        ignore_invisibility: is_current_attack_target(tick, entity_id),
    })
}

/// `Sensor.isEntityAttackable`.
#[inline(always)]
fn is_entity_attackable(tick: &BrainTick<'_>, entity_id: i32) -> bool {
    tick.world().targeting_test(EntityTargetingTest {
        entity_id,
        combat: true,
        range: tick.world().body().follow_range,
        ignore_line_of_sight: false,
        ignore_invisibility: is_current_attack_target(tick, entity_id),
    })
}

// `TemptingSensor.doTick`. The scanner is
// the body's own level walk, whose order is the sort the
// nearest-first pick below reads.
fn frog_temptations(tick: &mut BrainTick<'_>) {
    // `targeting.copy().range(body.getAttributeValue(TEMPT_RANGE))`
    // (`:39`) over `forNonCombat().ignoreLineOfSight()` (`:22`), which is
    // the condition `TemptingSensor` builds for itself rather than the
    // `Sensor` one `isEntityTargetable` tests with.
    let range = tick.world().body().tempt_range;
    let nearest = tick
        .world()
        .players_within(range)
        .into_iter()
        .find(|player| {
            // `EntitySelector.NO_SPECTATORS` (`:42`) and `targeting.test`
            // (`:43`) are one term here -- the condition's own
            // `canBeSeenByAnyone()` refuses a spectator before it looks at
            // the range -- followed by `playerHoldingTemptation` (`:44`)
            // and `!body.hasPassenger(player)` (`:45`).
            player_is_seen(tick, player.entity_id, range)
                && player_holds_temptation(tick, *player)
                && !tick.world().body_has_passenger(player.entity_id)
        })
        .map(|player| player.entity_id);
    match nearest {
        // `brain.setMemory(TEMPTING_PLAYER, players.get(0))` (`:48-50`).
        Some(entity_id) => tick.set_memory_value(
            MemoryModuleType::TemptingPlayer,
            MemoryValue::EntityRef(entity_id),
        ),
        // `brain.eraseMemory(TEMPTING_PLAYER)` (`:52`).
        None => tick.erase_memory(MemoryModuleType::TemptingPlayer),
    }
}

/// `targeting.test(level, body, player)` under the
/// condition and build.
fn player_is_seen(tick: &BrainTick<'_>, entity_id: i32, range: f64) -> bool {
    // `canBeSeenByAnyone`, the term
    // `targeting_test` applies to every target it is handed and the one
    // `EntitySelector.NO_SPECTATORS` makes here.
    tick.world().targeting_test(EntityTargetingTest {
        entity_id,
        combat: false,
        range,
        ignore_line_of_sight: true,
        ignore_invisibility: false,
    })
}

/// `TemptingSensor.playerHoldingTemptation` over
/// `FrogAi.getTemptations`'s `ItemTags.FROG_FOOD` predicate
fn player_holds_temptation(tick: &BrainTick<'_>, player: BrainPlayer) -> bool {
    // An empty hand is `Items.AIR`, which no tempting tag holds. A tag the
    // world cannot answer holds nothing either, which leaves the slot erased
    // rather than inventing a temptation.
    [player.main_hand_item_id, player.off_hand_item_id]
        .into_iter()
        .flatten()
        .any(|item_type_id| tick.world().item_in_tag(item_type_id, frog::FROG_FOOD_TAG))
}

// `NearestVisibleLivingEntitySensor.doTick`
// over `getNearestEntity`.
fn frog_attackables(tick: &mut BrainTick<'_>) {
    let nearby = match tick.memory(MemoryModuleType::NearestVisibleLivingEntities) {
        Some(MemoryValue::VisibleEntities(ids)) => ids.as_slice(),
        // `getVisibleEntities`
        // is an `Optional`, so a missing or empty memory sets nothing.
        _ => &[],
    };
    let unreachable = match tick.memory(MemoryModuleType::UnreachableTongueTargets) {
        Some(MemoryValue::UuidList(uuids)) => uuids.as_slice(),
        _ => &[],
    };
    let nearest = nearby.iter().copied().find(|entity_id| {
        is_matching_entity(tick, *entity_id, unreachable) && is_entity_targetable(tick, *entity_id)
    });
    match nearest {
        Some(entity_id) => tick.set_memory_value(
            MemoryModuleType::NearestAttackable,
            MemoryValue::EntityRef(entity_id),
        ),
        None => tick.erase_memory(MemoryModuleType::NearestAttackable),
    }
}

/// `FrogAttackablesSensor.isMatchingEntity`,
/// whose `mob.closerThan(body, 10.0)` term compares full positions rather than
/// block centres (`Entity.closerThan`).
fn is_matching_entity(tick: &BrainTick<'_>, entity_id: i32, unreachable: &[uuid::Uuid]) -> bool {
    let Some(facts) = tick.world().entity_facts(entity_id) else {
        return false;
    };
    // `Sensor.isEntityAttackable` && `Frog.canEat`
    // && `!isUnreachableAttackTarget`
    // then the ten-block reach.
    is_entity_attackable(tick, entity_id)
        && frog::can_eat(tick.world(), &facts)
        && !unreachable.contains(&facts.uuid)
        && tick
            .world()
            .entity_distance_to_sqr(entity_id)
            .is_some_and(|distance| distance < 100.0)
}

/// `body.getBrain().isMemoryValue(MemoryModuleType.ATTACK_TARGET, entity)`,
/// which both conditions swap `ignoreInvisibilityTesting` on
fn is_current_attack_target(tick: &BrainTick<'_>, entity_id: i32) -> bool {
    tick.is_memory_value(
        MemoryModuleType::AttackTarget,
        &MemoryValue::EntityRef(entity_id),
    )
}

#[cfg(test)]
mod tests;

fn secondary_pois(tick: &mut BrainTick<'_>) {
    let body = tick.world().body();
    let farmer = body
        .villager
        .is_some_and(|villager| villager.profession == "minecraft:farmer");
    let sites = if farmer {
        tick.world().farmland_around(body.block_position)
    } else {
        Vec::new()
    };
    if sites.is_empty() {
        tick.erase_memory(MemoryModuleType::SecondaryJobSite);
        return;
    }
    let dimension = tick.world().dimension().to_owned();
    tick.set_memory_value(
        MemoryModuleType::SecondaryJobSite,
        MemoryValue::SameDimensionPosList {
            dimension,
            positions: sites,
        },
    );
}
