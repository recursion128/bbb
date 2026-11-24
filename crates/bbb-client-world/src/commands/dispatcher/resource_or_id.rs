use bbb_protocol::{
    packets::{
        VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID, VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID,
    },
    scan_command_resource_or_id_utf16, ScannedCommandResourceOrIdUtf16,
};

use super::{syntax_error, CommandSyntaxErrorKind, ParsedValue, RawCommandSyntaxErrorState};
use crate::{client::validate_command_dialog, FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn parse(
    type_id: i32,
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, RawCommandSyntaxErrorState>> {
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
    command: &[u16],
    start: usize,
    registry: &str,
    dialog: bool,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let (end, value) = match scan_command_resource_or_id_utf16(command, start)? {
        ScannedCommandResourceOrIdUtf16::Reference { end, resource_id } => {
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
                    end,
                    format!("unknown {registry} resource {resource_id}"),
                )),
            };
        }
        ScannedCommandResourceOrIdUtf16::Inline { end, value } => (end, value),
    };

    let Some(registry_access) = registry_access else {
        return Ok(ParsedValue {
            end,
            validated: true,
        });
    };
    let lookup_available = registry_access
        .resource_ids_in_holder_order(registry)
        .is_some();
    if !lookup_available {
        return Ok(ParsedValue {
            end,
            validated: true,
        });
    }
    if dialog {
        validate_command_dialog(&value, registry_access).map_err(|error| invalid(end, error))?;
        Ok(ParsedValue {
            end,
            validated: true,
        })
    } else {
        Ok(ParsedValue {
            end,
            validated: false,
        })
    }
}

fn invalid(cursor: usize, message: impl Into<String>) -> RawCommandSyntaxErrorState {
    syntax_error(
        CommandSyntaxErrorKind::InvalidArgument,
        cursor,
        message.into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RegistryPacketEntry, WorldStore};

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn resource_or_id_resolves_references_and_validates_inline_dialogs() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "minecraft:dialog",
            0,
            vec![RegistryPacketEntry::stub("example:notice")],
        );
        let access = Some(store.frozen_registry_access());
        assert!(parse_resource_or_id(
            &units("example:notice"),
            0,
            "minecraft:dialog",
            true,
            access,
        )
        .is_ok_and(|value| value.validated));
        assert!(parse_resource_or_id(
            &units("example:missing"),
            0,
            "minecraft:dialog",
            true,
            access,
        )
        .is_err());
        assert!(parse_resource_or_id(
            &units("{type:'minecraft:notice',title:'Hello'}"),
            0,
            "minecraft:dialog",
            true,
            access,
        )
        .is_ok_and(|value| value.validated));
    }

    #[test]
    fn command_resource_or_id_errors_use_end_and_raw_values_reach_dialog_codec() {
        use bbb_protocol::{scan_command_resource_or_id_utf16, ScannedCommandResourceOrIdUtf16};
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "minecraft:dialog",
            0,
            vec![RegistryPacketEntry::stub("example:notice")],
        );
        let access = store.frozen_registry_access();
        let start = units("😀").len();
        for argument in ["example:missing", "{type:'missing'}"] {
            let command = units(&format!("😀  {argument} tail"));
            let error =
                parse_resource_or_id(&command, start, "minecraft:dialog", true, Some(access))
                    .unwrap_err();
            assert_eq!(error.cursor, units("😀  ").len() + units(argument).len());
        }
        let command: Vec<_> = "{type:'minecraft:notice',title:'"
            .encode_utf16()
            .chain([0xd800])
            .chain("'} tail".encode_utf16())
            .collect();
        let ScannedCommandResourceOrIdUtf16::Inline { end, value } =
            scan_command_resource_or_id_utf16(&command, 0).unwrap()
        else {
            panic!("inline dialog required")
        };
        assert_eq!(end, command.len() - 5);
        validate_command_dialog(&value, access).unwrap();
    }

    #[test]
    fn unavailable_optional_lookup_matches_vanilla_null_parse_result() {
        let store = WorldStore::new();
        let parsed = parse_resource_or_id(
            &units("{pools:[]}"),
            0,
            "minecraft:loot_table",
            false,
            Some(store.frozen_registry_access()),
        )
        .unwrap();
        assert!(parsed.validated);
    }
}
