//! `Arrow`: the `AbstractArrow` a bow or crossbow fires, carrying the potion
//! its item names.

use bbb_protocol::entity_synched_data::VANILLA_ARROW_EFFECT_COLOR_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::abstract_arrow::AbstractArrowState;
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

pub use crate::entity::abstract_arrow::{ArrowPickup, DEFAULT_DAMAGE, DEFAULT_GRAVITY};

/// `Arrow` leaves `ID_EFFECT_COLOR` at `-1` when there is no potion
/// (`Arrow.java:21`, `:60`).
pub const DEFAULT_EFFECT_COLOR: i32 = -1;

/// The `AbstractArrow` and `Arrow` fields this slice carries.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowData {
    base: AbstractArrowState,
    effect_color: i32,
}

impl std::ops::Deref for ArrowData {
    type Target = AbstractArrowState;

    fn deref(&self) -> &AbstractArrowState {
        &self.base
    }
}

impl std::ops::DerefMut for ArrowData {
    fn deref_mut(&mut self) -> &mut AbstractArrowState {
        &mut self.base
    }
}

impl ArrowData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.base.restore_command_transients(&previous.base);
    }

    /// `Arrow(level, x, y, z, item, weapon)`, whose `updateColor` reads the
    /// item's potion (`Arrow.java:29-31`).
    pub fn new(item: RuntimeItemStack) -> Self {
        Self::from_state(AbstractArrowState::new(item, "minecraft:entity.arrow.hit"))
    }

    pub(crate) fn from_state(base: AbstractArrowState) -> Self {
        let effect_color = crate::entity::thrown_potion::arrow_item_color(base.item())
            .unwrap_or(DEFAULT_EFFECT_COLOR);
        Self { base, effect_color }
    }

    pub const fn effect_color(&self) -> i32 {
        self.effect_color
    }

    pub fn set_effect_color(&mut self, effect_color: i32) {
        self.effect_color = effect_color;
    }
}

pub fn spawn_arrow(
    store: &mut EntityStore,
    position: Vec3d,
    delta_movement: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    crate::entity::abstract_arrow::spawn(
        store,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        position,
        delta_movement,
        EntityData::Arrow(ArrowData::new(item)),
    )
}

/// `AbstractArrow` and `Arrow` synched rows after the `Entity` rows.
pub fn synched_values(entity: &Entity, data: &ArrowData) -> Vec<SynchedValue> {
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
    use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
    let mut rows = crate::entity::abstract_arrow::synched_values(entity, data);
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_ARROW_EFFECT_COLOR_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(data.effect_color),
        },
        is_default: data.effect_color == DEFAULT_EFFECT_COLOR,
    });
    rows
}
