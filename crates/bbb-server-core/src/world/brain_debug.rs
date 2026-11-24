//! The dump the `minecraft:brains` debug channel publishes for one mob.
//!
//! `Mob.registerDebugValues` (`Mob.java:1439-1441`) registers the source for
//! every mob whose brain is not brain-dead, and the value is
//! `DebugBrainDump.takeBrainDump` (`DebugBrainDump.java:93-128`).
//!
//! A mob whose brain this server has built answers from that live brain:
//! its registered slots with the values they hold now, the activities it has
//! active and the behaviours it is running. A mob with no live brain -- one
//! whose type has no provider yet, or one that has never been allowed to step
//! (`Mob.isEffectiveAi`, `Mob.java:1278-1280`) -- answers with the brain a
//! fresh load holds instead: the memories the save handed it, the activity its
//! schedule picks and no running behaviour. A type [`brain_shape`] does not
//! name and that has no live brain publishes nothing, which reads on the wire
//! like the brain-dead answer.

use crate::world::LevelId;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Write as _},
};

use crate::entity::ai::{
    mob::{creaking::CREAKING_MEMORY_MODULES, frog::FROG_MEMORY_MODULES},
    Brain, BrainPacked, MemoryModuleType, MemoryValue,
};
use crate::entity::{Entity, EntityData, EntityStore, GeneralLivingData};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
};
use bbb_protocol::packets::{BlockPos, DebugBrainDump};

use super::ServerWorld;

/// One entity type's brain as its `Brain.Provider` builds it: a slot per
/// sensor requirement and per behaviour requirement (`Brain`, `Brain.java:70-94`).
struct BrainShape {
    /// The registry paths `MemoryModuleType` is keyed by, which is what a
    /// memory line names (`DebugBrainDump.java:189`).
    memory_modules: &'static [MemoryModuleType],
    /// Whether the type hands `Brain.setSchedule` an activity attribute, which
    /// is what `updateActivityFromSchedule` reads (`Brain.java:327-334`).
    scheduled: bool,
    /// The activities the type registers, with the memory each one requires to
    /// be allowed (`Brain.activityRequirementsAreMet`, `Brain.java:434-447`).
    activities: &'static [(&'static str, Option<&'static str>)],
}

/// `Villager.BRAIN_PROVIDER` (`Villager.java:129-170`): nine sensors and the
/// `VillagerGoalPackages` the activity supplier names. The sensor half is
/// `NearestItemSensor.java:20-22`, `NearestLivingEntitySensor.java:28-30`,
/// `PlayerSensor.java:18-25`, `NearestBedSensor.java:35-37`,
/// `HurtBySensor.java:14-16`, `VillagerHostilesSensor.java:35-37` through
/// `NearestVisibleLivingEntitySensor.java:16-18`, `VillagerBabiesSensor.java:15-17`,
/// `SecondaryPoiSensor.java:49-51` and `GolemSensor.java:29-31`; the rest is
/// what the packages' behaviours require, from `InteractWithDoor`, `WakeUp`,
/// `SleepInBed`, `ReactToBell`, `LocateHidingPlace`, `MoveToTargetSink`,
/// `GoToWantedItem`, `AcquirePoi`, `InteractWith` and `SetEntityLookTarget`
/// (`VillagerGoalPackages.java:32-61`, `:68-100`, `:127-172`, `:174-208`).
const VILLAGER_MEMORY_MODULES: &[MemoryModuleType] = &[
    MemoryModuleType::BreedTarget,
    MemoryModuleType::CantReachWalkTargetSince,
    MemoryModuleType::DoorsToClose,
    MemoryModuleType::GolemDetectedRecently,
    MemoryModuleType::HeardBellTime,
    MemoryModuleType::HidingPlace,
    MemoryModuleType::Home,
    MemoryModuleType::HurtBy,
    MemoryModuleType::HurtByEntity,
    MemoryModuleType::InteractionTarget,
    MemoryModuleType::ItemPickupCooldownTicks,
    MemoryModuleType::JobSite,
    MemoryModuleType::LastSlept,
    MemoryModuleType::LastWoken,
    MemoryModuleType::LookTarget,
    MemoryModuleType::MeetingPoint,
    MemoryModuleType::NearestLivingEntities,
    MemoryModuleType::NearestBed,
    MemoryModuleType::NearestHostile,
    MemoryModuleType::NearestPlayers,
    MemoryModuleType::NearestVisiblePlayer,
    MemoryModuleType::NearestVisibleAttackablePlayer,
    MemoryModuleType::NearestVisibleAttackablePlayers,
    MemoryModuleType::NearestVisibleWantedItem,
    MemoryModuleType::Path,
    MemoryModuleType::PotentialJobSite,
    MemoryModuleType::SecondaryJobSite,
    MemoryModuleType::NearestVisibleLivingEntities,
    MemoryModuleType::VisibleVillagerBabies,
    MemoryModuleType::WalkTarget,
];

