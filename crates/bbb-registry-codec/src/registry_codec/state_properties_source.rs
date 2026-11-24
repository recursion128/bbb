use bbb_protocol::NbtSummaryValue;

use crate::persistent_component_nbt::source_codec::decode_criterion_state_properties_predicate_source;

use super::{recursive_values::RegistryCodecDecodeContext, RegistryCodecError};

pub fn canonical_state_properties_predicate_from_source(
    input: crate::RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let registry = "minecraft:advancement";
    let codec = "StatePropertiesPredicate.CODEC";
    crate::validate_registry_source_input(
        input,
        crate::PersistentNbtDecodeLimits {
            max_nodes: usize::MAX,
            ..crate::PersistentNbtDecodeLimits::DEFAULT
        },
    )?;
    let mut context = RegistryCodecDecodeContext::new(registry, codec, source);
    decode_criterion_state_properties_predicate_source(input, "<root>", &mut context)
        .map(|decoded| decoded.into_parts().0)
}

#[cfg(test)]
mod tests;
