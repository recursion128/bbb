//! The grammar of `minecraft:block_state` and `minecraft:block_predicate`.
//!
//! The two arguments share one shape -- an optionally `#`-tagged block name, an
//! optional bracketed property list and an optional block-entity compound --
//! and differ only in what a caller is allowed to resolve afterwards. The scan
//! is split in two so that a caller can reject an unknown block name at the
//! cursor vanilla rejects it at, before the property list has been read.

use std::collections::BTreeMap;

use super::{
    read_command_string_value_utf16, scan_command_resource_location_utf16,
    scan_command_whitespace_utf16, strings, CommandSyntaxError, CommandSyntaxErrorKind,
    CommandSyntaxErrorUtf16,
};
use crate::JavaString;

/// The block name a block-state or block-predicate argument opens with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedCommandBlockIdentifier {
    /// Whether the name was written as a `#tag` rather than as a block.
    pub tagged: bool,
    /// Where the name itself starts -- past the `#` when there was one.
    pub identifier_start: usize,
    pub resource_id: String,
    pub end: usize,
}

impl ScannedCommandBlockIdentifier {
    fn map_offsets(self, to_byte: &dyn Fn(usize) -> usize) -> Self {
        Self {
            tagged: self.tagged,
            identifier_start: to_byte(self.identifier_start),
            resource_id: self.resource_id,
            end: to_byte(self.end),
        }
    }
}

pub type ScannedCommandBlockIdentifierUtf16 = ScannedCommandBlockIdentifier;

/// The property list and block-entity compound that may follow the name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScannedCommandBlockBody {
    /// The `[key=value]` pairs, empty when the argument had no property list.
    pub properties: BTreeMap<String, String>,
    pub end: usize,
}

pub fn scan_command_block_identifier_utf16(
    command: &[u16],
    start: usize,
    allow_tag: bool,
) -> Result<ScannedCommandBlockIdentifierUtf16, CommandSyntaxErrorUtf16> {
    let tagged = command.get(start) == Some(&35);
    if tagged && !allow_tag {
        return Err(block_error(
            start,
            "block-state argument does not allow tags",
        ));
    }
    let identifier_start = start + usize::from(tagged);
    let (end, resource_id) = scan_command_resource_location_utf16(command, identifier_start)?;
    Ok(ScannedCommandBlockIdentifier {
        tagged,
        identifier_start,
        resource_id,
        end,
    })
}

