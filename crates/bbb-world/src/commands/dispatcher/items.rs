use std::collections::BTreeSet;

use bbb_protocol::{
    data_component_types::{
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_PREDICATE_TYPES,
    },
    packets::{
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
    },
    parse_command_snbt_value_prefix,
};

use super::{
    registry_aware::parse_identifier, syntax_error, CommandSyntaxErrorKind,
    CommandSyntaxErrorState, ParsedValue,
};
use crate::{
    client::{validate_command_item_component, validate_command_item_predicate_value},
    FrozenRegistryAccess, FrozenRegistryMembership,
};

pub(super) fn parse(
    type_id: i32,
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, CommandSyntaxErrorState>> {
    let result = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID => {
            parse_item_stack(command, start, registry_access)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID => {
            parse_item_predicate(command, start, registry_access)
        }
        _ => return None,
    };
    Some(result)
}

fn parse_item_stack(
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (mut cursor, item_id) = parse_identifier(command, start)?;
    let item_membership = membership(registry_access, "minecraft:item", &item_id);
    if item_membership == FrozenRegistryMembership::Absent {
        return Err(invalid(start, format!("unknown item {item_id}")));
    }
    let mut validated = item_membership == FrozenRegistryMembership::Present;
    if command.as_bytes().get(cursor) == Some(&b'[') {
        let parsed = parse_component_patch(command, cursor, registry_access)?;
        cursor = parsed.end;
        validated &= parsed.validated;
    }
    Ok(ParsedValue {
        end: cursor,
        validated,
    })
}

fn parse_component_patch(
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let mut cursor = start + 1;
    let mut visited = BTreeSet::new();
    let mut validated = true;
    skip_whitespace(command, &mut cursor);
    while command.as_bytes().get(cursor) != Some(&b']') {
        if cursor >= command.len() {
            return Err(invalid(cursor, "unclosed item component patch"));
        }
        let removed = command.as_bytes().get(cursor) == Some(&b'!');
        if removed {
            cursor += 1;
        }
        let component_start = cursor;
        let (end, component_id) = parse_identifier(command, cursor)?;
        cursor = end;
        let component = persistent_component(&component_id).ok_or_else(|| {
            invalid(
                component_start,
                format!("unknown item component {component_id}"),
            )
        })?;
        if !visited.insert(component.type_id) {
            return Err(invalid(
                component_start,
                format!("duplicate item component {component_id}"),
            ));
        }
        skip_whitespace(command, &mut cursor);
        if !removed {
            if command.as_bytes().get(cursor) != Some(&b'=') {
                return Err(invalid(cursor, "expected item component assignment"));
            }
            cursor += 1;
            skip_whitespace(command, &mut cursor);
            let (consumed, value) = parse_command_snbt_value_prefix(&command[cursor..])
                .ok_or_else(|| invalid(cursor, "invalid item component SNBT"))?;
            cursor += consumed;
            validated &= validate_component_value(
                registry_access,
                component.type_id,
                component.resource_id,
                &value,
                component_start,
            )?;
        }
        skip_whitespace(command, &mut cursor);
        match command.as_bytes().get(cursor) {
            Some(b',') => {
                cursor += 1;
                skip_whitespace(command, &mut cursor);
            }
            Some(b']') => {}
            _ => return Err(invalid(cursor, "expected item component separator or end")),
        }
    }
    Ok(ParsedValue {
        end: cursor + 1,
        validated,
    })
}

fn parse_item_predicate(
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let mut cursor = start;
    let mut validated = true;
    match command.as_bytes().get(cursor) {
        Some(b'*') => cursor += 1,
        Some(b'#') => {
            let tag_start = cursor;
            cursor += 1;
            let (end, tag) = parse_identifier(command, cursor)?;
            cursor = end;
            match registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
                access.tag_membership("minecraft:item", &tag)
            }) {
                FrozenRegistryMembership::Present => {}
                FrozenRegistryMembership::Unavailable => validated = false,
                FrozenRegistryMembership::Absent => {
                    return Err(invalid(tag_start, format!("unknown item tag {tag}")));
                }
            }
        }
        _ => {
            let item_start = cursor;
            let (end, item) = parse_identifier(command, cursor)?;
            cursor = end;
            match membership(registry_access, "minecraft:item", &item) {
                FrozenRegistryMembership::Present => {}
                FrozenRegistryMembership::Unavailable => validated = false,
                FrozenRegistryMembership::Absent => {
                    return Err(invalid(item_start, format!("unknown item {item}")));
                }
            }
        }
    }
    if command.as_bytes().get(cursor) != Some(&b'[') {
        return Ok(ParsedValue {
            end: cursor,
            validated,
        });
    }

    cursor += 1;
    if command.as_bytes().get(cursor) == Some(&b']') {
        return Ok(ParsedValue {
            end: cursor + 1,
            validated,
        });
    }
    loop {
        if command.as_bytes().get(cursor) == Some(&b'!') {
            cursor += 1;
        }
        let test_start = cursor;
        let (end, test_id) = parse_identifier(command, cursor)?;
        cursor = end;
        match command.as_bytes().get(cursor) {
            Some(b'=') => {
                let component = predicate_component(&test_id).ok_or_else(|| {
                    invalid(test_start, format!("unknown item component {test_id}"))
                })?;
                cursor += 1;
                let (consumed, value) = parse_command_snbt_value_prefix(&command[cursor..])
                    .ok_or_else(|| invalid(cursor, "invalid item component predicate SNBT"))?;
                cursor += consumed;
                if let Some(component) = component {
                    validated &= validate_component_value(
                        registry_access,
                        component.type_id,
                        component.resource_id,
                        &value,
                        test_start,
                    )?;
                } else if let Some(registry_access) = registry_access {
                    validate_command_item_predicate_value(&test_id, &value, registry_access)
                        .map_err(|error| invalid(test_start, error))?;
                } else {
                    validated = false;
                }
            }
            Some(b'~') => {
                if !known_predicate(&test_id) {
                    return Err(invalid(
                        test_start,
                        format!("unknown item predicate {test_id}"),
                    ));
                }
                cursor += 1;
                let (consumed, value) = parse_command_snbt_value_prefix(&command[cursor..])
                    .ok_or_else(|| invalid(cursor, "invalid item predicate SNBT"))?;
                cursor += consumed;
                if let Some(registry_access) = registry_access {
                    validate_command_item_predicate_value(&test_id, &value, registry_access)
                        .map_err(|error| invalid(test_start, error))?;
                } else {
                    validated = false;
                }
            }
            _ => {
                if predicate_component(&test_id).is_none() {
                    return Err(invalid(
                        test_start,
                        format!("unknown item component {test_id}"),
                    ));
                }
            }
        }
        match command.as_bytes().get(cursor) {
            Some(b'|') => cursor += 1,
            Some(b',') => cursor += 1,
            Some(b']') => {
                cursor += 1;
                break;
            }
            _ => return Err(invalid(cursor, "expected item predicate separator or end")),
        }
    }
    Ok(ParsedValue {
        end: cursor,
        validated,
    })
}

