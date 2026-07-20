use bbb_protocol::{
    packets::{VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID, VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID},
    parse_command_snbt_value_prefix,
};

use super::{syntax_error, CommandSyntaxErrorKind, CommandSyntaxErrorState, ParsedValue};
use crate::{
    client::{validate_command_rich_text, validate_command_style},
    FrozenRegistryAccess,
};

pub(super) fn parse(
    type_id: i32,
    command: &str,
    start: usize,
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<Result<ParsedValue, CommandSyntaxErrorState>> {
    let validator = match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID => validate_command_rich_text,
        VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID => validate_command_style,
        _ => return None,
    };
    let result = parse_command_snbt_value_prefix(&command[start..])
        .ok_or_else(|| invalid(start, "invalid text SNBT"))
        .and_then(|(consumed, value)| {
            let Some(registry_access) = registry_access else {
                return Ok(ParsedValue {
                    end: start + consumed,
                    validated: false,
                });
            };
            validator(&value, registry_access)
                .map_err(|error| invalid(start, error))
                .map(|()| ParsedValue {
                    end: start + consumed,
                    validated: true,
                })
        });
    Some(result)
}

fn invalid(cursor: usize, message: impl Into<String>) -> CommandSyntaxErrorState {
    syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistrySet;

    #[test]
    fn rich_text_and_style_reuse_the_component_codec_owner() {
        let registries = RegistrySet::vanilla_26_1();
        let access = Some(registries.frozen_access());
        let rich_text = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID,
            "{text:'hello',bold:true}",
            0,
            access,
        )
        .expect("text parser")
        .expect("valid component");
        assert!(rich_text.validated);

        let style = parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
            "{color:'red',italic:true}",
            0,
            access,
        )
        .expect("style parser")
        .expect("valid style");
        assert!(style.validated);
        assert!(parse(
            VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
            "{bold:'yes'}",
            0,
            access,
        )
        .expect("style parser")
        .is_err());
    }
}
