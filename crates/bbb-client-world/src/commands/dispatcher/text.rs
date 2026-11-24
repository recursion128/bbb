use bbb_protocol::{
    packets::{VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID, VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID},
    scan_command_text_argument_utf16,
};

use super::{syntax_error, CommandSyntaxErrorKind, ParsedValue, RawCommandSyntaxErrorState};
use crate::{
    client::{validate_command_rich_text, validate_command_style},
    FrozenRegistryAccess,
};

pub(super) fn parse(
    type_id: i32,
    command: &[u16],
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, RawCommandSyntaxErrorState>> {
    let validator = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID => validate_command_rich_text,
        VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID => validate_command_style,
        _ => return None,
    };
    let result = scan_command_text_argument_utf16(command, start)
        .map_err(RawCommandSyntaxErrorState::from)
        .and_then(|(end, value)| {
            let Some(registry_access) = registry_access else {
                return Ok(ParsedValue {
                    end,
                    validated: false,
                });
            };
            validator(&value, registry_access)
                .map_err(|error| invalid(start, error))
                .map(|()| ParsedValue {
                    end,
                    validated: true,
                })
        });
    Some(result)
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
    fn command_raw_text_and_style_reach_codec_and_errors_rewind_to_start() {
        use bbb_protocol::scan_command_text_argument_utf16;
        let registries = RegistrySet::vanilla_26_1();
        for (prefix, validate) in [
            (
                "{text:'",
                validate_command_rich_text
                    as fn(
                        &bbb_protocol::NbtSummaryValue,
                        FrozenRegistryAccess<'_>,
                    ) -> Result<(), String>,
            ),
            (
                "{insertion:'",
                validate_command_style
                    as fn(
                        &bbb_protocol::NbtSummaryValue,
                        FrozenRegistryAccess<'_>,
                    ) -> Result<(), String>,
            ),
        ] {
            let input: Vec<_> = [0xdc00]
                .into_iter()
                .chain(prefix.encode_utf16())
                .chain([0xd800])
                .chain("'} tail".encode_utf16())
                .collect();
            let (end, value) = scan_command_text_argument_utf16(&input, 1).unwrap();
            assert_eq!(end, input.len() - 5);
            validate(&value, registries.frozen_access()).unwrap();
        }
        for type_id in [
            VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID,
            VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
        ] {
            let error = parse(
                type_id,
                &units("😀  {bold:'yes'} tail"),
                units("😀").len(),
                Some(registries.frozen_access()),
            )
            .unwrap()
            .unwrap_err();
            assert_eq!(error.cursor, units("😀").len());
        }
    }

    #[test]
    fn rich_text_and_style_reuse_the_component_codec_owner() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        let rich_text = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID,
            &units("{text:'hello',bold:true}"),
            0,
            access,
        )
        .expect("text parser")
        .expect("valid component");
        assert!(rich_text.validated);

        let style = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
            &units("{color:'red',italic:true}"),
            0,
            access,
        )
        .expect("style parser")
        .expect("valid style");
        assert!(style.validated);
        assert!(parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
            &units("{bold:'yes'}"),
            0,
            access,
        )
        .expect("style parser")
        .is_err());
    }
}
