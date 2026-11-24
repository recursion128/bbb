use bbb_protocol::{
    packets::{
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
    },
    scan_command_item_predicate_head_utf16, scan_command_resource_location_utf16,
    CommandItemComponentPatchScannerUtf16, CommandItemPredicateTestScannerUtf16, JavaString,
    ScannedCommandItemPredicateHeadUtf16, ScannedCommandItemPredicateTestUtf16,
};

use super::{syntax_error, CommandSyntaxErrorKind, ParsedValue, RawCommandSyntaxErrorState};
use crate::{
    client::{validate_command_item_component, validate_command_item_predicate_value},
    FrozenRegistryAccess, FrozenRegistryMembership,
};

pub(super) fn parse(
    type_id: i32,
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, RawCommandSyntaxErrorState>> {
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
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let (mut cursor, item_id) = scan_command_resource_location_utf16(command, start)?;
    let item_membership = membership(registry_access, "minecraft:item", &item_id);
    if item_membership == FrozenRegistryMembership::Absent {
        return Err(invalid(start, format!("unknown item {item_id}")));
    }
    let mut validated = item_membership == FrozenRegistryMembership::Present;
    if command.get(cursor) == Some(&91) {
        let parsed = parse_component_patch(command, cursor, registry_access)?;
        cursor = parsed.end;
        validated &= parsed.validated;
    }
    Ok(ParsedValue {
        end: cursor,
        validated,
    })
}

/// Resolves each patch entry as the grammar layer hands it over, which is where
/// vanilla resolves it: a patch carrying both a rejected value and a later
/// syntax fault has to report the value.
fn parse_component_patch(
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let mut scanner = CommandItemComponentPatchScannerUtf16::new(command, start);
    let mut validated = true;
    while let Some(component) = scanner.next_component()? {
        if let Some(value) = &component.value {
            validated &= validate_component_value(
                registry_access,
                component.component.type_id,
                component.component.resource_id,
                value,
                component
                    .value_start
                    .expect("assigned component has a value cursor"),
            )?;
        }
    }
    Ok(ParsedValue {
        end: scanner.end(),
        validated,
    })
}

fn parse_item_predicate(
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let mut validated = true;
    let head = scan_command_item_predicate_head_utf16(command, start)?;
    match &head {
        ScannedCommandItemPredicateHeadUtf16::Any { .. } => {}
        ScannedCommandItemPredicateHeadUtf16::Tag {
            end: tag_end,
            resource_id,
            ..
        } => match registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
            access.tag_membership("minecraft:item", resource_id)
        }) {
            FrozenRegistryMembership::Present => {}
            FrozenRegistryMembership::Unavailable => validated = false,
            FrozenRegistryMembership::Absent => {
                return Err(invalid(*tag_end, format!("unknown item tag {resource_id}")));
            }
        },
        ScannedCommandItemPredicateHeadUtf16::Item {
            end: item_end,
            resource_id,
            ..
        } => match membership(registry_access, "minecraft:item", resource_id) {
            FrozenRegistryMembership::Present => {}
            FrozenRegistryMembership::Unavailable => validated = false,
            FrozenRegistryMembership::Absent => {
                return Err(invalid(*item_end, format!("unknown item {resource_id}")));
            }
        },
    }

    let mut scanner = CommandItemPredicateTestScannerUtf16::new(command, head.end());
    while let Some(term) = scanner.next_term()? {
        match term.test {
            ScannedCommandItemPredicateTestUtf16::Exact {
                start: _,
                value_end,
                resource_id,
                component,
                value,
            } => {
                if let Some(component) = component {
                    validated &= validate_component_value(
                        registry_access,
                        component.type_id,
                        component.resource_id,
                        &value,
                        value_end,
                    )?;
                } else if let Some(registry_access) = registry_access {
                    validate_command_item_predicate_value(&resource_id, &value, registry_access)
                        .map_err(|error| invalid(value_end, error))?;
                } else {
                    validated = false;
                }
            }
            ScannedCommandItemPredicateTestUtf16::Subpredicate {
                start: _,
                value_end,
                resource_id,
                value,
            } => {
                if let Some(registry_access) = registry_access {
                    validate_command_item_predicate_value(&resource_id, &value, registry_access)
                        .map_err(|error| invalid(value_end, error))?;
                } else {
                    validated = false;
                }
            }
            ScannedCommandItemPredicateTestUtf16::Presence { .. } => {}
        }
    }
    Ok(ParsedValue {
        end: scanner.end(),
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
) -> Result<bool, RawCommandSyntaxErrorState> {
    let Some(registry_access) = registry_access else {
        return Ok(false);
    };
    validate_command_item_component(type_id, resource_id, value, registry_access)
        .map_err(|error| invalid(cursor, error))
}

fn invalid(cursor: usize, message: impl Into<JavaString>) -> RawCommandSyntaxErrorState {
    syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistrySet;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn message(error: &RawCommandSyntaxErrorState) -> String {
        String::from_utf16_lossy(error.message.units())
    }

    #[test]
    fn item_stack_resolves_items_components_and_values() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        let parsed = parse_item_stack(
            &units("minecraft:stone[minecraft:max_stack_size=16,!minecraft:custom_name]"),
            0,
            access,
        )
        .unwrap();
        assert!(parsed.validated);
        assert!(parse_item_stack(&units("missing"), 0, access).is_err());
        assert!(parse_item_stack(&units("stone[max_stack_size=0]"), 0, access).is_err());
        assert!(parse_item_stack(
            &units("stone[max_stack_size=16,max_stack_size=8]"),
            0,
            access
        )
        .is_err());
    }

    #[test]
    fn item_component_value_error_precedes_separator_error_and_points_to_value() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        let expected = parse_item_stack(&units("stone[max_stack_size=0]"), 0, access).unwrap_err();
        assert_eq!(expected.cursor, "stone[max_stack_size=".len());
        for suffix in ["", " !custom_name]", ",missing=1]", " garbage"] {
            let command = format!("stone[max_stack_size=0{suffix}");
            assert_eq!(
                parse_item_stack(&units(&command), 0, access).unwrap_err(),
                expected
            );
        }
        let command = "stone[max_stack_size=16 !custom_name]";
        let error = parse_item_stack(&units(command), 0, access).unwrap_err();
        assert!(message(&error).contains("separator"));
        assert_eq!(error.cursor, command.find('!').unwrap());
        assert!(
            parse_item_stack(&units("stone[max_stack_size=16,]"), 0, access)
                .unwrap()
                .validated
        );
    }

    #[test]
    fn raw_item_component_input_reaches_codec_without_projection() {
        let registries = RegistrySet::vanilla_26_1();
        let input: Vec<u16> = [0xd800]
            .into_iter()
            .chain("[custom_data={\"".encode_utf16())
            .chain([0xd800])
            .chain("\\uDC00\":'".encode_utf16())
            .chain([0xdc00])
            .chain("'},!custom_name]".encode_utf16())
            .chain([32, 0xd800])
            .collect();
        let mut scanner = bbb_protocol::CommandItemComponentPatchScannerUtf16::new(&input, 1);
        let assigned = scanner.next_component().unwrap().unwrap();
        assert_eq!(assigned.start, 2);
        assert_eq!(assigned.value_start, Some(14));
        let value = assigned.value.unwrap();
        let bbb_protocol::NbtSummaryValue::Compound(entries) = &value else {
            panic!("compound required")
        };
        assert_eq!(entries[0].name, "𐀀");
        assert_eq!(
            entries[0].value,
            bbb_protocol::NbtSummaryValue::string_from_utf16(vec![0xdc00])
        );
        assert!(validate_component_value(
            Some(registries.frozen_access()),
            assigned.component.type_id,
            assigned.component.resource_id,
            &value,
            assigned.value_start.unwrap()
        )
        .unwrap());
        let removed = scanner.next_component().unwrap().unwrap();
        assert!(removed.removed);
        assert_eq!(removed.value_start, None);
        assert!(scanner.next_component().unwrap().is_none());
        assert_eq!(scanner.end(), input.len() - 2);
    }

    #[test]
    fn item_predicate_whitespace_and_codec_error_order_match_grammar() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        for command in [
            "* [ ! custom_name | damage = 1 , count ~ {min:1} ]",
            "* [ ]",
            "\u{2000}stone [ damage = 1 ]",
        ] {
            assert!(
                parse_item_predicate(&units(command), 0, access)
                    .unwrap()
                    .validated
            );
        }
        let prefix = "*[max_stack_size=0";
        let expected = parse_item_predicate(&units(&format!("{prefix}]")), 0, access).unwrap_err();
        assert_eq!(expected.cursor, prefix.len());
        for suffix in ["", " !custom_name]", "|missing]", ",missing]"] {
            assert_eq!(
                parse_item_predicate(&units(&format!("{prefix}{suffix}")), 0, access).unwrap_err(),
                expected
            );
        }
        for command in [
            "*[damage=1,]",
            "*[damage=1|]",
            "*[!!damage]",
            "*[damage=1 custom_name]",
        ] {
            assert!(parse_item_predicate(&units(command), 0, access).is_err());
        }
    }

    #[test]
    fn raw_item_predicate_value_reaches_component_codec() {
        let registries = RegistrySet::vanilla_26_1();
        let input: Vec<u16> = "* [custom_data = {value:'"
            .encode_utf16()
            .chain([0xd800])
            .chain("'} | ! custom_name ]".encode_utf16())
            .chain([32, 0xdc00])
            .collect();
        let head = scan_command_item_predicate_head_utf16(&input, 0).unwrap();
        let mut scanner = CommandItemPredicateTestScannerUtf16::new(&input, head.end());
        let term = scanner.next_term().unwrap().unwrap();
        assert!(!term.negated && !term.alternative);
        let ScannedCommandItemPredicateTestUtf16::Exact {
            component: Some(component),
            value,
            value_end,
            ..
        } = term.test
        else {
            panic!("exact component required")
        };
        let bbb_protocol::NbtSummaryValue::Compound(entries) = &value else {
            panic!("compound required")
        };
        assert_eq!(
            entries[0].value,
            bbb_protocol::NbtSummaryValue::string_from_utf16(vec![0xd800])
        );
        assert_eq!(input[value_end - 1], 125);
        assert!(validate_component_value(
            Some(registries.frozen_access()),
            component.type_id,
            component.resource_id,
            &value,
            value_end
        )
        .unwrap());
        let negated = scanner.next_term().unwrap().unwrap();
        assert!(negated.negated && negated.alternative);
        assert!(matches!(
            negated.test,
            ScannedCommandItemPredicateTestUtf16::Presence { .. }
        ));
        assert!(scanner.next_term().unwrap().is_none());
        assert_eq!(scanner.end(), input.len() - 2);
    }

    #[test]
    fn item_predicate_lookup_errors_point_after_identifier() {
        let mut registries = RegistrySet::vanilla_26_1();
        registries.tags.insert(
            "minecraft:item".to_string(),
            crate::RegistryTagState::from_materialized(
                "minecraft:item".to_string(),
                std::collections::BTreeMap::new(),
            ),
        );
        let access = Some(registries.frozen_access());
        for identifier in ["missing", "#missing", "#  missing"] {
            let command = format!("😀 {identifier}   [damage=1]");
            let error =
                parse_item_predicate(&units(&command), "😀 ".encode_utf16().count(), access)
                    .unwrap_err();
            assert_eq!(
                error.cursor,
                "😀 ".encode_utf16().count() + identifier.len()
            );
        }
        for term in ["missing", "missing = 1", "missing ~ {}"] {
            let command = format!("😀 *[  {term}]");
            let error =
                parse_item_predicate(&units(&command), "😀 ".encode_utf16().count(), access)
                    .unwrap_err();
            assert_eq!(error.cursor, "😀 *[  missing".encode_utf16().count());
        }
        let error = parse_item_stack(&units("missing[damage=1]"), 0, access).unwrap_err();
        assert_eq!(error.cursor, 0);
        let error = parse_item_stack(&units("stone[missing=1]"), 0, access).unwrap_err();
        assert_eq!(error.cursor, "stone[".len());
    }

    #[test]
    fn item_predicate_transient_component_existence_uses_unit_codec() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        for name in [
            "creative_slot_lock",
            "additional_trade_cost",
            "map_post_processing",
        ] {
            for value in ["{}", "{ignored:1}"] {
                let command = format!("*[{name}~{value}]");
                let parsed = parse(
                    VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
                    &units(&command),
                    0,
                    access,
                )
                .unwrap()
                .unwrap();
                assert!(parsed.validated);
                assert_eq!(parsed.end, command.len());
            }
            for term in [
                name.to_string(),
                format!("{name}={{}}"),
                format!("{name}~1"),
            ] {
                assert!(
                    parse_item_predicate(&units(&format!("*[{term}]")), 0, access).is_err(),
                    "{term}"
                );
            }
            assert!(parse_item_stack(&units(&format!("stone[{name}={{}}]")), 0, access).is_err());
            assert!(parse_item_stack(&units(&format!("stone[!{name}]")), 0, access).is_err());
        }
        assert!(parse_item_predicate(&units("*[missing~{}]"), 0, access).is_err());
    }

    #[test]
    fn item_predicate_validates_exact_component_and_predicate_codecs() {
        let mut registries = RegistrySet::vanilla_26_1();
        registries.tags.insert(
            "minecraft:item".to_string(),
            crate::RegistryTagState::from_materialized(
                "minecraft:item".to_string(),
                std::collections::BTreeMap::from([("minecraft:logs".to_string(), vec![1])]),
            ),
        );
        let access = Some(registries.frozen_access());
        let component = parse_item_predicate(&units("#logs[max_stack_size=16]"), 0, access);
        assert!(
            component.as_ref().is_ok_and(|value| value.validated),
            "{component:?}"
        );
        assert!(parse_item_predicate(&units("*[count~{min:1}]"), 0, access)
            .is_ok_and(|value| value.validated));
        assert!(
            parse_item_predicate(&units("*[damage~{damage:{min:1}}]"), 0, access)
                .is_ok_and(|value| value.validated)
        );
        assert!(parse_item_predicate(&units("*[count~{min:'bad'}]"), 0, access).is_err());
        assert!(parse_item_predicate(&units("*[missing]"), 0, access).is_err());
    }
}
