//! Shared command grammar: raw entry points use UTF-16 units; `&str` adapters use bytes.

use serde::{Deserialize, Serialize};

mod blocks;
mod context_free;
mod items;
mod resources;
mod strings;
#[cfg(test)]
mod vanilla_grammar_differential;

pub(in crate::component::command) use context_free::parse_command_bounds_utf16;
pub(in crate::component::command) use strings::byte_cursor;
pub use strings::{
    read_command_string_value, read_command_string_value_utf16, scan_command_quotable_string,
    scan_command_quotable_string_utf16, scan_command_unquoted_string,
    scan_command_unquoted_string_utf16, CommandSyntaxErrorUtf16,
};

pub use blocks::{
    scan_command_block_body, scan_command_block_body_utf16,
    scan_command_block_body_with_validation, scan_command_block_identifier,
    scan_command_block_identifier_utf16, ScannedCommandBlockBody, ScannedCommandBlockBodyUtf16,
    ScannedCommandBlockIdentifier, ScannedCommandBlockIdentifierUtf16,
};
pub use context_free::{
    scan_command_angle_argument, scan_command_angle_argument_utf16,
    scan_command_catalog_argument_utf16, scan_command_color_argument,
    scan_command_context_free_argument, scan_command_context_free_argument_utf16,
    scan_command_coordinates_argument, scan_command_degrees_range_argument_utf16,
    scan_command_entity_anchor_argument, scan_command_entity_argument,
    scan_command_float_range_argument, scan_command_game_mode_argument,
    scan_command_game_profile_argument, scan_command_game_profile_argument_utf16,
    scan_command_heightmap_argument, scan_command_hex_color_argument,
    scan_command_hex_color_argument_utf16, scan_command_int_range_argument,
    scan_command_item_slot_argument, scan_command_nbt_path_argument,
    scan_command_operation_argument, scan_command_operation_argument_utf16,
    scan_command_range_argument_utf16, scan_command_resource_location_argument,
    scan_command_resource_location_argument_utf16, scan_command_score_holder_argument,
    scan_command_scoreboard_slot_argument, scan_command_snbt_argument,
    scan_command_snbt_argument_utf16, scan_command_swizzle_argument,
    scan_command_swizzle_argument_utf16, scan_command_template_mirror_argument,
    scan_command_template_rotation_argument, scan_command_time_argument,
    scan_command_time_argument_utf16, scan_command_uuid_argument, scan_command_uuid_argument_utf16,
};
pub use items::{
    command_item_predicate_component, command_known_item_predicate,
    command_persistent_data_component, scan_command_item_predicate_head,
    scan_command_item_predicate_head_utf16, CommandItemComponentPatchScanner,
    CommandItemComponentPatchScannerUtf16, CommandItemComponentSuggestions,
    CommandItemPredicateSuggestions, CommandItemPredicateTestScanner,
    CommandItemPredicateTestScannerUtf16, ScannedCommandItemComponent,
    ScannedCommandItemComponentUtf16, ScannedCommandItemPredicateHead,
    ScannedCommandItemPredicateHeadUtf16, ScannedCommandItemPredicateTerm,
    ScannedCommandItemPredicateTermUtf16, ScannedCommandItemPredicateTest,
    ScannedCommandItemPredicateTestUtf16,
};
pub use resources::{
    command_resource_selector_matches, command_stat_key_for_resource_location,
    scan_command_particle_options, scan_command_particle_options_utf16,
    scan_command_resource_location, scan_command_resource_location_utf16,
    scan_command_resource_or_id, scan_command_resource_or_id_utf16, scan_command_resource_selector,
    scan_command_resource_selector_utf16, scan_command_text_argument,
    scan_command_text_argument_utf16, ScannedCommandParticleOptions,
    ScannedCommandParticleOptionsUtf16, ScannedCommandResourceOrId,
    ScannedCommandResourceOrIdUtf16,
};

/// The `CommandSyntaxException` families a grammar scan can raise, named after
/// the `BuiltInExceptionProvider` member each one stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandSyntaxErrorKind {
    UnknownCommand,
    UnknownArgument,
    IncorrectLiteral,
    ExpectedSeparator,
    ExpectedBoolean,
    InvalidBoolean,
    ExpectedNumber,
    InvalidNumber,
    NumberTooLow,
    NumberTooHigh,
    ExpectedQuoteEnd,
    InvalidEscape,
    InvalidArgument,
}

/// One raised `CommandSyntaxException`: which family it belongs to, the
/// optional byte cursor it was raised at, and a human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSyntaxError {
    pub kind: CommandSyntaxErrorKind,
    pub cursor: Option<usize>,
    pub message: String,
}

impl CommandSyntaxError {
    pub fn without_context(kind: CommandSyntaxErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            cursor: None,
            message: message.into(),
        }
    }
}

pub fn command_syntax_error(
    kind: CommandSyntaxErrorKind,
    cursor: usize,
    message: impl Into<String>,
) -> CommandSyntaxError {
    CommandSyntaxError {
        kind,
        cursor: Some(cursor),
        message: message.into(),
    }
}

fn invalid_argument(cursor: usize, message: impl Into<String>) -> CommandSyntaxError {
    command_syntax_error(CommandSyntaxErrorKind::InvalidArgument, cursor, message)
}

/// The characters Brigadier's numeric readers accept before they hand the run
/// to `Integer.parseInt`/`Double.parseDouble`, and the run itself.
pub fn scan_command_number_lexeme(
    command: &str,
    start: usize,
) -> Result<(usize, &str), CommandSyntaxError> {
    if !command.is_char_boundary(start) {
        return Err(command_syntax_error(
            CommandSyntaxErrorKind::ExpectedNumber,
            start,
            "expected number",
        ));
    }
    strings::scan_suffix_via_utf16(command, start, |input| {
        scan_command_number_lexeme_utf16(input, 0).map(|(end, _)| end)
    })
    .map(|end| (end, &command[start..end]))
}

