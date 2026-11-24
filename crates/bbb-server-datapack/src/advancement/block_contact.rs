use bbb_protocol::{
    block_state_types::vanilla_block_has_property, block_types::vanilla_block_type_for_resource_id,
    content_registry::ContentRegistry, NbtSummaryValue,
};
use bbb_registry_codec::registry_codec::{
    canonical_advancement_player_predicate_from_source,
    canonical_state_properties_predicate_from_source,
};
use bbb_registry_codec::RegistryJsonValue;

use crate::loot::{codec::V, StatePropertiesPredicate};

use super::{decode_error, parse_resource_id, AdvancementDecodeError};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct BlockContactCondition {
    pub player: Option<NbtSummaryValue>,
    pub block: Option<String>,
    pub state: Option<StatePropertiesPredicate>,
}

/// EnterBlockTrigger.java:25-45; SlideDownBlockTrigger.java:25-45.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn parse_block_contact_condition(
    value: &RegistryJsonValue,
) -> Result<BlockContactCondition, AdvancementDecodeError> {
    parse_block_contact_condition_with_content_registry(value, ContentRegistry::vanilla())
}

pub(super) fn parse_block_contact_condition_with_content_registry(
    value: &RegistryJsonValue,
    content_registry: &ContentRegistry,
) -> Result<BlockContactCondition, AdvancementDecodeError> {
    value
        .as_object()
        .ok_or_else(|| decode_error("block contact conditions must be an object"))?;
    let field = |name| value.get(name).filter(|value| !value.is_null());
    let player = field("player")
        .map(|value| -> Result<_, AdvancementDecodeError> {
            canonical_advancement_player_predicate_from_source(
                V::Json(value),
                crate::loot::codec::source(V::Json(value)),
            )
            .map_err(|error| decode_error(error.to_string()))
        })
        .transpose()?;
    let block = field("block")
        .map(|value| {
            let block = parse_resource_id(value)?;
            if vanilla_block_type_for_resource_id(&block).is_none()
                && content_registry.block_by_resource_id(&block).is_none()
            {
                return Err(decode_error(format!("unknown block {block}")));
            }
            Ok(block)
        })
        .transpose()?;
    let state = field("state")
        .map(|value| {
            let canonical = canonical_state_properties_predicate_from_source(
                V::Json(value),
                crate::loot::codec::source(V::Json(value)),
            )
            .map_err(|error| decode_error(error.to_string()))?;
            StatePropertiesPredicate::decode(V::Nbt(&canonical))
                .map_err(|error| decode_error(error.to_string()))
        })
        .transpose()?;
    if let (Some(block), Some(properties)) = (&block, &state) {
        let mod_spec = vanilla_block_type_for_resource_id(block)
            .is_none()
            .then(|| content_registry.block_by_resource_id(block))
            .flatten();
        for (name, _) in &properties.0 {
            let has_property = name
                .with_utf8(|name| {
                    if let Some(spec) = mod_spec {
                        spec.state_count > 1
                            && name
                                == bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY
                    } else {
                        vanilla_block_has_property(block, name)
                    }
                })
                .unwrap_or(false);
            if !has_property {
                return Err(decode_error(format!(
                    "block {block} has no property {name}"
                )));
            }
        }
    }
    Ok(BlockContactCondition {
        player,
        block,
        state,
    })
}

#[cfg(test)]
mod tests;