/// `CreakingAi.getActivities` (`CreakingAi.java:75-77`): `CORE` and `IDLE`
/// carry no condition, `FIGHT` carries the `ATTACK_TARGET`
/// `ActivityData.create` was handed (`CreakingAi.java:57-67`).
const CREAKING_ACTIVITIES: &[(&str, Option<&str>)] = &[
    ("core", None),
    ("idle", None),
    ("fight", Some("attack_target")),
];

/// The activities the villager supplier registers for an adult and for a baby
/// (`Villager.java:141-168`), with the memory `ActivityData.create` turns into
/// a condition (`ActivityData.java:27-38`, `Villager.java:146-152`, `:157-161`).
const VILLAGER_ADULT_ACTIVITIES: &[(&str, Option<&str>)] = &[
    ("core", None),
    ("work", Some("job_site")),
    ("meet", Some("meeting_point")),
    ("rest", None),
    ("idle", None),
    ("panic", None),
    ("pre_raid", None),
    ("raid", None),
    ("hide", None),
];
const VILLAGER_BABY_ACTIVITIES: &[(&str, Option<&str>)] = &[
    ("core", None),
    ("play", None),
    ("meet", Some("meeting_point")),
    ("rest", None),
    ("idle", None),
    ("panic", None),
    ("pre_raid", None),
    ("raid", None),
    ("hide", None),
];

/// `FrogAi.getActivities` (`FrogAi.java:67-68`). None of the five carries a
/// condition a schedule could select on: the frog hands `setSchedule` nothing,
/// so its brain keeps the activity its constructor set.
const FROG_ACTIVITIES: &[(&str, Option<&str>)] = &[
    ("core", None),
    ("idle", None),
    ("swim", None),
    ("lay_spawn", None),
    ("tongue", None),
    ("long_jump", None),
];

fn brain_shape(entity_type_id: i32, baby: bool) -> Option<BrainShape> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_VILLAGER_ID => Some(BrainShape {
            memory_modules: VILLAGER_MEMORY_MODULES,
            scheduled: true,
            activities: if baby {
                VILLAGER_BABY_ACTIVITIES
            } else {
                VILLAGER_ADULT_ACTIVITIES
            },
        }),
        VANILLA_ENTITY_TYPE_FROG_ID => Some(BrainShape {
            memory_modules: FROG_MEMORY_MODULES,
            scheduled: false,
            activities: FROG_ACTIVITIES,
        }),
        VANILLA_ENTITY_TYPE_CREAKING_ID => Some(BrainShape {
            memory_modules: CREAKING_MEMORY_MODULES,
            scheduled: false,
            activities: CREAKING_ACTIVITIES,
        }),
        _ => None,
    }
}

/// `Brain.defaultActivity` (`Brain.java:48`), which nothing here reassigns.
const DEFAULT_ACTIVITY: &str = "idle";

/// `Brain.coreActivities` as every provider-built brain sets it
/// (`Brain.java:100`).
const CORE_ACTIVITY: &str = "core";

/// What an empty memory slot is published as (`DebugBrainDump.java:186`).
const EMPTY_MEMORY: &str = "-";

/// `StringUtil.truncateStringIfNecessary(description, 255, true)`
/// (`DebugBrainDump.java:164`).
const MEMORY_LINE_LIMIT: usize = 255;

fn memory_lines(shape: &BrainShape, packed: &BrainPacked, game_time: i64) -> Vec<String> {
    let mut lines = Vec::with_capacity(shape.memory_modules.len());
    for module in shape.memory_modules {
        let stored = packed
            .memories
            .iter()
            .find(|memory| memory.memory_type == *module);
        lines.push(described_memory_line(
            module.registry_path(),
            stored.map(|memory| &memory.value),
            stored.and_then(|memory| memory.time_to_live),
            game_time,
        ));
    }
    lines.sort();
    lines
}

fn live_memory_lines(live: &Brain, game_time: i64) -> Vec<String> {
    let mut lines = Vec::new();
    live.for_each_memory_with_ttl(|memory_type, value, ttl| {
        lines.push(described_memory_line(
            memory_type.registry_path(),
            value,
            ttl,
            game_time,
        ));
    });
    lines.sort();
    lines
}

fn described_memory_line(
    module: &str,
    value: Option<&MemoryValue>,
    ttl: Option<i64>,
    game_time: i64,
) -> String {
    match (value, ttl) {
        (None, _) => memory_line(format_args!("{module}: {EMPTY_MEMORY}")),
        (Some(MemoryValue::Long(stamp)), _) if module == "heard_bell_time" => {
            memory_line(format_args!("{module}: {} ticks ago", game_time - stamp))
        }
        (Some(value), Some(ttl)) => memory_line(format_args!("{module}: {value:?} (ttl: {ttl})")),
        (Some(value), None) => memory_line(format_args!("{module}: {value:?}")),
    }
}

