//! `ResourceOrIdArgument.lootTable` (`ResourceOrIdArgument.java:87-92`, `:178-181`).

use bbb_protocol::{
    scan_command_resource_or_id_utf16, NbtSummaryValue, ScannedCommandResourceOrIdUtf16,
};
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::{parse_loot_table_nbt, LootElementValue, LootTable};

use super::parse::ParseFailure;
use super::translatable;
use crate::world::ServerWorld;

/// The registry `ResourceOrIdArgument.LootTableArgument` names
/// (`ResourceOrIdArgument.java:178-181`).
pub(super) const LOOT_TABLE_REGISTRY: &str = "minecraft:loot_table";

/// A loot table the `/loot … loot` source already resolved.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandLootTable {
    pub id: String,
    pub table: LootTable,
}

/// `ResourceOrIdArgument.parse` (`ResourceOrIdArgument.java:123-132`) for
/// `Registries.LOOT_TABLE`: the decode and nothing else.
///
/// A table is checked against a parameter set once at load, against the set the
/// table itself declares (`ValidationContext.java:68-74`); the set a command
/// supplies is never checked here, and `LootCommand.drop` (`LootCommand.java:492-498`)
/// resolves the `give` players only after the roll (`:231-239`). bbb's roll
/// still refuses a table naming a parameter the supplied set lacks, in
/// `bbb-server-block`'s loot runtime rather than at this parse.
pub(super) fn read_loot_table(
    command: &[u16],
    cursor: usize,
    world: &ServerWorld,
) -> Result<(usize, CommandLootTable), ParseFailure> {
    let scanned = scan_command_resource_or_id_utf16(command, cursor)
        .map_err(|_| failed_to_parse(cursor, "expected an id or an inline SNBT value"))?;
    match scanned {
        ScannedCommandResourceOrIdUtf16::Reference { end, resource_id } => {
            let found =
                world
                    .loot_elements("minecraft:loot_table")
                    .iter()
                    .find_map(
                        |element| match (&element.value, element.id == resource_id) {
                            (LootElementValue::Table(table), true) => Some(table.clone()),
                            _ => None,
                        },
                    );
            let Some(table) = found else {
                return Err(ParseFailure {
                    message: translatable(
                        "argument.resource_or_id.no_such_element",
                        vec![
                            NbtSummaryValue::string(resource_id),
                            NbtSummaryValue::string(LOOT_TABLE_REGISTRY),
                        ],
                    ),
                    cursor: Some(cursor),
                });
            };
            Ok((
                end,
                CommandLootTable {
                    id: resource_id,
                    table,
                },
            ))
        }
        ScannedCommandResourceOrIdUtf16::Inline { end, value } => {
            let lookup = world
                .datapacks()
                .map(FrozenDatapackRegistryLookup::from_startup);
            let table = parse_loot_table_nbt(&value, lookup.as_ref())
                .map_err(|error| failed_to_parse(cursor, error.to_string()))?;
            Ok((
                end,
                CommandLootTable {
                    id: String::new(),
                    table,
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
