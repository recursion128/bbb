use bbb_protocol::{
    packets::VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID, scan_command_particle_options_utf16,
    scan_command_resource_location_utf16,
};

use super::{syntax_error, CommandSyntaxErrorKind, ParsedValue, RawCommandSyntaxErrorState};
use crate::{
    client::validate_command_item_stack_template, registries::validate_command_particle_fields,
    FrozenRegistryAccess, FrozenRegistryMembership,
};

pub(super) fn parse(
    type_id: i32,
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, RawCommandSyntaxErrorState>> {
    if type_id != VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID {
        return None;
    }
    Some(parse_particle(command, start, registry_access))
}

fn parse_particle(
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Result<ParsedValue, RawCommandSyntaxErrorState> {
    let (cursor, particle_id) = scan_command_resource_location_utf16(command, start)?;
    let membership = registry_access.map_or(FrozenRegistryMembership::Unavailable, |access| {
        access.resource_membership("minecraft:particle_type", &particle_id)
    });
    if membership == FrozenRegistryMembership::Absent {
        return Err(invalid(cursor, format!("unknown particle {particle_id}")));
    }
    let content_registry = registry_access.map_or(
        bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
        |registries| registries.content_registry(),
    );
    let Some(_) = bbb_protocol::packets::particle_command_options_with_registry(
        &particle_id,
        content_registry,
    ) else {
        return Ok(ParsedValue {
            end: cursor,
            validated: false,
        });
    };
    // ParticleArgument supplies an empty map to the codec when no compound follows.
    let scanned = scan_command_particle_options_utf16(command, cursor)?;
    let cursor = scanned.end;
    let empty_fields = Vec::new();
    let fields = scanned.fields.as_deref().unwrap_or(&empty_fields);
    let options_valid = if particle_id == "minecraft:item" {
        let item = fields
            .iter()
            .find(|entry| entry.name == "item")
            .ok_or_else(|| codec_error(start, "particle item is required"))?;
        let Some(registry_access) = registry_access else {
            return Ok(ParsedValue {
                end: cursor,
                validated: false,
            });
        };
        validate_command_item_stack_template(&item.value, registry_access)
            .map_err(|error| codec_error(start, error))?;
        true
    } else if !content_registry.is_vanilla_only()
        && content_registry
            .particle_by_resource_id(&particle_id)
            .is_some()
    {
        fields.is_empty()
    } else {
        validate_command_particle_fields(&particle_id, fields)
    };
    if !options_valid {
        return Err(codec_error(start, "invalid particle options"));
    }
    Ok(ParsedValue {
        end: cursor,
        validated: membership == FrozenRegistryMembership::Present,
    })
}

fn codec_error(start: usize, message: impl Into<String>) -> RawCommandSyntaxErrorState {
    RawCommandSyntaxErrorState::without_context(
        CommandSyntaxErrorKind::InvalidArgument,
        start,
        message.into(),
    )
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
    use crate::RegistrySet;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn command_particle_raw_item_options_reach_codec_and_unknown_name_ends_at_cursor() {
        use bbb_protocol::scan_command_particle_options_utf16;
        let registries = RegistrySet::vanilla_26_1();
        let input: Vec<_> = "{item:{id:'minecraft:stick',components:{custom_name:'"
            .encode_utf16()
            .chain([0xd800])
            .chain("'}}} tail".encode_utf16())
            .collect();
        let parsed = scan_command_particle_options_utf16(&input, 0).unwrap();
        assert_eq!(parsed.end, input.len() - 5);
        validate_command_item_stack_template(
            &parsed.fields.unwrap()[0].value,
            registries.frozen_access(),
        )
        .unwrap();
        let input = units("😀minecraft:missing{}");
        let error = parse_particle(&input, units("😀").len(), Some(registries.frozen_access()))
            .unwrap_err();
        assert_eq!(error.cursor, units("😀minecraft:missing").len());
    }

    #[test]
    fn particle_identity_and_empty_option_contract_are_exact() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        assert!(
            parse_particle(&units("minecraft:heart"), 0, access).is_ok_and(|value| value.validated)
        );
        assert!(parse_particle(&units("minecraft:dust"), 0, access).is_err());
        assert!(parse_particle(
            &units("minecraft:dust{color:16711680,scale:1.0f}"),
            0,
            access
        )
        .is_ok_and(|value| value.validated));
        assert!(parse_particle(
            &units("minecraft:dust{color:16711680,scale:8.0f}"),
            0,
            access
        )
        .is_err());
        assert!(parse_particle(&units("minecraft:effect"), 0, access)
            .is_ok_and(|value| value.validated));
        assert!(parse_particle(
            &units("minecraft:item{item:{id:'minecraft:stick',count:2}}"),
            0,
            access,
        )
        .is_ok_and(|value| value.validated));
        assert!(parse_particle(&units("minecraft:missing"), 0, access).is_err());
    }
}
