use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID, VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID,
};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ZOMBIE_BABY_DATA_ID, VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_HUSK_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::entity::attributes::{AttributeModifier, AttributeOperation};
use crate::entity::conversion::SingleMobConversionSnapshot;
use crate::entity::damage::StoredEntityDamageSource;
use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_bool, optional_int};
use crate::entity::{
    Entity, EntityCommand, EntityData, EntityStore, GeneralLivingData, StoredDamageCausingEntity,
};

const START_CONVERSION_AFTER: i32 = 600;
const TOTAL_CONVERSION_TIME: i32 = 300;
const NOT_CONVERTING: i32 = -1;
const BABY_SPEED_MODIFIER_ID: &str = "minecraft:baby";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ZombieRetainedState {
    is_baby: bool,
    can_break_doors: bool,
    in_water_time: i32,
    conversion_time: i32,
    converting: bool,
}

impl ZombieRetainedState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        if is_zombie_hierarchy(entity_type_id) {
            Some(Self {
                is_baby: false,
                can_break_doors: false,
                in_water_time: 0,
                conversion_time: 0,
                converting: false,
            })
        } else {
            None
        }
    }

    const fn from_saved(
        is_baby: bool,
        can_break_doors: bool,
        in_water_time: i32,
        conversion_time: i32,
    ) -> Self {
        Self {
            is_baby,
            can_break_doors,
            in_water_time,
            conversion_time,
            converting: conversion_time != NOT_CONVERTING,
        }
    }

    pub(crate) const fn conversion_time(self) -> i32 {
        if self.converting {
            self.conversion_time
        } else {
            NOT_CONVERTING
        }
    }

    pub(crate) const fn in_water_time(self) -> i32 {
        self.in_water_time
    }

    pub(crate) const fn converting(self) -> bool {
        self.converting
    }

    pub(crate) const fn is_baby(self) -> bool {
        self.is_baby
    }

    pub(crate) const fn can_break_doors(self) -> bool {
        self.can_break_doors
    }

    fn set_baby(&mut self, baby: bool) -> bool {
        let changed = self.is_baby != baby;
        self.is_baby = baby;
        changed
    }

    fn set_can_break_doors(&mut self, can_break_doors: bool) {
        self.can_break_doors = can_break_doors;
    }

    fn start_conversion(&mut self) -> bool {
        self.conversion_time = TOTAL_CONVERSION_TIME;
        let changed = !self.converting;
        self.converting = true;
        changed
    }

    #[cfg(test)]
    fn set_in_water_time_for_test(&mut self, time: i32) {
        self.in_water_time = time;
    }

    #[cfg(test)]
    fn set_conversion_time_for_test(&mut self, time: i32) {
        self.conversion_time = time;
        self.converting = true;
    }
}

pub(crate) const fn is_zombie_hierarchy(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID
            | VANILLA_ENTITY_TYPE_HUSK_ID
            | VANILLA_ENTITY_TYPE_DROWNED_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
    )
}

fn apply_baby_speed_modifier(data: &mut GeneralLivingData, baby: bool) {
    let speed = data
        .living_mut()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .expect("Zombie hierarchy declares movement speed");
    speed.remove_modifier(BABY_SPEED_MODIFIER_ID);
    if baby {
        speed
            .add_transient_modifier(AttributeModifier {
                id: BABY_SPEED_MODIFIER_ID.to_string(),
                amount: 0.5,
                operation: AttributeOperation::AddMultipliedBase,
            })
            .expect("the baby modifier was just removed");
    }
}

pub(crate) fn set_baby(entity: &mut Entity, baby: bool) -> bool {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let Some(state) = data.zombie_retained_mut() else {
                return false;
            };

            state.set_baby(baby)
        }
        _ => return false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_ZOMBIE_BABY_DATA_ID);
        crate::entity::living::refresh_dimensions(entity);
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        apply_baby_speed_modifier(data, baby);
    }
    true
}

pub(crate) fn apply_conversion_phenotype(
    entity: &mut Entity,
    baby: bool,
    can_break_doors: bool,
) -> bool {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let Some(state) = data.zombie_retained_mut() else {
                return false;
            };
            let changed = state.set_baby(baby);
            state.set_can_break_doors(can_break_doors);
            changed
        }
        _ => return false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_ZOMBIE_BABY_DATA_ID);
        crate::entity::living::refresh_dimensions(entity);
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        apply_baby_speed_modifier(data, baby);
    }
    true
}

