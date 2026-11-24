#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use bbb_protocol::{
    block_state_types::{
        vanilla_block_property_kind, vanilla_block_property_value_rank_utf16,
        VanillaBlockPropertyKind,
    },
    packets::{
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
    },
    scan_command_block_body_utf16, scan_command_block_identifier_utf16, JavaString,
};

use super::{syntax_error, CommandSyntaxErrorKind, ParsedValue, RawCommandSyntaxErrorState};
use crate::{FrozenRegistryAccess, FrozenRegistryMembership};

pub(super) fn parse(
    type_id: i32,
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, RawCommandSyntaxErrorState>> {
    let allow_tag = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID => false,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID => true,
        _ => return None,
    };
    Some(parse_block(command, start, allow_tag, registry_access))
}

/// Resolves what the grammar layer scanned. The two scans are kept apart so the
/// block name is resolved before the property list is read, which is where
/// vanilla reports an unknown block.
fn parse_block(
    command: &[u16],
    start: usize,
    allow_tag: bool,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let identifier = scan_command_block_identifier_utf16(command, start, allow_tag)?;
    let membership = registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
        if identifier.tagged {
            access.tag_membership("minecraft:block", &identifier.resource_id)
        } else {
            access.resource_membership("minecraft:block", &identifier.resource_id)
        }
    });
    if membership == FrozenRegistryMembership::Absent {
        return Err(invalid(
            start,
            if identifier.tagged {
                "unknown block tag"
            } else {
                "unknown block"
            },
        ));
    }

    let property_membership = if identifier.tagged {
        membership
    } else {
        registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
            access.block_properties_membership(&identifier.resource_id, &Default::default())
        })
    };
    let body =
        scan_command_block_body_utf16(command, identifier.end, identifier.tagged, |key, value| {
            if identifier.tagged || property_membership != FrozenRegistryMembership::Present {
                return Ok(());
            }
            let key = String::from_utf16_lossy(key.units());
            let kind = vanilla_block_property_kind(&identifier.resource_id, &key)
                .ok_or_else(|| format!("unknown block property {key}"))?;
            if let Some(value) = value {
                if vanilla_block_property_value_rank_utf16(kind, value.units()).is_none() {
                    let value = String::from_utf16_lossy(value.units());
                    return Err(format!("invalid block property value {value}").into());
                }
            }
            Ok(())
        })?;
    Ok(validated(
        body.end,
        property_membership == FrozenRegistryMembership::Present,
    ))
}

/// `Property.getValue` for one definition value. A definition lists the
/// spellings it accepts, and one whose values are the integers of a single
/// range goes through `Integer.parseInt` inside that range
/// (`IntegerProperty.java:52-59`), which reads `Character.digit`'s decimal
/// digits -- every spelling of a listed value, `00`, `+0`, `-0` and the Unicode
/// decimal-digit runs included -- and nothing outside the range.
pub(super) fn property_accepts_value(values: &BTreeSet<&str>, value: &[u16]) -> bool {
    if values
        .iter()
        .any(|candidate| candidate.encode_utf16().eq(value.iter().copied()))
    {
        return true;
    }
    let Some((min, max)) = integer_range(values) else {
        return false;
    };
    vanilla_block_property_value_rank_utf16(&VanillaBlockPropertyKind::Integer { min, max }, value)
        .is_some()
}

/// Tag completion combines value sets, so integer bounds come from the union.
fn integer_range(values: &BTreeSet<&str>) -> Option<(i32, i32)> {
    let mut range = None::<(i32, i32)>;
    for candidate in values {
        let parsed = candidate.parse::<i32>().ok()?;
        range = Some(match range {
            Some((min, max)) => (min.min(parsed), max.max(parsed)),
            None => (parsed, parsed),
        });
    }
    range
}

