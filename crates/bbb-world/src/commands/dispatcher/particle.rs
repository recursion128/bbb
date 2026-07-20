use bbb_protocol::{
    packets::{
        world_effects::{vanilla_particle_command_options, VanillaParticleCommandOptions},
        VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID,
    },
    parse_command_snbt_value_prefix,
};

use super::{
    registry_aware::parse_identifier, syntax_error, CommandSyntaxErrorKind,
    CommandSyntaxErrorState, ParsedValue,
};
use crate::{
    client::validate_command_item_stack_template, registries::validate_command_particle_fields,
    FrozenRegistryAccess, FrozenRegistryMembership,
};

pub(super) fn parse(
    type_id: i32,
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, CommandSyntaxErrorState>> {
    if type_id != VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID {
        return None;
    }
    Some(parse_particle(command, start, registry_access))
}

fn parse_particle(
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, CommandSyntaxErrorState> {
    let (mut cursor, particle_id) = parse_identifier(command, start)?;
    let membership = registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
        access.resource_membership("minecraft:particle_type", &particle_id)
    });
    if membership == FrozenRegistryMembership::Absent {
        return Err(invalid(start, format!("unknown particle {particle_id}")));
    }
    let Some(options) = vanilla_particle_command_options(&particle_id) else {
        return Ok(ParsedValue {
            end: cursor,
            validated: false,
        });
    };
    let has_map = command.as_bytes().get(cursor) == Some(&b'{');
    let mut option_fields = None;
    if has_map {
        let (consumed, value) = parse_command_snbt_value_prefix(&command[cursor..])
            .filter(|(_, value)| matches!(value, bbb_protocol::NbtSummaryValue::Compound(_)))
            .ok_or_else(|| invalid(cursor, "invalid particle options"))?;
        cursor += consumed;
        let bbb_protocol::NbtSummaryValue::Compound(fields) = value else {
            unreachable!("compound filter above")
        };
        option_fields = Some(fields);
    }
    if options == VanillaParticleCommandOptions::RequiredMap && !has_map {
        return Err(invalid(cursor, "particle options are required"));
    }
    let empty_fields = Vec::new();
    let fields = option_fields.as_deref().unwrap_or(&empty_fields);
    let options_valid = if particle_id == "minecraft:item" {
        let item = fields
            .iter()
            .find(|entry| entry.name == "item")
            .ok_or_else(|| invalid(cursor, "particle item is required"))?;
        let Some(registry_access) = registry_access else {
            return Ok(ParsedValue {
                end: cursor,
                validated: false,
            });
        };
        validate_command_item_stack_template(&item.value, registry_access)
            .map_err(|error| invalid(cursor, error))?;
        true
    } else {
        validate_command_particle_fields(&particle_id, fields)
    };
    if !options_valid {
        return Err(invalid(cursor, "invalid particle options"));
    }
    Ok(ParsedValue {
        end: cursor,
        validated: membership == FrozenRegistryMembership::Present,
    })
}

fn invalid(cursor: usize, message: impl Into<String>) -> CommandSyntaxErrorState {
    syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistrySet;

    #[test]
    fn particle_identity_and_empty_option_contract_are_exact() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        assert!(parse_particle("minecraft:heart", 0, access).is_ok_and(|value| value.validated));
        assert!(parse_particle("minecraft:dust", 0, access).is_err());
        assert!(
            parse_particle("minecraft:dust{color:16711680,scale:1.0f}", 0, access)
                .is_ok_and(|value| value.validated)
        );
        assert!(parse_particle("minecraft:dust{color:16711680,scale:8.0f}", 0, access).is_err());
        assert!(parse_particle("minecraft:effect", 0, access).is_ok_and(|value| value.validated));
        assert!(parse_particle(
            "minecraft:item{item:{id:'minecraft:stick',count:2}}",
            0,
            access,
        )
        .is_ok_and(|value| value.validated));
        assert!(parse_particle("minecraft:missing", 0, access).is_err());
    }
}