pub(crate) fn before_super(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    villager::tick_cure(entity, store, level);
    let (can_start, target_type_id, level_event) = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_ZOMBIE_ID => (true, VANILLA_ENTITY_TYPE_DROWNED_ID, 1040),
        VANILLA_ENTITY_TYPE_HUSK_ID => (true, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 1041),
        VANILLA_ENTITY_TYPE_DROWNED_ID
        | VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
        | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => (false, VANILLA_ENTITY_TYPE_DROWNED_ID, 1040),
        _ => return,
    };
    if !entity.is_alive()
        || entity.living().is_none_or(|living| living.health() <= 0.0)
        || matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
    {
        return;
    }
    let converting = matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.zombie_retained().is_some_and(ZombieRetainedState::converting)
    );
    let eye_in_water = if converting {
        None
    } else if !can_start {
        return;
    } else {
        Some(entity.is_eye_in_fluid(crate::entity::environment::FluidTag::Water))
    };
    let (dirty, convert) = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let Some(state) = data.zombie_retained_mut() else {
                return;
            };
            if state.converting {
                state.conversion_time = state.conversion_time.wrapping_sub(1);
                (false, state.conversion_time < 0)
            } else {
                match eye_in_water {
                    None => (false, false),
                    Some(false) => {
                        state.in_water_time = NOT_CONVERTING;
                        (false, false)
                    }
                    Some(true) => {
                        state.in_water_time = state.in_water_time.wrapping_add(1);
                        if state.in_water_time >= START_CONVERSION_AFTER {
                            (state.start_conversion(), false)
                        } else {
                            (false, false)
                        }
                    }
                }
            }
        }
        _ => return,
    };
    if dirty {
        entity.mark_synched_dirty(VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID);
    }
    if convert {
        let _ = convert_zombie(entity, store, level, target_type_id, level_event);
    }
}

fn convert_zombie(
    source: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_type_id: i32,
    event_id: i32,
) -> bool {
    if source.removal_reason().is_some() {
        return false;
    }
    let Some(data) = GeneralLivingData::new(target_type_id) else {
        return false;
    };
    let uuid = store.next_uuid();
    if store.contains_uuid(uuid) {
        return false;
    }
    let snapshot = SingleMobConversionSnapshot::capture(source);
    let special = level
        .special_multiplier_at(source.block_position())
        .unwrap_or(0.0);
    let random = store.random().clone();
    source
        .living_mut()
        .expect("Zombie hierarchy is living")
        .equipment_mut()
        .clear();
    let converted_id = store
        .restore_initialized(
            uuid,
            target_type_id,
            snapshot.position(),
            EntityData::GeneralLiving(data),
            |target| {
                snapshot.apply(target);
                // `Zombie.convertToZombieType` (`Zombie.java:246-252`).
                let _ = crate::entity::general_living::zombie_handle_attributes(
                    target, &random, special, true, true,
                );
            },
        )
        .expect("Zombie conversion passed the complete restore preflight");
    crate::entity::conversion::transfer_single_relations_from(source, store, converted_id);
    source.discard();
    if !source.is_silent() {
        level.level_event(event_id, source.block_position(), 0);
    }
    true
}

pub(crate) fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    state: ZombieRetainedState,
    is_in_water: bool,
) {
    entries.extend([
        NbtSummaryEntry {
            name: "IsBaby".into(),
            value: NbtSummaryValue::Byte(i8::from(state.is_baby())),
        },
        NbtSummaryEntry {
            name: "CanBreakDoors".into(),
            value: NbtSummaryValue::Byte(i8::from(state.can_break_doors())),
        },
        NbtSummaryEntry {
            name: "InWaterTime".into(),
            value: NbtSummaryValue::Int(if is_in_water {
                state.in_water_time()
            } else {
                NOT_CONVERTING
            }),
        },
        NbtSummaryEntry {
            name: "DrownedConversionTime".into(),
            value: NbtSummaryValue::Int(state.conversion_time()),
        },
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> ZombieRetainedState {
    let is_baby = optional_bool(entries, "IsBaby").unwrap_or(false);
    let can_break_doors = optional_bool(entries, "CanBreakDoors").unwrap_or(false);
    let in_water_time = optional_int(entries, "InWaterTime").unwrap_or(0);
    let conversion_time = optional_int(entries, "DrownedConversionTime").unwrap_or(-1);
    ZombieRetainedState::from_saved(is_baby, can_break_doors, in_water_time, conversion_time)
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(
        name,
        "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
    )
}

pub(crate) fn synched_values(state: ZombieRetainedState) -> [SynchedValue; 2] {
    [
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_ZOMBIE_BABY_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(state.is_baby()),
            },
            is_default: !state.is_baby(),
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(state.converting()),
            },
            is_default: !state.converting(),
        },
    ]
}

