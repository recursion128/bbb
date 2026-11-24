use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::level::NearbyPlayer;
use crate::entity::save::{optional_bool, optional_int};
use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkeletonHorseTrapState {
    is_trap: bool,
    trap_time: i32,
}

impl SkeletonHorseTrapState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        if entity_type_id == VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID {
            Some(Self {
                is_trap: false,
                trap_time: 0,
            })
        } else {
            None
        }
    }

    /// `SkeletonHorse.setTrap(true)` (`SkeletonHorse.java:156-165`) as
    /// `ServerLevel.tickThunder` applies it beside `setAge(0)`, which is
    /// already this type's spawn default (`ServerLevel.java:544-545`).
    pub(crate) const fn trap() -> Self {
        Self {
            is_trap: true,
            trap_time: 0,
        }
    }

    const fn from_saved(is_trap: bool, trap_time: i32) -> Self {
        Self { is_trap, trap_time }
    }
}

pub(crate) fn set_state(entity: &mut Entity, state: SkeletonHorseTrapState) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(current) = data.horse_state_mut().and_then(|horse| horse.trap.as_mut()) else {
        return false;
    };
    if *current == state {
        return false;
    }
    *current = state;
    true
}

pub(crate) fn is_trap(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data)
        if data.skeleton_horse_trap().is_some_and(|state| state.is_trap))
}

// SkeletonTrapGoal.java:28; EntityGetter.java:100-108.
pub(crate) fn has_trigger_player(position: Vec3d, players: &[NearbyPlayer]) -> bool {
    players.iter().any(|player| {
        let dx = player.position.x - position.x;
        let dy = player.position.y - position.y;
        let dz = player.position.z - position.z;
        !player.spectator && !player.dead_or_dying && dx * dx + dy * dy + dz * dz < 100.0
    })
}

// SkeletonTrapGoal.java:35-37.
pub(crate) fn activate_trap(entity: &mut Entity) -> bool {
    if !is_trap(entity) {
        return false;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    data.horse_state_mut()
        .and_then(|horse| horse.trap.as_mut())
        .expect("classified skeleton horse trap")
        .is_trap = false;
    tame_and_grow(entity)
}

pub(crate) fn tame_and_grow(entity: &mut Entity) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(horse) = data.abstract_horse_retained_mut() else {
        return false;
    };
    if horse.set_tame(true) {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID);
    }
    crate::entity::set_spawner_age(entity, 0);
    true
}

// SkeletonHorse.java:126-130.
pub(crate) fn after_ai_step(entity: &mut Entity) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data
        .mob_retained()
        .is_some_and(|mob| mob.persistence_required())
    {
        return;
    }
    let Some(state) = data.horse_state_mut().and_then(|horse| horse.trap.as_mut()) else {
        return;
    };
    if !state.is_trap {
        return;
    }
    let expired = state.trap_time >= 18_000;
    state.trap_time = state.trap_time.wrapping_add(1);
    if expired {
        entity.discard();
    }
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "SkeletonTrap" | "SkeletonTrapTime")
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: SkeletonHorseTrapState) {
    // Concrete writer appends after AbstractHorse (`SkeletonHorse.java:134-137`).
    entries.extend([
        bool_entry("SkeletonTrap", state.is_trap),
        int_entry("SkeletonTrapTime", state.trap_time),
    ]);
}

pub(crate) fn read_save_data(
    entity_type_id: i32,
    entries: &[NbtSummaryEntry],
) -> Option<SkeletonHorseTrapState> {
    if entity_type_id != VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID {
        return None;
    }
    // `getBooleanOr` / `getIntOr` (`SkeletonHorse.java:141-144`).
    Some(SkeletonHorseTrapState::from_saved(
        optional_bool(entries, "SkeletonTrap").unwrap_or(false),
        optional_int(entries, "SkeletonTrapTime").unwrap_or(0),
    ))
}

fn int_entry(name: &str, value: i32) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Int(value),
    }
}

fn bool_entry(name: &str, value: bool) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Byte(i8::from(value)),
    }
}

#[cfg(test)]
#[path = "skeleton_horse/tests.rs"]
mod tests;
