use bbb_protocol::{
    packets::{
        VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID, VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID,
    },
    parse_command_snbt_value_prefix,
};

use super::{
    registry_aware::parse_identifier, syntax_error, CommandSyntaxErrorKind,
    CommandSyntaxErrorState, ParsedValue,
};
use crate::{client::validate_command_dialog, FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn parse(
    type_id: i32,
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, CommandSyntaxErrorState>> {
    let registry = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID => "minecraft:loot_table",
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID => "minecraft:predicate",
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID => "minecraft:item_modifier",
        VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID => "minecraft:dialog",
        _ => return None,
    };
    Some(parse_resource_or_id(
        command,
        start,
        registry,
        type_id == VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID,
        registry_access,
    ))
}

fn parse_resource_or_id(
    command: &str,
    start: usize,
    registry: &str,
    dialog: bool,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    if let Ok((end, resource_id)) = parse_identifier(command, start) {
        if end != start {
            return match registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
                access.resource_membership(registry, &resource_id)
            }) {
                FrozenRegistryMembership::Present | FrozenRegistryMembership::Unavailable => {
                    Ok(ParsedValue {
                        end,
                        validated: true,
                    })
                }
                FrozenRegistryMembership::Absent => Err(invalid(
                    start,
                    format!("unknown {registry} resource {resource_id}"),
                )),
            };
        }
    }

    let (consumed, value) = parse_command_snbt_value_prefix(&command[start..])
        .ok_or_else(|| invalid(start, "invalid inline resource SNBT"))?;
    let Some(registry_access) = registry_access else {
        return Ok(ParsedValue {
            end: start + consumed,
            validated: true,
        });
    };
    let lookup_available = registry_access
        .resource_ids_in_holder_order(registry)
        .is_some();
    if !lookup_available {
        return Ok(ParsedValue {
            end: start + consumed,
            validated: true,
        });
    }
    if dialog {
        validate_command_dialog(&value, registry_access).map_err(|error| invalid(start, error))?;
        Ok(ParsedValue {
            end: start + consumed,
            validated: true,
        })
    } else {
        Ok(ParsedValue {
            end: start + consumed,
            validated: false,
        })
    }
}

fn invalid(cursor: usize, message: impl Into<String>) -> CommandSyntaxErrorState {
    syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RegistryPacketEntry, WorldStore};

    #[test]
    fn resource_or_id_resolves_references_and_validates_inline_dialogs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "minecraft:dialog",
            0,
            vec![RegistryPacketEntry::stub("example:notice")],
        );
        let access = Some(store.frozen_registry_access());
        assert!(
            parse_resource_or_id("example:notice", 0, "minecraft:dialog", true, access,)
                .is_ok_and(|value| value.validated)
        );
        assert!(
            parse_resource_or_id("example:missing", 0, "minecraft:dialog", true, access,).is_err()
        );
        assert!(parse_resource_or_id(
            "{type:'minecraft:notice',title:'Hello'}",
            0,
            "minecraft:dialog",
            true,
            access,
        )
        .is_ok_and(|value| value.validated));
    }

    #[test]
    fn unavailable_optional_lookup_matches_vanilla_null_parse_result() {
        let store = WorldStore::new();
        let parsed = parse_resource_or_id(
            "{pools:[]}",
            0,
            "minecraft:loot_table",
            false,
            Some(store.frozen_registry_access()),
        )
        .unwrap();
        assert!(parsed.validated);
    }
}