pub fn scan_command_block_identifier(
    command: &str,
    start: usize,
    allow_tag: bool,
) -> Result<ScannedCommandBlockIdentifier, CommandSyntaxError> {
    strings::scan_from_start_via_utf16(
        command,
        start,
        |units, unit_start| scan_command_block_identifier_utf16(units, unit_start, allow_tag),
        ScannedCommandBlockIdentifier::map_offsets,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScannedCommandBlockBodyUtf16 {
    pub properties: BTreeMap<JavaString, JavaString>,
    pub end: usize,
}

pub fn scan_command_block_body(
    command: &str,
    start: usize,
) -> Result<ScannedCommandBlockBody, CommandSyntaxError> {
    scan_command_block_body_with_validation(command, start, false, |_, _| Ok(()))
}

/// The callback checks a name before `=`, then its value before the next delimiter.
pub fn scan_command_block_body_with_validation(
    command: &str,
    start: usize,
    vague: bool,
    mut validate: impl FnMut(&str, Option<&str>) -> Result<(), String>,
) -> Result<ScannedCommandBlockBody, CommandSyntaxError> {
    strings::scan_from_start_via_utf16(
        command,
        start,
        |units, unit_start| {
            scan_command_block_body_utf16(units, unit_start, vague, |key, value| {
                let key = String::from_utf16(key.units()).expect("UTF-8 property name");
                let value = value
                    .map(|value| String::from_utf16(value.units()).expect("UTF-8 property value"));
                validate(&key, value.as_deref()).map_err(JavaString::from)
            })
        },
        |body, to_byte| ScannedCommandBlockBody {
            properties: body
                .properties
                .into_iter()
                .map(|(key, value)| {
                    (
                        String::from_utf16(key.units()).expect("UTF-8 property name"),
                        String::from_utf16(value.units()).expect("UTF-8 property value"),
                    )
                })
                .collect(),
            end: to_byte(body.end),
        },
    )
}

/// All positions and property strings retain the original UTF-16 units.
pub fn scan_command_block_body_utf16(
    command: &[u16],
    start: usize,
    vague: bool,
    mut validate: impl FnMut(&JavaString, Option<&JavaString>) -> Result<(), JavaString>,
) -> Result<ScannedCommandBlockBodyUtf16, CommandSyntaxErrorUtf16> {
    let mut cursor = start;
    let mut properties = BTreeMap::new();
    if command.get(cursor) == Some(&91) {
        cursor = scan_command_whitespace_utf16(command, cursor + 1);
        let mut last_value_start = None;
        while cursor < command.len() && command.get(cursor) != Some(&93) {
            cursor = scan_command_whitespace_utf16(command, cursor);
            let key_start = cursor;
            let (end, key) = read_command_string_value_utf16(command, cursor)?;
            cursor = end;
            validate(&key, None).map_err(|message| block_error(key_start, message))?;
            if properties.contains_key(&key) {
                let message: Vec<_> = "duplicate block property "
                    .encode_utf16()
                    .chain(key.units().iter().copied())
                    .collect();
                return Err(block_error(key_start, JavaString::from(message)));
            }
            cursor = scan_command_whitespace_utf16(command, cursor);
            if command.get(cursor) != Some(&61) {
                return Err(block_error(
                    if vague { key_start } else { cursor },
                    "expected block property value",
                ));
            }
            cursor = scan_command_whitespace_utf16(command, cursor + 1);
            let value_start = cursor;
            let (end, value) = read_command_string_value_utf16(command, cursor)?;
            cursor = end;
            validate(&key, Some(&value)).map_err(|message| block_error(value_start, message))?;
            properties.insert(key, value);
            cursor = scan_command_whitespace_utf16(command, cursor);
            last_value_start = Some(value_start);
            if cursor < command.len() {
                last_value_start = None;
                match command[cursor] {
                    44 => cursor += 1,
                    93 => break,
                    _ => {
                        return Err(block_error(
                            cursor,
                            "expected block property separator or end",
                        ))
                    }
                }
            }
        }
        if command.get(cursor) != Some(&93) {
            return Err(block_error(
                if vague {
                    last_value_start.unwrap_or(cursor)
                } else {
                    cursor
                },
                "unclosed block properties",
            ));
        }
        cursor += 1;
    }
    if command.get(cursor) == Some(&123) {
        let (consumed, _) = crate::parse_snbt_compound_prefix_utf16(&command[cursor..])
            .map_err(|_| block_error(cursor, "invalid block entity SNBT"))?;
        cursor += consumed;
    }
    Ok(ScannedCommandBlockBodyUtf16 {
        properties,
        end: cursor,
    })
}

fn block_error(cursor: usize, message: impl Into<JavaString>) -> CommandSyntaxErrorUtf16 {
    CommandSyntaxErrorUtf16 {
        kind: CommandSyntaxErrorKind::InvalidArgument,
        cursor,
        message: message.into(),
    }
}

#[cfg(test)]
#[test]
fn raw_command_block_identifier_preserves_cursor_and_unread_units() {
    let input: Vec<_> = [0xd800]
        .into_iter()
        .chain("#oak_log".encode_utf16())
        .chain([0xdc00])
        .collect();
    let head = scan_command_block_identifier_utf16(&input, 1, true).unwrap();
    assert!(head.tagged);
    assert_eq!(head.identifier_start, 2);
    assert_eq!(head.resource_id, "minecraft:oak_log");
    assert_eq!(head.end, input.len() - 1);
    assert_eq!(
        scan_command_block_identifier_utf16(&input, 1, false)
            .unwrap_err()
            .cursor,
        1
    );
    let input = "😀#oak_log";
    let head = scan_command_block_identifier(input, "😀".len(), true).unwrap();
    assert_eq!(head.identifier_start, "😀#".len());
    assert_eq!(head.end, input.len());
}