fn memory_value<'a>(
    live: Option<&'a Brain>,
    packed: &'a BrainPacked,
    memory_type: MemoryModuleType,
) -> Option<&'a MemoryValue> {
    match live {
        Some(brain) => brain.memory(memory_type),
        None => packed
            .memories
            .iter()
            .find(|memory| memory.memory_type == memory_type)
            .map(|memory| &memory.value),
    }
}

struct MemoryLine {
    text: String,
    units: usize,
}

impl fmt::Write for MemoryLine {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        for (index, character) in value.char_indices() {
            let width = character.len_utf16();
            if self.units + width > MEMORY_LINE_LIMIT {
                self.text.push_str(&value[..index]);
                // Stop Debug before it traverses the rest of a long collection.
                return Err(fmt::Error);
            }
            self.units += width;
        }
        self.text.push_str(value);
        Ok(())
    }
}

/// `DebugBrainDump.java:164`: memory lines are capped at 255 UTF-16 units.
fn memory_line(arguments: fmt::Arguments<'_>) -> String {
    let mut line = MemoryLine {
        text: String::new(),
        units: 0,
    };
    if line.write_fmt(arguments).is_err() {
        while line.units > MEMORY_LINE_LIMIT - 3 {
            line.units -= line.text.pop().expect("nonempty memory prefix").len_utf16();
        }
        line.text.push_str("...");
    }
    line.text
}

/// The registry path a memory line is keyed by (`DebugBrainDump.java:189`).
fn registry_path(resource_id: &str) -> &str {
    resource_id
        .split_once(':')
        .map_or(resource_id, |(_, path)| path)
}

/// The block positions `getKnownBlockPositions` collects out of the memories it
/// is handed (`DebugBrainDump.java:122-123`, `:130-133`).
fn known_block_positions(
    shape: &BrainShape,
    live: Option<&Brain>,
    packed: &BrainPacked,
    modules: &[MemoryModuleType],
) -> Vec<BlockPos> {
    let mut positions = BTreeSet::new();
    for module in modules {
        if !shape.memory_modules.contains(module) {
            continue;
        }
        if let Some(MemoryValue::GlobalPos(global)) = memory_value(live, packed, *module) {
            positions.insert([global.pos.x, global.pos.y, global.pos.z]);
        }
    }
    positions
        .into_iter()
        .map(|[x, y, z]| BlockPos { x, y, z })
        .collect()
}

