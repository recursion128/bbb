//! Player-owned projectiles retained across disconnects.

use bbb_protocol::{content_registry::ContentRegistry, NbtSummaryValue};
use bbb_server_types::save::Java26LevelKey;

use crate::entity_schema::Java26EntityRecord;

use super::{field_parsers::field, field_patches::set_field};

#[derive(Debug, Clone, PartialEq)]
pub struct Java26PlayerEnderPearl {
    pub dimension: Java26LevelKey,
    pub entity: Java26EntityRecord,
}

pub(super) fn parse_with_content_registry(
    value: Option<&NbtSummaryValue>,
    content_registry: &ContentRegistry,
) -> Vec<Java26PlayerEnderPearl> {
    let Some(NbtSummaryValue::List(values)) = value else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| {
            let NbtSummaryValue::Compound(entries) = value else {
                return None;
            };
            let NbtSummaryValue::String(dimension) = field(entries, "ender_pearl_dimension")?
            else {
                return None;
            };
            Some(Java26PlayerEnderPearl {
                dimension: dimension.with_utf8(Java26LevelKey::parse)?.ok()?,
                entity: Java26EntityRecord::parse_with_content_registry(entries, content_registry)?,
            })
        })
        .collect()
}

pub(super) fn value(pearls: &[Java26PlayerEnderPearl]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        pearls
            .iter()
            .map(|pearl| {
                let mut entries = pearl.entity.as_ref().raw().to_vec();
                set_field(
                    &mut entries,
                    "ender_pearl_dimension",
                    NbtSummaryValue::string(pearl.dimension.as_str()),
                );
                NbtSummaryValue::Compound(entries)
            })
            .collect(),
    )
}
