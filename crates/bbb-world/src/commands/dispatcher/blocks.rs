use std::collections::BTreeMap;

use bbb_protocol::{
    packets::{
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
    },
    parse_command_snbt_prefix, CommandSnbtKind,
};

use super::{
    read_string_value, registry_aware::parse_identifier, syntax_error, CommandSyntaxErrorKind,
    CommandSyntaxErrorState, ParsedValue,
};
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn parse(
    type_id: i32,
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, CommandSyntaxErrorState>> {
    let allow_tag = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID => false,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID => true,
        _ => return None,
    };
    Some(parse_block(command, start, allow_tag, registry_access))
}

fn parse_block(
    command: &str,
    start: usize,
    allow_tag: bool,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let tagged = command.as_bytes().get(start) == Some(&b'#');
    if tagged && !allow_tag {
        return Err(invalid(start, "block-state argument does not allow tags"));
    }
    let identifier_start = start + usize::from(tagged);
    let (mut cursor, resource_id) = parse_identifier(command, identifier_start)?;
    let membership = registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
        if tagged {
            access.tag_membership("minecraft:block", &resource_id)
        } else {
            access.resource_membership("minecraft:block", &resource_id)
        }
    });
    if membership == FrozenRegistryMembership::Absent {
        return Err(invalid(
            identifier_start,
            if tagged {
                "unknown block tag"
            } else {
                "unknown block"
            },
        ));
    }

    let properties = if command.as_bytes().get(cursor) == Some(&b'[') {
        let (end, properties) = parse_properties(command, cursor)?;
        cursor = end;
        properties
    } else {
        BTreeMap::new()
    };
    if command.as_bytes().get(cursor) == Some(&b'{') {
        let consumed = parse_command_snbt_prefix(&command[cursor..], CommandSnbtKind::Compound)
            .ok_or_else(|| invalid(cursor, "invalid block entity SNBT"))?;
        cursor += consumed;
    }

    let properties_membership = if tagged || membership == FrozenRegistryMembership::Unavailable {
        membership
    } else {
        registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
            access.block_properties_membership(&resource_id, &properties)
        })
    };
    match properties_membership {
        FrozenRegistryMembership::Present => Ok(validated(cursor, true)),
        FrozenRegistryMembership::Unavailable => Ok(validated(cursor, false)),
        FrozenRegistryMembership::Absent => Err(invalid(cursor, "invalid block properties")),
    }
}

fn parse_properties(
    command: &str,
    start: usize,
) -> Result<(usize, BTreeMap<String, String>), CommandSyntaxErrorState> {
    let mut cursor = start + 1;
    let mut properties = BTreeMap::new();
    skip_whitespace(command, &mut cursor);
    while command.as_bytes().get(cursor) != Some(&b']') {
        if cursor >= command.len() {
            return Err(invalid(cursor, "unclosed block properties"));
        }
        let key_start = cursor;
        let (end, key) = read_string_value(command, cursor)?;
        if end == cursor {
            return Err(invalid(cursor, "expected block property name"));
        }
        cursor = end;
        skip_whitespace(command, &mut cursor);
        if command.as_bytes().get(cursor) != Some(&b'=') {
            return Err(invalid(cursor, "expected block property value"));
        }
        cursor += 1;
        skip_whitespace(command, &mut cursor);
        let (end, value) = read_string_value(command, cursor)?;
        if end == cursor {
            return Err(invalid(cursor, "expected block property value"));
        }
        cursor = end;
        if properties.insert(key.clone(), value).is_some() {
            return Err(invalid(
                key_start,
                format!("duplicate block property {key}"),
            ));
        }
        skip_whitespace(command, &mut cursor);
        match command.as_bytes().get(cursor) {
            Some(b',') => {
                cursor += 1;
                skip_whitespace(command, &mut cursor);
            }
            Some(b']') => {}
            _ => return Err(invalid(cursor, "expected block property separator or end")),
        }
    }
    Ok((cursor + 1, properties))
}

fn skip_whitespace(command: &str, cursor: &mut usize) {
    while command
        .as_bytes()
        .get(*cursor)
        .is_some_and(u8::is_ascii_whitespace)
    {
        *cursor += 1;
    }
}

fn validated(end: usize, validated: bool) -> ParsedValue {
    ParsedValue { end, validated }
}

fn invalid(cursor: usize, message: impl Into<String>) -> CommandSyntaxErrorState {
    syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistrySet;

    #[test]
    fn block_state_validates_exact_properties_and_snbt() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        let valid = parse_block("minecraft:oak_log[axis=x]{Custom:1b}", 0, false, access).unwrap();
        assert_eq!(valid.end, "minecraft:oak_log[axis=x]{Custom:1b}".len());
        assert!(valid.validated);

        assert!(parse_block("oak_log[axis=diagonal]", 0, false, access).is_err());
        assert!(parse_block("oak_log[axis=x,axis=y]", 0, false, access).is_err());
        assert!(parse_block("#logs", 0, false, access).is_err());
    }

    #[test]
    fn block_predicate_resolves_tags_but_keeps_vague_properties() {
        let mut registries = RegistrySet::vanilla_26_1();
        registries.tags.insert(
            "minecraft:block".to_string(),
            crate::RegistryTagState {
                registry: "minecraft:block".to_string(),
                tags: BTreeMap::from([("minecraft:logs".to_string(), vec![0])]),
            },
        );
        let parsed = parse_block(
            "#logs[server_property=server_value]{Custom:1b}",
            0,
            true,
            Some(registries.frozen_access()),
        )
        .unwrap();
        assert!(parsed.validated);
        assert!(parse_block(
            "#missing[server_property=server_value]",
            0,
            true,
            Some(registries.frozen_access()),
        )
        .is_err());
    }
}
