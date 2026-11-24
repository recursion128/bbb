use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_STRAY_ID};
use bbb_protocol::packets::{BlockPos, EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::conversion::SingleMobConversionSnapshot;
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData, GeneralLivingData};

const START_CONVERSION_AFTER: i32 = 140;
const TOTAL_CONVERSION_TIME: i32 = 300;
const NOT_CONVERTING: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkeletonRetainedState {
    in_powder_snow_time: i32,
    conversion_time: i32,
    converting: bool,
}

impl SkeletonRetainedState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.in_powder_snow_time = previous.in_powder_snow_time;
    }

    pub(crate) const fn new() -> Self {
        Self {
            in_powder_snow_time: 0,
            conversion_time: 0,
            converting: false,
        }
    }

    const fn from_saved(conversion_time: i32) -> Self {
        Self {
            in_powder_snow_time: 0,
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

    pub(crate) const fn converting(self) -> bool {
        self.converting
    }

    fn set_converting(&mut self, converting: bool) -> bool {
        let changed = self.converting != converting;
        self.converting = converting;
        changed
    }

    #[cfg(test)]
    fn start_for_test(&mut self, time: i32) {
        self.conversion_time = time;
        self.converting = true;
    }

    #[cfg(test)]
    fn set_powder_time_for_test(&mut self, time: i32) {
        self.in_powder_snow_time = time;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PowderSnowContact {
    Known(bool),
    Unknown,
}

fn powder_snow_contact(entity: &Entity, level: &LevelCtx<'_, '_>) -> PowderSnowContact {
    let Some(powder_snow) = vanilla_block_type_for_resource_id("minecraft:powder_snow") else {
        return PowderSnowContact::Unknown;
    };
    let bb = entity.bounding_box();
    let epsilon = f64::from(1.0e-5_f32);
    let min_x = (bb.min_x + epsilon).floor() as i32;
    let min_y = (bb.min_y + epsilon).floor() as i32;
    let min_z = (bb.min_z + epsilon).floor() as i32;
    let max_x = (bb.max_x - epsilon).ceil() as i32 - 1;
    let max_y = (bb.max_y - epsilon).ceil() as i32 - 1;
    let max_z = (bb.max_z - epsilon).ceil() as i32 - 1;
    let mut unknown = false;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let state = level.block_state(BlockPos { x, y, z });
                let Some(block_type) = vanilla_block_state_block_type_id(state) else {
                    unknown = true;
                    continue;
                };
                if block_type == powder_snow.id {
                    return PowderSnowContact::Known(true);
                }
            }
        }
    }
    if unknown {
        PowderSnowContact::Unknown
    } else {
        PowderSnowContact::Known(false)
    }
}

pub(crate) fn before_super(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_SKELETON_ID
        || !entity.is_alive()
        || entity.living().is_none_or(|living| living.health() <= 0.0)
        || matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
    {
        return;
    }
    let contact = powder_snow_contact(entity, level);
    let (dirty, convert) = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let Some(state) = data.skeleton_retained_mut() else {
                return;
            };
            match contact {
                PowderSnowContact::Unknown => (false, false),
                PowderSnowContact::Known(false) => {
                    state.in_powder_snow_time = NOT_CONVERTING;
                    (state.set_converting(false), false)
                }
                PowderSnowContact::Known(true) if state.converting => {
                    state.conversion_time = state.conversion_time.wrapping_sub(1);
                    (false, state.conversion_time < 0)
                }
                PowderSnowContact::Known(true) => {
                    state.in_powder_snow_time = state.in_powder_snow_time.wrapping_add(1);
                    if state.in_powder_snow_time >= START_CONVERSION_AFTER {
                        state.conversion_time = TOTAL_CONVERSION_TIME;
                        (state.set_converting(true), false)
                    } else {
                        (false, false)
                    }
                }
            }
        }
        _ => return,
    };
    if dirty {
        entity.mark_synched_dirty(VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID);
    }
    if convert {
        let _ = convert_to_stray(entity, store, level);
    }
}

fn convert_to_stray(
    source: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if source.removal_reason().is_some() {
        return false;
    }
    let Some(data) = GeneralLivingData::new(VANILLA_ENTITY_TYPE_STRAY_ID) else {
        return false;
    };
    let uuid = store.next_uuid();
    if store.contains_uuid(uuid) {
        return false;
    }
    let snapshot = SingleMobConversionSnapshot::capture(source);
    source
        .living_mut()
        .expect("Skeleton is living")
        .equipment_mut()
        .clear();
    if !source.is_silent() {
        level.level_event(1048, source.block_position(), 0);
    }
    let converted_id = store
        .restore_initialized(
            uuid,
            VANILLA_ENTITY_TYPE_STRAY_ID,
            snapshot.position(),
            EntityData::GeneralLiving(data),
            |target| snapshot.apply(target),
        )
        .expect("Stray conversion passed the complete restore preflight");
    crate::entity::conversion::transfer_single_relations_from(source, store, converted_id);
    source.discard();
    true
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: SkeletonRetainedState) {
    entries.push(NbtSummaryEntry {
        name: "StrayConversionTime".into(),
        value: NbtSummaryValue::Int(state.conversion_time()),
    });
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> SkeletonRetainedState {
    let conversion_time =
        crate::entity::save::optional_int(entries, "StrayConversionTime").unwrap_or(NOT_CONVERTING);
    SkeletonRetainedState::from_saved(conversion_time)
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "StrayConversionTime")
}

pub(crate) fn synched_values(state: SkeletonRetainedState) -> [SynchedValue; 1] {
    [SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_SKELETON_STRAY_CONVERSION_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(state.converting()),
        },
        is_default: !state.converting(),
    }]
}

#[cfg(test)]
mod tests;
