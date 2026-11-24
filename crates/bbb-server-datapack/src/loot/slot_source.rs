//! Typed slot selections for loot entries.

use bbb_registry_codec::loot_context_types::VanillaLootContextParam;
use bbb_server_types::slot_ranges::slot_range;

use super::codec::{fail, id, int, list, string, At, Obj, Result, V};
use super::function::container_component;
use super::number::entity_target;
use super::predicate::ItemPredicate;

/// `SlotSources.CODEC` in its canonical object form.
#[derive(Debug, Clone, PartialEq)]
pub enum LootSlotSource {
    Range {
        source: VanillaLootContextParam,
        slots: &'static [i32],
    },
    Group(Vec<LootSlotSource>),
    Empty,
    Filtered {
        slot_source: Box<LootSlotSource>,
        item_filter: ItemPredicate,
    },
    Limit {
        slot_source: Box<LootSlotSource>,
        limit: usize,
    },
    Contents {
        slot_source: Box<LootSlotSource>,
        component: i32,
    },
}

impl LootSlotSource {
    pub fn decode(value: V<'_>) -> Result<Self> {
        Self::decode_with_content_registry(
            value,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    pub fn decode_with_content_registry(
        value: V<'_>,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Result<Self> {
        let obj = Obj::of(value)?;
        let nested = || {
            obj.req("slot_source", |value| {
                Self::decode_with_content_registry(value, content_registry)
            })
            .map(Box::new)
        };
        Ok(match obj.req("type", id)?.as_str() {
            "minecraft:slot_range" => Self::Range {
                source: obj.req("source", |value| {
                    if string(value)? == "block_entity" {
                        Ok(VanillaLootContextParam::BlockEntity)
                    } else {
                        entity_target(value)
                    }
                })?,
                slots: obj.req("slots", |value| {
                    let name = string(value)?;
                    slot_range(&name).map_or_else(|| fail(format!("unknown slot range {name}")), Ok)
                })?,
            },
            "minecraft:group" => Self::Group(obj.req("terms", |value| {
                list(value, |value| {
                    Self::decode_with_content_registry(value, content_registry)
                })
            })?),
            "minecraft:empty" => Self::Empty,
            "minecraft:filtered" => Self::Filtered {
                slot_source: nested()?,
                item_filter: obj.req("item_filter", |value| {
                    ItemPredicate::decode_with_content_registry(value, content_registry)
                })?,
            },
            "minecraft:limit_slots" => Self::Limit {
                slot_source: nested()?,
                limit: obj.req("limit", |value| {
                    let limit = int(value)?;
                    if limit <= 0 {
                        return fail("limit must be positive");
                    }
                    Ok(limit as usize)
                })?,
            },
            "minecraft:contents" => Self::Contents {
                slot_source: nested()?,
                component: obj.req("component", container_component)?,
            },
            other => return fail(format!("unknown slot source type {other}")).at("type"),
        })
    }
}
