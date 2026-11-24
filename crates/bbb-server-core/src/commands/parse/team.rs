//! The team-name and colour argument readers.
//!
//! Neither resolves a team: `minecraft:team` reads a bare word
//! (`TeamArgument.java:40-42`) and leaves the lookup to `getTeam` (`:29-38`),
//! which runs when the command does. `minecraft:color` is the one of the pair
//! that does resolve, because `ChatFormatting.getByName` is what decides
//! whether the word is a colour at all.

use bbb_protocol::chat_formattings::{ChatFormatting, VANILLA_CHAT_FORMATTINGS};
use bbb_protocol::{
    component_literal_nbt, scan_command_hex_color_argument_utf16,
    scan_command_unquoted_string_utf16, JavaString,
};

use super::super::translatable;
use super::{word_end, ArgumentValue, ParseFailure};

/// `TeamArgument.parse` (`TeamArgument.java:40-42`): `readUnquotedString` and
/// nothing else.
pub(in crate::commands) fn read_team(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    Ok((
        end,
        ArgumentValue::Team(
            JavaString::with_utf8_units(&command[cursor..end], str::to_owned)
                .expect("an unquoted word is ASCII"),
        ),
    ))
}

/// `ColorArgument.parse` (`ColorArgument.java:35-43`): `readUnquotedString`
/// through `ChatFormatting.getByName`, refused unless the answer is a
/// non-format one. `createWithContext` reads the cursor after the word, so the
/// refusal points at the end of what was typed rather than at its start.
pub(in crate::commands) fn read_color(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = word_end(command, cursor);
    let name = &command[cursor..end];
    match JavaString::with_utf8_units(name, chat_formatting_by_name)
        .flatten()
        .filter(|formatting| !formatting.is_format())
    {
        Some(formatting) => Ok((end, ArgumentValue::Color(formatting))),
        None => Err(ParseFailure {
            message: translatable("argument.color.invalid", vec![component_literal_nbt(name)]),
            cursor: Some(end),
        }),
    }
}

/// `HexColorArgument.parse` (`HexColorArgument.java:35-50`).
pub(in crate::commands) fn read_hex_color(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let end = scan_command_unquoted_string_utf16(command, cursor);
    // The context-free scanners answer an optional cursor; this argument always
    // has one, and the scanner reports it at `end`.
    scan_command_hex_color_argument_utf16(command, cursor).map_err(|failure| ParseFailure {
        message: translatable(
            "argument.hexcolor.invalid",
            vec![component_literal_nbt(&command[cursor..end])],
        ),
        cursor: Some(failure.cursor),
    })?;

    let raw = &command[cursor..end];
    let rgb = if raw.len() == 3 {
        raw.iter().copied().fold(0_u32, |color, digit| {
            let digit = char::from_u32(u32::from(digit))
                .expect("a hex digit is ASCII")
                .to_digit(16)
                .expect("the scanner accepted a hex digit");
            (color << 8) | (digit * 17)
        })
    } else {
        JavaString::with_utf8_units(raw, |raw| u32::from_str_radix(raw, 16))
            .expect("a hex digit is ASCII")
            .expect("the scanner accepted six hex digits")
    };
    Ok((end, ArgumentValue::HexColor((0xff00_0000 | rgb) as i32)))
}

/// `ChatFormatting.getByName` (`ChatFormatting.java:111-113`) over `cleanName`
/// (`:56-58`), which lowercases and then drops every character that is not a
/// letter -- so `dark_blue`, `DarkBlue` and `dark-blue` are one name.
fn chat_formatting_by_name(name: &str) -> Option<ChatFormatting> {
    let cleaned = clean_name(name);
    VANILLA_CHAT_FORMATTINGS
        .iter()
        .find(|entry| clean_name(entry.serialized_name) == cleaned)
        .map(|entry| entry.formatting)
}

fn clean_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(char::is_ascii_lowercase)
        .collect()
}
