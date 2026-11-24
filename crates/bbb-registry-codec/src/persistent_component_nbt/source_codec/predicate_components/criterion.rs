//! Source-aware advancement criterion predicate codecs.

mod damage_source;
mod entity;
mod location;

use crate::registry_codec::{
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};

pub(crate) use entity::decode as decode_entity_predicate;
pub(crate) use location::decode as decode_location_predicate;

pub(crate) fn decode_damage_source_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    damage_source::decode(input, field, context, |input, field, context| {
        entity::decode(input, field, context)
    })
}
