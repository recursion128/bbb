//! `SpectralArrow`: an `AbstractArrow` that makes what it hits glow.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::abstract_arrow::AbstractArrowState;
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// `SpectralArrow.DEFAULT_DURATION` (`SpectralArrow.java:17`).
pub const DEFAULT_DURATION: i32 = 200;

/// The `AbstractArrow` and `SpectralArrow` fields this slice carries.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralArrowData {
    base: AbstractArrowState,
    duration: i32,
}

impl std::ops::Deref for SpectralArrowData {
    type Target = AbstractArrowState;

    fn deref(&self) -> &AbstractArrowState {
        &self.base
    }
}

impl std::ops::DerefMut for SpectralArrowData {
    fn deref_mut(&mut self) -> &mut AbstractArrowState {
        &mut self.base
    }
}

impl SpectralArrowData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.base.restore_command_transients(&previous.base);
    }

    pub fn new(item: RuntimeItemStack) -> Self {
        Self::from_state(
            AbstractArrowState::new(item, "minecraft:entity.arrow.hit"),
            DEFAULT_DURATION,
        )
    }

    pub(crate) fn from_state(base: AbstractArrowState, duration: i32) -> Self {
        Self { base, duration }
    }

    pub const fn duration(&self) -> i32 {
        self.duration
    }
}

pub fn spawn_spectral_arrow(
    store: &mut EntityStore,
    position: Vec3d,
    delta_movement: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    crate::entity::abstract_arrow::spawn(
        store,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
        position,
        delta_movement,
        EntityData::SpectralArrow(SpectralArrowData::new(item)),
    )
}

/// `AbstractArrow` synched rows after the `Entity` rows.
pub fn synched_values(entity: &Entity, data: &SpectralArrowData) -> Vec<SynchedValue> {
    crate::entity::abstract_arrow::synched_values(entity, data)
}