fn membership(
    registry_access: Option<FrozenRegistryAccess<'_>>,
    registry: &str,
    resource_id: &str,
) -> FrozenRegistryMembership {
    registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
        access.resource_membership(registry, resource_id)
    })
}

fn validate_component_value(
    registry_access: Option<FrozenRegistryAccess<'_>>,
    type_id: i32,
    resource_id: &str,
    value: &bbb_protocol::NbtSummaryValue,
    cursor: usize,
) -> Result<bool, CommandSyntaxErrorState> {
    let Some(registry_access) = registry_access else {
        return Ok(false);
    };
    validate_command_item_component(type_id, resource_id, value, registry_access)
        .map_err(|error| invalid(cursor, error))
}

fn persistent_component(
    resource_id: &str,
) -> Option<&'static bbb_protocol::data_component_types::DataComponentTypeCatalogEntry> {
    vanilla_data_component_type_for_resource_id(resource_id).filter(|entry| entry.persistent)
}

fn predicate_component(
    resource_id: &str,
) -> Option<Option<&'static bbb_protocol::data_component_types::DataComponentTypeCatalogEntry>> {
    if resource_id == "minecraft:count" {
        Some(None)
    } else {
        persistent_component(resource_id).map(Some)
    }
}

fn known_predicate(resource_id: &str) -> bool {
    resource_id == "minecraft:count"
        || VANILLA_DATA_COMPONENT_PREDICATE_TYPES
            .iter()
            .any(|entry| entry.resource_id == resource_id)
        || persistent_component(resource_id).is_some()
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

fn invalid(cursor: usize, message: impl Into<String>) -> CommandSyntaxErrorState {
    syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistrySet;

    #[test]
    fn item_stack_resolves_items_components_and_values() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        let parsed = parse_item_stack(
            "minecraft:stone[minecraft:max_stack_size=16,!minecraft:custom_name]",
            0,
            access,
        )
        .unwrap();
        assert!(parsed.validated);
        assert!(parse_item_stack("missing", 0, access).is_err());
        assert!(parse_item_stack("stone[max_stack_size=0]", 0, access).is_err());
        assert!(parse_item_stack("stone[max_stack_size=16,max_stack_size=8]", 0, access).is_err());
    }

    #[test]
    fn item_predicate_validates_exact_component_and_predicate_codecs() {
        let mut registries = RegistrySet::vanilla_26_1();
        registries.tags.insert(
            "minecraft:item".to_string(),
            crate::RegistryTagState {
                registry: "minecraft:item".to_string(),
                tags: std::collections::BTreeMap::from([("minecraft:logs".to_string(), vec![1])]),
            },
        );
        let access = Some(registries.frozen_access());
        let component = parse_item_predicate("#logs[max_stack_size=16]", 0, access);
        assert!(
            component.as_ref().is_ok_and(|value| value.validated),
            "{component:?}"
        );
        assert!(
            parse_item_predicate("*[count~{min:1}]", 0, access).is_ok_and(|value| value.validated)
        );
        assert!(
            parse_item_predicate("*[damage~{damage:{min:1}}]", 0, access)
                .is_ok_and(|value| value.validated)
        );
        assert!(parse_item_predicate("*[count~{min:'bad'}]", 0, access).is_err());
        assert!(parse_item_predicate("*[missing]", 0, access).is_err());
    }
}