/// `Villager.getInventory().toString()` through `InventoryCarrier`
/// (`DebugBrainDump.java:109-115`), which is `SimpleContainer.toString`
/// (`SimpleContainer.java:156-158`) over `ItemStack.toString`
/// (`ItemStack.java:709-711`).
fn inventory_description(
    data: &GeneralLivingData,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> String {
    let mut description = String::new();
    if let Some(inventory) = data.internal_inventory() {
        for item in inventory.original_items().filter(|item| !item.is_empty()) {
            description.push_str(if description.is_empty() { "[" } else { ", " });
            let resource_id = item
                .item_resource_id_with_registry(content_registry)
                .unwrap_or("minecraft:air");
            write!(description, "{} {}", item.count(), resource_id).expect("String writer");
        }
    }
    if !description.is_empty() {
        description.push(']');
    }
    description
}

/// `DebugBrainDump.getVillagerGossips` (`DebugBrainDump.java:135-142`), whose
/// gossipee is named off its uuid alone and whose type prints as the enum
/// constant (`GossipType.java:6-11`).
fn gossip_lines(data: &GeneralLivingData) -> Vec<String> {
    data.villager_gossips()
        .unwrap_or(&[])
        .iter()
        .map(|gossip| {
            format!(
                "{}: {}: {}",
                gossip.target,
                gossip.kind.debug_name(),
                gossip.value
            )
        })
        .collect()
}

/// `Brain.getActiveActivities` (`Brain.java:260-263`) for a brain that has been
/// built and, where the type carries a schedule, had
/// `registerBrainGoals` run once over it (`Villager.java:218-226`).
fn active_activities(
    shape: &BrainShape,
    packed: &BrainPacked,
    scheduled: Option<&str>,
) -> Vec<String> {
    let mut chosen = DEFAULT_ACTIVITY;
    if shape.scheduled {
        let wanted = scheduled.map_or(DEFAULT_ACTIVITY, registry_path);
        if wanted != DEFAULT_ACTIVITY && activity_requirements_are_met(shape, packed, wanted) {
            chosen = wanted;
        }
    }
    vec![CORE_ACTIVITY.to_owned(), chosen.to_owned()]
}

// `Brain.activityRequirementsAreMet` (`Brain.java:434-447`): an activity the
// type never registered is refused before its conditions are read.
fn activity_requirements_are_met(shape: &BrainShape, packed: &BrainPacked, activity: &str) -> bool {
    let Some((_, condition)) = shape.activities.iter().find(|(name, _)| *name == activity) else {
        return false;
    };
    let Some(module) = condition else {
        return true;
    };
    packed
        .memories
        .iter()
        .any(|memory| memory.memory_type.registry_path() == *module)
}

/// `Villager.wantsToSpawnGolem` (`Villager.java:850-852`) over
/// `golemSpawnConditionsMet` (`:896-899`).
fn wants_golem(live: Option<&Brain>, packed: &BrainPacked, game_time: i64) -> bool {
    matches!(memory_value(live, packed, MemoryModuleType::LastSlept),
        Some(MemoryValue::Long(stamp))
        if game_time - stamp < crate::entity::villager::TIME_SINCE_SLEEPING)
        && memory_value(live, packed, MemoryModuleType::GolemDetectedRecently).is_none()
}

/// One mob's dump, or nothing where the type carries no brain bbb can state.
fn brain_dump(
    entity: &Entity,
    entities: &EntityStore,
    game_time: i64,
    schedule: impl FnOnce(bool) -> Option<String>,
) -> Option<DebugBrainDump> {
    let living = entity.living()?;
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let packed = crate::entity::brain_packed(entity)?;
    let live = entities.live_brain(entity.id());
    let baby = data.is_baby();
    let shape = brain_shape(entity.entity_type_id(), baby)?;
    // `Villager` is the only `InventoryCarrier` and the only gossip and xp
    // holder among the published types (`DebugBrainDump.java:97-124`).
    let villager = entity.entity_type_id() == VANILLA_ENTITY_TYPE_VILLAGER_ID;
    let scheduled = shape.scheduled.then(|| schedule(baby)).flatten();
    Some(DebugBrainDump {
        name: entity.uuid().to_string(),
        profession: data
            .villager_profession_and_level()
            .map_or_else(String::new, |(profession, _)| profession.to_owned()),
        xp: data.villager_work().map_or(0, |work| work.xp),
        health: living.health(),
        max_health: living.max_health(),
        inventory: if villager {
            inventory_description(data, entities.plugins().content_registry())
        } else {
            String::new()
        },
        wants_golem: villager && wants_golem(live, packed, game_time),
        // `Warden.getClientAngerLevel` (`DebugBrainDump.java:118`), which no
        // published type reaches while [`brain_shape`] names no warden.
        anger_level: -1,
        activities: match live {
            Some(live) => live
                .active_activities()
                .into_iter()
                .map(|activity| registry_path(activity.resource_id()).to_owned())
                .collect(),
            None => active_activities(&shape, packed, scheduled.as_deref()),
        },
        // `Brain.getRunningBehaviors` (`Brain.java:265-280`).
        behaviors: match live {
            Some(live) => live
                .running_behaviors()
                .into_iter()
                .map(str::to_owned)
                .collect(),
            None => Vec::new(),
        },
        memories: match live {
            Some(live) => live_memory_lines(live, game_time),
            None => memory_lines(&shape, packed, game_time),
        },
        gossips: if villager {
            gossip_lines(data)
        } else {
            Vec::new()
        },
        pois: known_block_positions(
            &shape,
            live,
            packed,
            &[
                MemoryModuleType::JobSite,
                MemoryModuleType::Home,
                MemoryModuleType::MeetingPoint,
            ],
        ),
        potential_pois: known_block_positions(
            &shape,
            live,
            packed,
            &[MemoryModuleType::PotentialJobSite],
        ),
    })
}

impl ServerWorld {
    /// Every brain the level's tracked mobs would publish this tick, keyed by
    /// network entity id.
    pub(super) fn brain_debug_dumps(&self, dimension: &LevelId) -> BTreeMap<i32, DebugBrainDump> {
        let Some(level) = self.levels.get(dimension) else {
            return BTreeMap::new();
        };
        let game_time = self.game_time();
        let entities = &level.entities;
        let mut dumps = BTreeMap::new();
        for entity in entities.bodies() {
            let id = entity.id();
            if brain_shape(entity.entity_type_id(), false).is_none() {
                continue;
            }
            let position = entity.block_position();
            let Some(dump) = brain_dump(entity, entities, game_time, |baby| {
                self.villager_schedule_activity(dimension, position, baby)
                    .map(str::to_owned)
            }) else {
                continue;
            };
            dumps.insert(id, dump);
        }
        dumps
    }
}

#[cfg(test)]
mod tests;
