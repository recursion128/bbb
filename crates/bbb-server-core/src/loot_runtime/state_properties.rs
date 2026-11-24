use super::{unsupported_field, LootRuntimeReason};
use bbb_protocol::block_state_types::{
    vanilla_block_property_kind, vanilla_block_property_value_rank, VanillaBlockStateIdentity,
};
use bbb_server_datapack::loot::{PropertyMatcher, StatePropertiesPredicate};

/// `StatePropertiesPredicate.matches`: a property the state lacks fails before
/// its matcher; values compare by their parsed rank, not their spelling; an
/// unparsable bound fails.
pub fn matches_state_properties(
    predicate: &StatePropertiesPredicate,
    state: &VanillaBlockStateIdentity,
) -> Result<bool, LootRuntimeReason> {
    for (name, matcher) in &predicate.0 {
        let matched = name
            .with_utf8(|name| -> Result<bool, LootRuntimeReason> {
                let Some(actual) = state.properties.get(name) else {
                    return Ok(false);
                };
                let kind = vanilla_block_property_kind(&state.name, name).ok_or_else(|| {
                    unsupported_field(
                        "state properties predicate",
                        format!("missing type for {}.{name}", state.name),
                    )
                })?;
                let actual = vanilla_block_property_value_rank(kind, actual).ok_or_else(|| {
                    unsupported_field(
                        "state properties predicate",
                        format!("invalid actual {}.{name}", state.name),
                    )
                })?;
                Ok(matches_property_value(matcher, actual, |value| {
                    vanilla_block_property_value_rank(kind, value)
                }))
            })
            .unwrap_or(Ok(false))?;
        if !matched {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn matches_property_value(
    matcher: &PropertyMatcher,
    actual: i32,
    rank: impl Fn(&str) -> Option<i32>,
) -> bool {
    match matcher {
        PropertyMatcher::Exact(value) => value.with_utf8(&rank).flatten() == Some(actual),
        PropertyMatcher::Ranged { min, max } => {
            let bound = |value: &Option<bbb_protocol::JavaString>, inside: fn(i32, i32) -> bool| {
                match value {
                    None => true,
                    Some(value) => value
                        .with_utf8(&rank)
                        .flatten()
                        .is_some_and(|bound| inside(actual, bound)),
                }
            };
            bound(min, |actual, min| actual >= min) && bound(max, |actual, max| actual <= max)
        }
    }
}

#[cfg(test)]
mod tests;