fn validated(end: usize, validated: bool) -> ParsedValue {
    ParsedValue { end, validated }
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
    fn block_state_validates_exact_properties_and_snbt() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        let valid = parse_block(
            &units("minecraft:oak_log[axis=x]{Custom:1b}"),
            0,
            false,
            access,
        )
        .unwrap();
        assert_eq!(valid.end, "minecraft:oak_log[axis=x]{Custom:1b}".len());
        assert!(valid.validated);

        assert!(parse_block(&units("oak_log[axis=diagonal]"), 0, false, access).is_err());
        assert!(parse_block(&units("oak_log[axis=x,axis=y]"), 0, false, access).is_err());
        assert!(parse_block(&units("#logs"), 0, false, access).is_err());
    }

    #[test]
    fn command_block_properties_validate_before_following_syntax() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        for suffix in ["", "=", "=x]{broken", "=x,axis=y]"] {
            let error = parse_block(
                &units(&format!("oak_log[missing{suffix}")),
                0,
                false,
                access,
            )
            .unwrap_err();
            assert_eq!(error.cursor, "oak_log[".len());
            assert!(message(&error).contains("unknown block property"));
        }
        for suffix in ["", " ]{broken", " !other]", ",missing=x]"] {
            let error = parse_block(
                &units(&format!("oak_log[axis=diagonal{suffix}")),
                0,
                false,
                access,
            )
            .unwrap_err();
            assert_eq!(error.cursor, "oak_log[axis=".len());
            assert!(message(&error).contains("invalid block property value"));
        }
        let error = parse_block(&units("oak_log[axis=x,axis="), 0, false, access).unwrap_err();
        assert_eq!(error.cursor, "oak_log[axis=x,".len());
        assert!(message(&error).contains("duplicate"));
        assert!(
            parse_block(&units("oak_log[axis=x,]"), 0, false, access)
                .unwrap()
                .validated
        );
        assert!(parse_block(&units("oak_log[axis=x, ]"), 0, false, access).is_err());
    }

    #[test]
    fn block_state_accepts_the_integer_spellings_of_a_listed_value() {
        // `IntegerProperty.getValue` (`IntegerProperty.java:52-59`) parses the
        // value inside the range the definition lists, so the spellings the
        // state report never carries -- `00`, `+0`, `-0`, a quoted `"00"` -- are
        // the listed value itself, while `007` and an overflowing integer are
        // still outside it. Every expectation is the pinned client-26.1 jar's
        // `BlockStateParser.parseForBlock`.
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        for accepted in [
            "oak_sapling[stage=0]",
            "oak_sapling[stage=00]",
            "oak_sapling[stage=+0]",
            "oak_sapling[stage=-0]",
            "oak_sapling[stage=00000000000000001]",
            "oak_sapling[stage=\"00\"]",
            "wheat[age=00]",
            "lava[level=00]",
            "cocoa[age=00,facing=north]",
            // `Integer.parseInt` reads `Character.digit`, so a quoted
            // Arabic-Indic or fullwidth digit is the value it spells.
            "oak_sapling[stage=\"\u{0660}\"]",
            "oak_sapling[stage=\"\u{0661}\"]",
            "oak_sapling[stage=\"\u{ff10}\"]",
            "wheat[age=\"\u{0660}\"]",
            "lava[level=\"\u{ff11}\"]",
        ] {
            let units = units(accepted);
            let parsed = parse_block(&units, 0, false, access).unwrap();
            assert_eq!(parsed.end, units.len(), "{accepted}");
            assert!(parsed.validated, "{accepted}");
        }
        for rejected in [
            "oak_sapling[stage=7]",
            "oak_sapling[stage=007]",
            "oak_sapling[stage=2147483648]",
            "oak_sapling[stage=0x0]",
            "oak_sapling[stage=0_0]",
            "oak_sapling[stage=+]",
            "oak_sapling[stage=-]",
            "chest[waterlogged=TRUE]",
            // Outside the listed range, or a digit-like character
            // `Character.digit` does not read.
            "oak_sapling[stage=\"\u{0662}\"]",
            "oak_sapling[stage=\"\u{00b2}\"]",
            "oak_sapling[stage=\"\u{2160}\"]",
        ] {
            let error = parse_block(&units(rejected), 0, false, access).unwrap_err();
            assert_eq!(error.cursor, rejected.find('=').unwrap() + 1, "{rejected}");
            assert!(
                message(&error).contains("invalid block property value"),
                "{rejected}: {}",
                message(&error)
            );
        }
    }

    #[test]
    fn block_predicate_resolves_tags_but_keeps_vague_properties() {
        let mut registries = RegistrySet::vanilla_26_1();
        registries.tags.insert(
            "minecraft:block".to_string(),
            crate::RegistryTagState::from_materialized(
                "minecraft:block".to_string(),
                BTreeMap::from([("minecraft:logs".to_string(), vec![0])]),
            ),
        );
        let parsed = parse_block(
            &units("#logs[server_property=server_value]{Custom:1b}"),
            0,
            true,
            Some(registries.frozen_access()),
        )
        .unwrap();
        assert!(parsed.validated);
        assert_eq!(
            parse_block(
                &units("#missing[server_property=server_value]"),
                0,
                true,
                Some(registries.frozen_access()),
            )
            .unwrap_err()
            .cursor,
            0
        );
        let access = Some(registries.frozen_access());
        let error = parse_block(&units("#logs[axis=x"), 0, true, access).unwrap_err();
        assert_eq!(error.cursor, "#logs[axis=".len());
        let error = parse_block(&units("#logs[axis   ]"), 0, true, access).unwrap_err();
        assert_eq!(error.cursor, "#logs[".len());
        assert!(
            parse_block(&units("#logs[=]"), 0, true, access)
                .unwrap()
                .validated
        );
    }
}
