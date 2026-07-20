use bbb_protocol::{
    normalize_resource_location,
    packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID,
    },
};

use super::{syntax_error, CommandSyntaxErrorKind, CommandSyntaxErrorState, ParsedValue};
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn parse(
    type_id: i32,
    command: &str,
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, CommandSyntaxErrorState>> {
    let result = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID => {
            parse_resource_or_tag(command, start, template, registry_access, true)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID => {
            parse_resource_or_tag(command, start, template, registry_access, false)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID => {
            parse_resource(command, start, template, registry_access, true)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID => {
            parse_resource(command, start, template, registry_access, false)
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID => {
            parse_resource_selector(command, start, template, registry_access)
        }
        _ => return None,
    };
    Some(result)
}

fn registry_key(
    template: &CommandArgumentTemplate,
    start: usize,
) -> Result<&str, CommandSyntaxErrorState> {
    let CommandArgumentTemplate::Registry { registry } = template else {
        return Err(invalid(start, "invalid registry argument template"));
    };
    Ok(registry)
}

fn parse_resource_or_tag(
    command: &str,
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
    resolve: bool,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let registry = registry_key(template, start)?;
    let tagged = command.as_bytes().get(start) == Some(&b'#');
    let identifier_start = start + usize::from(tagged);
    let (end, resource_id) = parse_identifier(command, identifier_start)?;
    if !resolve {
        return Ok(validated(end, true));
    }
    let Some(registry_access) = registry_access else {
        return Ok(validated(end, false));
    };
    let membership = if tagged {
        registry_access.tag_membership(registry, &resource_id)
    } else {
        registry_access.resource_membership(registry, &resource_id)
    };
    membership_result(end, membership, tagged.then_some('#'))
}

fn parse_resource(
    command: &str,
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
    resolve: bool,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let registry = registry_key(template, start)?;
    let (end, resource_id) = parse_identifier(command, start)?;
    if !resolve {
        return Ok(validated(end, true));
    }
    let Some(registry_access) = registry_access else {
        return Ok(validated(end, false));
    };
    membership_result(
        end,
        registry_access.resource_membership(registry, &resource_id),
        None,
    )
}

fn parse_resource_selector(
    command: &str,
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let registry = registry_key(template, start)?;
    let mut end = start;
    while command.as_bytes().get(end).is_some_and(|byte| {
        byte.is_ascii_digit()
            || byte.is_ascii_lowercase()
            || matches!(byte, b'_' | b':' | b'/' | b'.' | b'-' | b'*' | b'?')
    }) {
        end += 1;
    }
    if end == start {
        return Err(invalid(start, "resource selector matched no resources"));
    }
    let raw = &command[start..end];
    let pattern = if raw.contains(':') {
        raw.to_string()
    } else {
        format!("minecraft:{raw}")
    };
    let Some(resources) =
        registry_access.and_then(|access| access.resource_ids_in_holder_order(registry))
    else {
        return Ok(validated(end, false));
    };
    resources
        .iter()
        .any(|resource_id| wildcard_matches(&pattern, resource_id))
        .then(|| validated(end, true))
        .ok_or_else(|| {
            invalid(
                end,
                format!("resource selector {pattern} matched no resources"),
            )
        })
}

fn parse_identifier(
    command: &str,
    start: usize,
) -> Result<(usize, String), CommandSyntaxErrorState> {
    let mut end = start;
    while command.as_bytes().get(end).is_some_and(|byte| {
        byte.is_ascii_digit()
            || byte.is_ascii_lowercase()
            || matches!(byte, b'_' | b':' | b'/' | b'.' | b'-')
    }) {
        end += 1;
    }
    normalize_resource_location(command[start..end].to_string())
        .map(|resource_id| (end, resource_id))
        .map_err(|_| invalid(start, "invalid resource location"))
}

fn membership_result(
    end: usize,
    membership: FrozenRegistryMembership,
    prefix: Option<char>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    match membership {
        FrozenRegistryMembership::Present => Ok(validated(end, true)),
        FrozenRegistryMembership::Unavailable => Ok(validated(end, false)),
        FrozenRegistryMembership::Absent => Err(invalid(
            end,
            if prefix.is_some() {
                "unknown registry tag"
            } else {
                "unknown registry resource"
            },
        )),
    }
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let mut pattern_index = 0;
    let mut value_index = 0;
    let mut star = None;
    let mut star_value_index = 0;

    while value_index < value.len() {
        if pattern
            .get(pattern_index)
            .is_some_and(|token| *token == b'?' || *token == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern.get(pattern_index) == Some(&b'*') {
            star = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }
    while pattern.get(pattern_index) == Some(&b'*') {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn validated(end: usize, validated: bool) -> ParsedValue {
    ParsedValue { end, validated }
}

fn invalid(cursor: usize, message: impl Into<String>) -> CommandSyntaxErrorState {
    syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
    };

    use super::{parse, wildcard_matches};

    #[test]
    fn resource_selector_wildcards_match_filename_utils_shapes() {
        assert!(wildcard_matches("minecraft:*", "minecraft:stone"));
        assert!(wildcard_matches("*:stone", "minecraft:stone"));
        assert!(wildcard_matches("minecraft:st?ne", "minecraft:stone"));
        assert!(!wildcard_matches("minecraft:st?ne", "minecraft:deepslate"));
        assert!(!wildcard_matches("minecraft:*", "example:stone"));
    }

    #[test]
    fn resource_keys_are_context_free_but_resources_require_registry_validation() {
        let template = CommandArgumentTemplate::Registry {
            registry: "example:things".to_string(),
        };
        let key = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
            "example:future",
            0,
            &template,
            None,
        )
        .expect("registry-aware parser")
        .expect("resource key");
        assert_eq!(key.end, "example:future".len());
        assert!(key.validated);

        let tag_key = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
            "#example:future",
            0,
            &template,
            None,
        )
        .expect("registry-aware parser")
        .expect("resource-or-tag key");
        assert_eq!(tag_key.end, "#example:future".len());
        assert!(tag_key.validated);

        let resource = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
            "example:future",
            0,
            &template,
            None,
        )
        .expect("registry-aware parser")
        .expect("resource syntax");
        assert_eq!(resource.end, "example:future".len());
        assert!(!resource.validated);
    }
}
