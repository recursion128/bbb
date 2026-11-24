//! The dyed collar `Cat` and `Wolf` each keep as their own synched row
//! (`Cat.java:76`, `Wolf.java:89`), saved under `CollarColor`
//! (`Cat.java:214`, `:224`, `Wolf.java:231`, `:245`).
//!
//! Both declare the same `DEFAULT_COLLAR_COLOR = DyeColor.RED` (`Cat.java:81`,
//! `Wolf.java:102`) and both publish it as the dye's id through an `INT`
//! serializer (`Cat.java:188-193`, `Wolf.java:572-577`).

use bbb_protocol::builtin_enum_component_values::{
    VANILLA_DYE_COLOR_RED_ID, VANILLA_DYE_COLOR_VALUES, VANILLA_DYE_COLOR_WHITE_ID,
};
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_CAT_COLLAR_COLOR_DATA_ID, VANILLA_WOLF_COLLAR_COLOR_DATA_ID,
};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_WOLF_ID};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::living::SynchedValue;
use crate::entity::{Entity, EntityData};

/// `DyeColor.RED` (`DyeColor.java:44`), which both bodies define their row
/// with.
pub(crate) const DEFAULT_COLLAR_COLOR: i8 = VANILLA_DYE_COLOR_RED_ID as i8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CollarRetainedState {
    color: i8,
}

impl CollarRetainedState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        if collar_data_id(entity_type_id).is_some() {
            Some(Self {
                color: DEFAULT_COLLAR_COLOR,
            })
        } else {
            None
        }
    }

    pub(crate) fn from_saved(color: i8) -> Self {
        Self {
            color: normalize_color(color),
        }
    }

    pub(crate) const fn color(self) -> i8 {
        self.color
    }
}

/// The synched row each body declares for its own collar; `None` is a type
/// that keeps no collar at all.
const fn collar_data_id(entity_type_id: i32) -> Option<u8> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CAT_ID => Some(VANILLA_CAT_COLLAR_COLOR_DATA_ID),
        VANILLA_ENTITY_TYPE_WOLF_ID => Some(VANILLA_WOLF_COLLAR_COLOR_DATA_ID),
        _ => None,
    }
}

/// `DyeColor.byId` (`DyeColor.java:96-98`) over a `ByIdMap.continuous` built
/// with `OutOfBoundsStrategy.ZERO` (`DyeColor.java:48`): an id the generated
/// catalog does not name resolves to the strategy's own element, `WHITE`.
fn normalize_color(color: i8) -> i8 {
    if VANILLA_DYE_COLOR_VALUES
        .iter()
        .any(|entry| entry.id == i32::from(color))
    {
        color
    } else {
        VANILLA_DYE_COLOR_WHITE_ID as i8
    }
}

/// `Cat.getCollarColor` (`Cat.java:188-190`) and `Wolf.getCollarColor`
/// (`Wolf.java:572-574`), or nothing for a body that keeps no collar.
pub(crate) fn color_of(entity: &Entity) -> Option<i8> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.collar_retained().map(CollarRetainedState::color)
}

/// `Cat.setCollarColor` (`Cat.java:192-193`) and `Wolf.setCollarColor`
/// (`Wolf.java:576-577`), which write the synched row and nothing else.
pub(crate) fn set_collar_color(entity: &mut Entity, color: i8) -> bool {
    let color = normalize_color(color);
    let Some(data_id) = collar_data_id(entity.entity_type_id()) else {
        return false;
    };
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(state) = data.collar_retained_mut() else {
        return false;
    };
    if state.color != color {
        state.color = color;
        entity.mark_synched_dirty(data_id);
    }
    true
}

/// `output.store("CollarColor", DyeColor.LEGACY_ID_CODEC, ..)`
/// (`Cat.java:214`, `Wolf.java:231`), whose codec is `Codec.BYTE`
/// (`DyeColor.java:55`).
pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: CollarRetainedState) {
    entries.push(NbtSummaryEntry {
        name: "CollarColor".into(),
        value: NbtSummaryValue::Byte(state.color()),
    });
}

/// `input.read("CollarColor", ..).orElse(DEFAULT_COLLAR_COLOR)`
/// (`Cat.java:224`, `Wolf.java:245`).
pub(crate) fn read_save_data(
    entity_type_id: i32,
    entries: &[NbtSummaryEntry],
) -> Option<CollarRetainedState> {
    collar_data_id(entity_type_id)?;
    let color =
        crate::entity::save::optional_byte(entries, "CollarColor").unwrap_or(DEFAULT_COLLAR_COLOR);
    Some(CollarRetainedState::from_saved(color))
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    name == "CollarColor"
}

/// `entityData.define(DATA_COLLAR_COLOR, DEFAULT_COLLAR_COLOR.getId())`
/// (`Cat.java:204`, `Wolf.java:219`).
pub(crate) fn synched_values(
    entity_type_id: i32,
    state: CollarRetainedState,
) -> Option<[SynchedValue; 1]> {
    let data_id = collar_data_id(entity_type_id)?;
    Some([SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(i32::from(state.color())),
        },
        is_default: state.color() == DEFAULT_COLLAR_COLOR,
    }])
}

#[cfg(test)]
mod tests;
