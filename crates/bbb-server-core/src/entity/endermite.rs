use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMITE_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::save::optional_int;
use crate::entity::{Entity, EntityData};

const MAX_LIFE: i32 = 2400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndermiteRetainedState {
    life: i32,
}

impl EndermiteRetainedState {
    pub(crate) const fn from_saved(life: i32) -> Self {
        Self { life }
    }

    pub(crate) const fn life(self) -> i32 {
        self.life
    }

    fn tick_after_super(&mut self, persistence_required: bool) -> bool {
        if !persistence_required {
            self.life = self.life.wrapping_add(1);
        }
        self.life >= MAX_LIFE
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: EndermiteRetainedState) {
    // `Endermite.addAdditionalSaveData` (`Endermite.java:87-90`).
    entries.push(NbtSummaryEntry {
        name: "Lifetime".into(),
        value: NbtSummaryValue::Int(state.life()),
    });
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> EndermiteRetainedState {
    // `Endermite.readAdditionalSaveData` defaults a missing int to zero
    // (`Endermite.java:81-84`).
    EndermiteRetainedState::from_saved(optional_int(entries, "Lifetime").unwrap_or(0))
}

// `Endermite.tick` fixes body yaw before the inherited tick
// (`Endermite.java:93-102`).
pub(crate) fn before_super(entity: &mut Entity) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ENDERMITE_ID {
        return;
    }
    let y_rot = entity.y_rot();
    if let Some(living) = entity.living_mut() {
        living.y_body_rot = y_rot;
    }
}

// `Endermite.aiStep` advances lifetime after `super.aiStep`; persistence only
// suppresses the increment, not the threshold check (`Endermite.java:105-129`).
pub(crate) fn after_ai_step(entity: &mut Entity) {
    let should_discard = match &mut entity.data {
        EntityData::GeneralLiving(data) => {
            let persistence_required = data
                .mob_retained()
                .is_some_and(|mob| mob.persistence_required());
            data.endermite_retained_mut()
                .is_some_and(|state| state.tick_after_super(persistence_required))
        }
        _ => false,
    };
    if should_discard {
        entity.discard();
    }
}

// Endermite's `setYBodyRot` mirrors every inherited body-control result into
// entity yaw (`Endermite.java:99-102`, `Mob.java:354-356`).
pub(crate) fn after_body_rotation(entity: &mut Entity) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ENDERMITE_ID {
        return;
    }
    let Some(y_body_rot) = entity.living().map(|living| living.y_body_rot) else {
        return;
    };
    entity.set_rot(y_body_rot, entity.x_rot());
}

#[cfg(test)]
mod tests;
