use bbb_protocol::{
    command_resource_selector_matches as wildcard_matches,
    command_stat_key_for_resource_location as location_to_stat_key,
    packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID,
    },
    scan_command_resource_location_utf16 as parse_identifier, scan_command_resource_selector_utf16,
    scan_command_token_utf16,
    stat_types::{VANILLA_OBJECTIVE_CRITERIA_NAMES, VANILLA_STAT_TYPES},
};

use super::{syntax_error, CommandSyntaxErrorKind, ParsedValue, RawCommandSyntaxErrorState};
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn parse(
    type_id: i32,
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, RawCommandSyntaxErrorState>> {
    let result = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID => {
            parse_objective_criteria(command, start, registry_access)
        }
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

fn parse_objective_criteria(
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let end = scan_command_token_utf16(command, start);
    let raw = String::from_utf16_lossy(&command[start..end]);
    if VANILLA_OBJECTIVE_CRITERIA_NAMES.contains(&raw.as_str()) {
        return Ok(validated(end, true));
    }
    let Some(registry_access) = registry_access else {
        return Ok(validated(end, false));
    };
    let mut registry_unavailable = false;
    for stat_type in VANILLA_STAT_TYPES {
        let type_key = location_to_stat_key(stat_type.resource_id);
        let Some(value_key) = raw
            .strip_prefix(&type_key)
            .and_then(|suffix| suffix.strip_prefix(':'))
        else {
            continue;
        };
        let Some(values) = registry_access.resource_ids_in_holder_order(stat_type.value_registry)
        else {
            registry_unavailable = true;
            continue;
        };
        if values
            .iter()
            .any(|value| location_to_stat_key(value) == value_key)
        {
            return Ok(validated(end, true));
        }
    }
    if registry_unavailable {
        Ok(validated(end, false))
    } else {
        Err(invalid(end, format!("invalid objective criteria {raw}")))
    }
}

fn registry_key(
    template: &CommandArgumentTemplate,
    start: usize,
) -> Result<&str, RawCommandSyntaxErrorState> {
    let CommandArgumentTemplate::Registry { registry } = template else {
        return Err(invalid(start, "invalid registry argument template"));
    };
    Ok(registry)
}

fn parse_resource_or_tag(
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
    resolve: bool,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let registry = registry_key(template, start)?;
    let tagged = command.get(start) == Some(&35);
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
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
    resolve: bool,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
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
    command: &[u16],
    start: usize,
    template: &CommandArgumentTemplate,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let registry = registry_key(template, start)?;
    let (end, pattern) = scan_command_resource_selector_utf16(command, start)?;
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

fn membership_result(
    end: usize,
    membership: FrozenRegistryMembership,
    prefix: Option<char>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
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

fn validated(end: usize, validated: bool) -> ParsedValue {
    ParsedValue { end, validated }
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
    use bbb_protocol::packets::{
        CommandArgumentTemplate, VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
    };

    use super::{parse, wildcard_matches};
    use crate::RegistrySet;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn resource_selector_wildcards_match_filename_utils_shapes() {
        assert!(wildcard_matches("minecraft:*", "minecraft:stone"));
        assert!(wildcard_matches("*:stone", "minecraft:stone"));
        assert!(wildcard_matches("minecraft:st?ne", "minecraft:stone"));
        assert!(!wildcard_matches("minecraft:st?ne", "minecraft:deepslate"));
        assert!(!wildcard_matches("minecraft:*", "example:stone"));
    }

    #[test]
    fn empty_resource_selector_defers_to_registry_matching() {
        let template = CommandArgumentTemplate::Registry {
            registry: "minecraft:item".into(),
        };
        let pending = super::parse_resource_selector(&[], 0, &template, None).unwrap();
        assert_eq!(pending.end, 0);
        assert!(!pending.validated);
        let registries = RegistrySet::vanilla_26_1();
        let error =
            super::parse_resource_selector(&[], 0, &template, Some(registries.frozen_access()))
                .unwrap_err();
        assert_eq!(error.cursor, 0);
        assert_eq!(
            error.message,
            "resource selector minecraft: matched no resources"
        );
    }

    #[test]
    fn resource_keys_are_context_free_but_resources_require_registry_validation() {
        let template = CommandArgumentTemplate::Registry {
            registry: "example:things".to_string(),
        };
        let key = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
            &units("example:future"),
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
            &units("#example:future"),
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
            &units("example:future"),
            0,
            &template,
            None,
        )
        .expect("registry-aware parser")
        .expect("resource syntax");
        assert_eq!(resource.end, "example:future".len());
        assert!(!resource.validated);
    }

    #[test]
    fn objective_criteria_uses_generated_fixed_names_and_stat_registries() {
        let fixed = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
            &units("teamkill.dark_blue"),
            0,
            &CommandArgumentTemplate::Unit,
            None,
        )
        .expect("registry-aware parser")
        .expect("fixed criteria");
        assert!(fixed.validated);

        let registries = RegistrySet::vanilla_26_1();
        let dynamic = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
            &units("minecraft.mined:minecraft.oak_log"),
            0,
            &CommandArgumentTemplate::Unit,
            Some(registries.frozen_access()),
        )
        .expect("registry-aware parser")
        .expect("dynamic criteria");
        assert!(dynamic.validated);
        assert!(parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
            &units("minecraft.mined:minecraft.missing"),
            0,
            &CommandArgumentTemplate::Unit,
            Some(registries.frozen_access()),
        )
        .expect("registry-aware parser")
        .is_err());
    }
}
