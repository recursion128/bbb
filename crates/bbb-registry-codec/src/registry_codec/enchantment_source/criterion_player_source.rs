use bbb_protocol::NbtSummaryValue;

use crate::{
    persistent_component_nbt::source_codec::decode_criterion_entity_predicate_source,
    registry_codec::{recursive_values::RegistryCodecDecodeContext, RegistryCodecError},
};

use super::loot_condition_source::decode_loot_condition_source;

pub fn canonical_advancement_player_predicate_from_source(
    input: crate::RegistryElementValueRef<'_>,
    source: crate::RegistryElementSource,
) -> Result<NbtSummaryValue, RegistryCodecError> {
    let registry = "minecraft:advancement";
    let codec = "EntityPredicate.ADVANCEMENT_CODEC";
    crate::validate_registry_source_input(
        input,
        crate::PersistentNbtDecodeLimits {
            max_nodes: usize::MAX,
            ..crate::PersistentNbtDecodeLimits::DEFAULT
        },
    )?;
    let mut context = RegistryCodecDecodeContext::new(registry, codec, source);
    if let Some(len) = input.list_len() {
        context.charge_node("<root>")?;
        context.check_collection("<root>", len)?;
        let mut values = Vec::with_capacity(len);
        for index in 0..len {
            let field = format!("<root>[{index}]");
            values.push(context.with_nested(&field, |context| {
                decode_loot_condition_source(
                    input
                        .list_element(index)
                        .expect("list length is consistent"),
                    &field,
                    context,
                )
                .map(|value| value.value.canonical.into_parts().0)
            })?);
        }
        Ok(NbtSummaryValue::List(values))
    } else {
        decode_criterion_entity_predicate_source(input, "<root>", &mut context)
            .map(|value| value.into_parts().0)
    }
}

#[cfg(test)]
mod tests;