/// The consumed position and borrowed numeric lexeme use UTF-16 units.
pub fn scan_command_number_lexeme_utf16(
    command: &[u16],
    start: usize,
) -> Result<(usize, &[u16]), CommandSyntaxErrorUtf16> {
    let mut end = start;
    while command
        .get(end)
        .is_some_and(|unit| matches!(*unit, 48..=57 | 46 | 45))
    {
        end += 1;
    }
    if end == start {
        return Err(CommandSyntaxErrorUtf16 {
            kind: CommandSyntaxErrorKind::ExpectedNumber,
            cursor: start,
            message: "expected number".into(),
        });
    }
    Ok((end, &command[start..end]))
}

pub fn scan_command_token(command: &str, start: usize) -> usize {
    let suffix = &command[start..];
    let input: Vec<_> = suffix.encode_utf16().collect();
    start + strings::byte_cursor(suffix, scan_command_token_utf16(&input, 0))
}

/// Token arguments stop at the literal command separator, not Java whitespace.
pub fn scan_command_token_utf16(command: &[u16], start: usize) -> usize {
    command[start..]
        .iter()
        .position(|unit| *unit == 32)
        .map_or(command.len(), |offset| start + offset)
}

#[cfg(test)]
#[test]
fn raw_command_numeric_and_token_scans_keep_unit_boundaries() {
    for unit in 0..=u16::MAX {
        let input = [0xd800, unit, 32];
        let numeric = scan_command_number_lexeme_utf16(&input, 1);
        if matches!(unit, 48..=57 | 46 | 45) {
            assert_eq!(numeric.unwrap(), (2, &input[1..2]));
        } else {
            let error = numeric.unwrap_err();
            assert_eq!(error.kind, CommandSyntaxErrorKind::ExpectedNumber);
            assert_eq!(error.cursor, 1);
        }
        assert_eq!(
            scan_command_token_utf16(&input, 0),
            if unit == 32 { 1 } else { 2 }
        );
    }
    let raw = [45, 49, 46, 50, 0xd800, 32];
    assert_eq!(
        scan_command_number_lexeme_utf16(&raw, 0).unwrap(),
        (4, &raw[..4])
    );
    assert_eq!(
        scan_command_number_lexeme("😀 -1.2x", 5).unwrap(),
        (9, "-1.2")
    );
    assert_eq!(
        scan_command_number_lexeme("😀", 1).unwrap_err().cursor,
        Some(1)
    );
    assert_eq!(
        scan_command_token("😀a\t\u{2000}b tail", 0),
        "😀a\t\u{2000}b".len()
    );
    assert!(scan_command_number_lexeme_utf16(&[], 0).is_err());
    assert_eq!(scan_command_token_utf16(&[], 0), 0);
}

pub fn scan_command_whitespace(command: &str, start: usize) -> usize {
    let units: Vec<_> = command.encode_utf16().collect();
    let start = command[..start].encode_utf16().count();
    strings::byte_cursor(command, scan_command_whitespace_utf16(&units, start))
}

pub fn scan_command_whitespace_utf16(command: &[u16], start: usize) -> usize {
    let mut cursor = start;
    while command
        .get(cursor)
        .is_some_and(|unit| crate::java_string::java_character_is_whitespace(*unit))
    {
        cursor += 1;
    }
    cursor
}

#[cfg(test)]
#[test]
fn command_java_whitespace_reaches_block_properties_and_item_components() {
    for whitespace in [
        '\t', '\u{001c}', '\u{1680}', '\u{2000}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{205f}',
        '\u{3000}',
    ] {
        let block = format!("[{whitespace}axis{whitespace}={whitespace}x{whitespace}] tail");
        let parsed = scan_command_block_body(&block, 0).unwrap();
        assert_eq!(parsed.properties.get("axis").map(String::as_str), Some("x"));
        assert_eq!(&block[parsed.end..], " tail");
        let item = format!("[{whitespace}damage{whitespace}={whitespace}1{whitespace}, {whitespace}!custom_name{whitespace}] tail");
        let mut scanner = CommandItemComponentPatchScanner::new(&item, 0);
        let assigned = scanner.next_component().unwrap().unwrap();
        assert_eq!(assigned.value, Some(crate::NbtSummaryValue::Int(1)));
        assert!(scanner.next_component().unwrap().unwrap().removed);
        assert!(scanner.next_component().unwrap().is_none());
        assert_eq!(&item[scanner.end()..], " tail");
        let empty = format!("[{whitespace}]");
        assert!(CommandItemComponentPatchScanner::new(&empty, 0)
            .next_component()
            .is_err());
        assert!(scan_command_block_body(&empty, 0)
            .unwrap()
            .properties
            .is_empty());
    }
    for unit in [0x0085, 0x00a0, 0x2007, 0x202f, 0xd800, 0xdc00] {
        assert_eq!(scan_command_whitespace_utf16(&[32, unit, 32], 0), 1);
    }
    for ch in ['\u{0085}', '\u{00a0}', '\u{2007}', '\u{202f}'] {
        assert!(scan_command_block_body(&format!("[{ch}axis=x]"), 0).is_err());
        let item = format!("[{ch}damage=1]");
        assert!(CommandItemComponentPatchScanner::new(&item, 0)
            .next_component()
            .is_err());
    }
    assert!(CommandItemComponentPatchScanner::new("[]", 0)
        .next_component()
        .unwrap()
        .is_none());
}

#[cfg(test)]
mod tests;