/// The level work a zombie hands the world: each needs a spawn placement,
/// `finalizeSpawn` or a player the entity store cannot reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingZombieAction {
    /// `Zombie.hurtServer`'s reinforcement search (`Zombie.java:294-329`).
    Reinforce {
        caller_id: i32,
        target_id: i32,
        target_uuid: Uuid,
    },
    /// `Zombie.convertVillagerToZombieVillager` (`Zombie.java:254-272`).
    InfectVillager {
        villager_id: i32,
        zombie_silent: bool,
        zombie_pos: BlockPos,
    },
    /// `ZombieVillager.finishConversion` (`ZombieVillager.java:235-275`).
    Cure { zombie_villager_id: i32 },
    /// What `Zombie.doHurtTarget` (`Zombie.java:336-346`) and
    /// `Husk.doHurtTarget` (`Husk.java:65-73`) add to a landed hit by the
    /// local difficulty at `pos`.
    HitTarget {
        source_entity_id: i32,
        target_id: i32,
        player: Option<Uuid>,
        pos: BlockPos,
        burning: bool,
        husk: bool,
    },
}

/// `Zombie.hurtServer` after a hurt that took (`Zombie.java:280-292`);
/// `isSpawningMonsters` and the placement search run in the world.
pub(crate) fn call_reinforcements(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
) {
    if !is_zombie_hierarchy(entity.entity_type_id()) || store.difficulty() != Some(Difficulty::Hard)
    {
        return;
    }
    let target = store
        .mob_target(entity.id())
        .and_then(|uuid| Some((store.entity_id_for_uuid(uuid)?, uuid)))
        .or_else(|| match source.causing_entity? {
            StoredDamageCausingEntity::Player {
                entity_id, uuid, ..
            } => Some((entity_id, uuid)),
            causing => {
                let id = causing.entity_id();
                store
                    .get(id)
                    .or_else(|| level.detached_entity(id))
                    .filter(|body| body.living().is_some())
                    .map(|body| (id, body.uuid()))
            }
        });
    let Some((target_id, target_uuid)) = target else {
        return;
    };
    let chance = entity.living().map_or(0.0, |living| {
        living.attribute_value(VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID)
    });
    if f64::from(store.random().next_float()) < chance {
        store.queue_command(EntityCommand::Zombie(PendingZombieAction::Reinforce {
            caller_id: entity.id(),
            target_id,
            target_uuid,
        }));
    }
}

/// `Zombie.killedEntity` (`Zombie.java:420-433`): `false` when the villager
/// becomes a zombie villager instead of perishing.
pub(crate) fn killed_entity(
    store: &mut EntityStore,
    killer_type_id: i32,
    killer_silent: bool,
    killer_pos: BlockPos,
    victim: &Entity,
) -> bool {
    if !is_zombie_hierarchy(killer_type_id)
        || victim.entity_type_id() != VANILLA_ENTITY_TYPE_VILLAGER_ID
    {
        return true;
    }
    match store.difficulty() {
        Some(Difficulty::Hard) => {}
        Some(Difficulty::Normal) if !store.random().next_boolean() => {}
        _ => return true,
    }
    store.queue_command(EntityCommand::Zombie(PendingZombieAction::InfectVillager {
        villager_id: victim.id(),
        zombie_silent: killer_silent,
        zombie_pos: killer_pos,
    }));
    false
}

pub(crate) mod villager;

#[cfg(test)]
mod tests;
