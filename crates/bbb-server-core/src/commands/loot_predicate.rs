//! Typed resource-or-inline loot predicates for command arguments.

use bbb_protocol::{
    scan_command_resource_or_id_utf16, NbtSummaryValue, ScannedCommandResourceOrIdUtf16,
};
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::{parse_loot_condition_nbt, LootCondition};

use super::{parse::ParseFailure, translatable};
use crate::world::ServerWorld;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandLootPredicate {
    pub id: String,
    pub condition: LootCondition,
}

pub(super) fn read(
    command: &[u16],
    cursor: usize,
    world: &ServerWorld,
) -> Result<(usize, CommandLootPredicate), ParseFailure> {
    let scanned = scan_command_resource_or_id_utf16(command, cursor)
        .map_err(|_| failed_to_parse(cursor, "expected an id or an inline SNBT value"))?;
    match scanned {
        ScannedCommandResourceOrIdUtf16::Reference { end, resource_id } => {
            let condition = world
                .datapacks()
                .and_then(|startup| startup.loot.predicate(&resource_id))
                .cloned()
                .ok_or_else(|| ParseFailure {
                    message: translatable(
                        "argument.resource_or_id.no_such_element",
                        vec![
                            NbtSummaryValue::string(resource_id.clone()),
                            NbtSummaryValue::string("minecraft:predicate"),
                        ],
                    ),
                    cursor: Some(cursor),
                })?;
            Ok((
                end,
                CommandLootPredicate {
                    id: resource_id,
                    condition,
                },
            ))
        }
        ScannedCommandResourceOrIdUtf16::Inline { end, value } => {
            let lookup = world
                .datapacks()
                .map(FrozenDatapackRegistryLookup::from_startup);
            let condition = parse_loot_condition_nbt(&value, lookup.as_ref())
                .map_err(|error| failed_to_parse(cursor, error.to_string()))?;
            Ok((
                end,
                CommandLootPredicate {
                    id: String::new(),
                    condition,
                },
            ))
        }
    }
}

fn failed_to_parse(cursor: usize, message: impl Into<String>) -> ParseFailure {
    ParseFailure {
        message: translatable(
            "argument.resource_or_id.failed_to_parse",
            vec![NbtSummaryValue::String(message.into().into())],
        ),
        cursor: Some(cursor),
    }
}
