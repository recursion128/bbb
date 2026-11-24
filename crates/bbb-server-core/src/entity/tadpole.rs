use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_TADPOLE_AGE_LOCKED_DATA_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::living::SynchedValue;
use crate::entity::save::{optional_bool, optional_int};
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore};

/// `Tadpole.ticksToBeFrog` (`Tadpole.java:48`).
const TICKS_TO_BE_FROG: i32 = 24_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TadpoleRetainedState {
    age: i32,
    age_locked: bool,
    age_lock_particle_timer: i32,
}

impl TadpoleRetainedState {
    pub(in crate::entity) const fn from_saved(age: i32, age_locked: bool) -> Self {
        Self {
            age,
            age_locked,
            age_lock_particle_timer: 0,
        }
    }

    pub(crate) const fn age(self) -> i32 {
        self.age
    }

    pub(crate) const fn age_locked(self) -> bool {
        self.age_locked
    }

    pub(crate) const fn age_lock_particle_timer(self) -> i32 {
        self.age_lock_particle_timer
    }

    /// `Tadpole.feed`'s `ageUp` (`Tadpole.java:212-228`); true once it is a frog.
    pub(crate) fn feed(&mut self) -> bool {
        let ticks_left = TICKS_TO_BE_FROG.saturating_sub(self.age).max(0);
        let seconds = crate::entity::animal_feeding::speed_up_seconds_when_feeding(ticks_left);
        self.age = self.age.wrapping_add(seconds.wrapping_mul(20));
        self.age >= TICKS_TO_BE_FROG
    }

    /// `Tadpole.setAgeLockedData` (`Tadpole.java:167-171`); answers the new lock.
    pub(crate) fn toggle_age_lock(&mut self) -> bool {
        self.age_locked = !self.age_locked;
        self.age = 0;
        self.age_lock_particle_timer = 40;
        self.age_locked
    }

    /// `Tadpole.aiStep` (`Tadpole.java:101-108`); true once it is a frog.
    fn tick(&mut self) -> bool {
        if self.age_lock_particle_timer > 0 {
            self.age_lock_particle_timer -= 1;
        }
        if self.age_locked {
            return false;
        }
        self.age = self.age.wrapping_add(1);
        self.age >= TICKS_TO_BE_FROG
    }
}

/// The server half of `Tadpole.aiStep`: a grown tadpole asks the world for
/// its frog (`Tadpole.java:230-246`).
pub(crate) fn after_ai_step(entity: &mut Entity, store: &mut EntityStore) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data
        .tadpole_retained_mut()
        .is_some_and(|state| state.tick())
    {
        store.queue_command(EntityCommand::TadpoleGrowUp {
            entity_id: entity.id(),
        });
    }
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "Age" | "AgeLocked")
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: TadpoleRetainedState) {
    // `Tadpole` writes and restores its state after AbstractFish (`Tadpole.java:111-121`).
    entries.extend([
        int_entry("Age", state.age),
        bool_entry("AgeLocked", state.age_locked),
    ]);
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> TadpoleRetainedState {
    TadpoleRetainedState::from_saved(
        optional_int(entries, "Age").unwrap_or(0),
        optional_bool(entries, "AgeLocked").unwrap_or(false),
    )
}

pub(crate) fn synched_values(state: TadpoleRetainedState) -> [SynchedValue; 1] {
    // `Tadpole.defineSynchedData` appends the lock after AbstractFish (`Tadpole.java:125-135`).
    [SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_TADPOLE_AGE_LOCKED_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(state.age_locked),
        },
        is_default: !state.age_locked,
    }]
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
#[path = "tadpole/tests.rs"]
mod tests;
