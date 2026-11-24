//! The `block_ticks` / `fluid_ticks` lists a chunk document carries.

use super::Java26ChunkContext;
use crate::schema_support::{field, int_value};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::registry_data_types::VanillaFluidIdentity;
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::chunk_ticks::Java26SavedTick;
use bbb_server_types::save::NamedCompound;

#[derive(Debug, Clone, Copy)]
pub enum TickKind {
    Block,
    Fluid,
}
/// Writes the save tick and the block and fluid delays packed relative to it.
pub fn write_chunk_ticks(
    raw: &mut NamedCompound,
    game_time: i64,
    block_ticks: &[Java26SavedTick],
    fluid_ticks: &[Java26SavedTick],
) {
    let list = |ticks: &[Java26SavedTick]| {
        NbtSummaryValue::List(ticks.iter().map(saved_tick_value).collect())
    };
    for (name, value) in [
        ("LastUpdate", NbtSummaryValue::Long(game_time)),
        ("block_ticks", list(block_ticks)),
        ("fluid_ticks", list(fluid_ticks)),
    ] {
        match raw
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.name == name)
        {
            Some(entry) => entry.value = value,
            None => raw.entries_mut().push(NbtSummaryEntry {
                name: name.into(),
                value,
            }),
        }
    }
}

/// `i`/`x`/`y`/`z`/`t`/`p` shape `SavedTick.codec` reads back
/// (`SavedTick.java:28-44`).
pub(crate) fn saved_tick_value(tick: &Java26SavedTick) -> NbtSummaryValue {
    let field = |name: &str, value: NbtSummaryValue| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    NbtSummaryValue::Compound(vec![
        field("i", NbtSummaryValue::string(tick.id.clone())),
        field("x", NbtSummaryValue::Int(tick.x)),
        field("y", NbtSummaryValue::Int(tick.y)),
        field("z", NbtSummaryValue::Int(tick.z)),
        field("t", NbtSummaryValue::Int(tick.delay)),
        field("p", NbtSummaryValue::Int(tick.priority)),
    ])
}

/// `SavedTick.codec` lists read with `resultOrPartial`: an entry that does
/// not decode is dropped, and chunk lists keep only their own positions
/// (`SavedTick.filterTickListForChunk`).
pub fn parse_ticks(
    value: &NbtSummaryValue,
    context: Java26ChunkContext,
    filter_to_chunk: bool,
    kind: TickKind,
) -> Vec<Java26SavedTick> {
    parse_ticks_with_content_registry(
        value,
        context,
        filter_to_chunk,
        kind,
        ContentRegistry::vanilla(),
    )
}

/// Registry-aware form of [`parse_ticks`].
pub fn parse_ticks_with_content_registry(
    value: &NbtSummaryValue,
    context: Java26ChunkContext,
    filter_to_chunk: bool,
    kind: TickKind,
    content_registry: &ContentRegistry,
) -> Vec<Java26SavedTick> {
    let NbtSummaryValue::List(values) = value else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| {
            let NbtSummaryValue::Compound(entries) = value else {
                return None;
            };
            let NbtSummaryValue::String(id) = field(entries, "i")? else {
                return None;
            };
            let id = id
                .try_to_string()
                .ok()
                .and_then(|raw| normalize_resource_location(raw).ok())?;
            let known = match kind {
                TickKind::Block => {
                    vanilla_block_type_for_resource_id(&id).is_some()
                        || content_registry.block_by_resource_id(&id).is_some()
                }
                TickKind::Fluid => VanillaFluidIdentity::for_resource_id(&id).is_some(),
            };
            let int = |name| field(entries, name).and_then(int_value);
            let tick = Java26SavedTick {
                id,
                x: int("x")?,
                y: int("y")?,
                z: int("z")?,
                delay: int("t")?,
                // TickPriority.byValue clamps out-of-range values.
                priority: int("p")?.clamp(-3, 3),
            };
            let in_chunk = tick.x.div_euclid(16) == context.expected_x
                && tick.z.div_euclid(16) == context.expected_z;
            (known && (in_chunk || !filter_to_chunk)).then_some(tick)
        })
        .collect()
}
