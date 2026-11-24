//! Typed reads of synchronized entity data. A missing value, or one carried by
//! another serializer, reads as the caller's fallback.

use bbb_protocol::packets::{EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind};

#[inline]
pub(crate) fn entity_data_value(
    data_values: &[EntityDataValue],
    data_id: u8,
) -> Option<&EntityDataValueKind> {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .map(|value| &value.value)
}

macro_rules! scalar_readers {
    ($($visibility:vis $name:ident: $ty:ty = $variant:ident;)*) => {$(
        #[inline]
        $visibility fn $name(data_values: &[EntityDataValue], data_id: u8, fallback: $ty) -> $ty {
            match entity_data_value(data_values, data_id) {
                Some(EntityDataValueKind::$variant(value)) => *value,
                _ => fallback,
            }
        }
    )*};
}

scalar_readers! {
    pub entity_data_bool: bool = Boolean;
    pub entity_data_byte: i8 = Byte;
    pub entity_data_int: i32 = Int;
    pub entity_data_long: i64 = Long;
    pub entity_data_float: f32 = Float;
    pub(crate) entity_data_humanoid_arm: i32 = HumanoidArm;
    pub(crate) entity_data_optional_unsigned_int: Option<i32> = OptionalUnsignedInt;
}

/// Reads an `EntityDataSerializers` enum accessor (e.g. `ARMADILLO_STATE`), which
/// is wire-encoded as the enum's id.
pub(crate) fn entity_data_enum_id(
    data_values: &[EntityDataValue],
    data_id: u8,
    serializer: EntityDataEnumSerializer,
    fallback: i32,
) -> i32 {
    match entity_data_value(data_values, data_id) {
        Some(EntityDataValueKind::EnumId {
            serializer: value_serializer,
            id,
        }) if *value_serializer == serializer => *id,
        _ => fallback,
    }
}
